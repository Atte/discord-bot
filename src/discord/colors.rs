use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::eyre::{eyre, OptionExt, Result};
use mongodb::{bson::doc, options::FindOneOptions, Database};
use serde::{Deserialize, Serialize};
use serenity::all::{Cache, CreateMessage, GuildId, Http, RoleId, UserId};
use std::collections::HashMap;

use crate::config::{ColorsConfig, DiscordConfig};

const COLLECTION_NAME: &str = "colors";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stats {
    time: SystemTime,
    guild_id: GuildId,
    names: HashMap<RoleId, String>,
    colors: HashMap<RoleId, (u8, u8, u8)>,
    users: HashMap<RoleId, Vec<UserId>>,
}

impl Stats {
    pub fn new(guild_id: GuildId) -> Self {
        Self {
            time: SystemTime::now(),
            guild_id,
            names: Default::default(),
            colors: Default::default(),
            users: Default::default(),
        }
    }
}

async fn collect_stats(cache: &Cache, config: &ColorsConfig, guild_id: GuildId) -> Result<Stats> {
    let guild = cache
        .guild(guild_id)
        .ok_or_eyre("Guild not found in cache")?;

    let highest_position = guild
        .roles
        .values()
        .filter(|role| config.start_roles.contains(&role.id))
        .map(|role| role.position)
        .max()
        .ok_or_else(|| eyre!("Colors start marker not found!"))?;
    let lowest_position = guild
        .roles
        .values()
        .filter(|role| config.end_roles.contains(&role.id))
        .map(|role| role.position)
        .min()
        .ok_or_else(|| eyre!("Colors end marker not found!"))?;

    let ranks = guild.roles.values().filter(|role| {
        !config.start_roles.contains(&role.id)
            && !config.end_roles.contains(&role.id)
            && role.position > lowest_position
            && role.position < highest_position
            && !role.name.starts_with('@')
    });

    let mut stats = Stats::new(guild_id);

    for rank in ranks {
        stats.names.insert(rank.id, rank.name.clone());
        stats.colors.insert(rank.id, rank.colour.tuple());
        stats.users.insert(
            rank.id,
            guild
                .members
                .values()
                .filter(|member| member.roles.iter().any(|id| id == &rank.id))
                .map(|member| member.user.id)
                .collect(),
        );
    }

    Ok(stats)
}

async fn remove_ranks(http: &Http, guild_id: GuildId, stats: &Stats) -> Result<()> {
    for (rank_id, users) in &stats.users {
        for user_id in users {
            if let Ok(member) = guild_id.member(http, user_id).await {
                if let Err(err) = member.remove_role(http, rank_id).await {
                    log::error!("Failed to remove rank {rank_id} from user {user_id} in guild {guild_id}: {err}");
                }
            }
        }
    }

    Ok(())
}

async fn latest_from_database(db: &Database, guild_id: GuildId) -> Result<Option<SystemTime>> {
    let collection = db.collection::<Stats>(COLLECTION_NAME);
    if let Some(latest) = collection
        .find_one(
            doc! {
                "guild_id": guild_id.to_string(),
            },
            FindOneOptions::builder().sort(doc! {"time": -1}).build(),
        )
        .await?
    {
        Ok(Some(latest.time))
    } else {
        Ok(None)
    }
}

async fn save_to_database(db: &Database, stats: &Stats) -> Result<()> {
    let collection = db.collection::<Stats>(COLLECTION_NAME);
    collection.insert_one(stats, None).await?;
    Ok(())
}

pub fn spawn(
    http: Arc<Http>,
    cache: Arc<Cache>,
    config: ColorsConfig,
    db: Database,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;

            'guilds: for guild_id in &config.guilds {
                match latest_from_database(&db, *guild_id).await {
                    Err(err) => {
                        log::error!("Failed to get latest from database for {guild_id}: {err}");
                        continue;
                    }
                    Ok(None) => {}
                    Ok(Some(latest)) => {
                        if let Ok(duration) = SystemTime::now().duration_since(latest) {
                            if duration < config.rate {
                                continue;
                            }
                        }
                    }
                }

                match collect_stats(&cache, &config, *guild_id).await {
                    Err(err) => {
                        log::error!("Failed to collect stats for {guild_id}: {err}");
                        continue;
                    }
                    Ok(stats) => {
                        if let Err(err) = save_to_database(&db, &stats).await {
                            log::error!("Failed to save stats for {guild_id}: {err}");
                            continue;
                        }

                        if let Err(err) = remove_ranks(&http, *guild_id, &stats).await {
                            log::error!("Failed to remove ranks from {guild_id}: {err}");
                            continue;
                        }

                        for channel_id in &config.channels {
                            if channel_id
                                .to_channel_cached(&cache)
                                .map_or(false, |channel| channel.guild_id == *guild_id)
                            {
                                if let Err(err) = channel_id
                                    .send_message(
                                        &http,
                                        CreateMessage::new().content(
                                            config
                                                .reset_message
                                                .as_ref()
                                                .map_or("Colors have been removed!", |s| {
                                                    s.as_str()
                                                }),
                                        ),
                                    )
                                    .await
                                {
                                    log::error!("Failed to announce rank removal on {guild_id} on {channel_id}: {err}");
                                } else {
                                    continue 'guilds;
                                }
                            }
                        }

                        log::error!("No channel to announce removal of ranks on {guild_id}");
                    }
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    })
}
