use crate::config::Config;
use serenity::{
    model::{
        channel::{GuildChannel, Message},
        guild::{Member, Role},
        id::{ChannelId, GuildId, RoleId, UserId},
    },
    CacheAndHttp,
};
use std::collections::HashMap;

pub async fn guild_member(
    guild_id: GuildId,
    user_id: UserId,
    discord: &CacheAndHttp,
) -> Option<Member> {
    if let Some(guild) = guild_id.to_guild_cached(&discord.cache).await {
        guild.members.get(&user_id).cloned()
    } else {
        guild_id.member(discord.clone(), user_id).await.ok()
    }
}

pub async fn guild_roles(
    guild_id: GuildId,
    discord: &CacheAndHttp,
) -> Option<HashMap<RoleId, Role>> {
    if let Some(guild) = guild_id.to_guild_cached(&discord.cache).await {
        Some(guild.roles)
    } else {
        guild_id.roles(&discord.http).await.ok()
    }
}

pub async fn guild_channels(
    guild_id: GuildId,
    discord: &CacheAndHttp,
) -> Option<HashMap<ChannelId, GuildChannel>> {
    if let Some(guild) = guild_id.to_guild_cached(&discord.cache).await {
        Some(guild.channels)
    } else {
        guild_id.channels(&discord.http).await.ok()
    }
}

pub async fn guild_rules(
    guild_id: GuildId,
    discord: &CacheAndHttp,
    config: &Config,
) -> Option<Message> {
    let channels = guild_channels(guild_id, discord).await?;
    for channel_id in &config.discord.rules_channels {
        if let Some(channel) = channels.get(channel_id) {
            for message in channel
                .id
                .messages(&discord.http, |get| get.limit(10))
                .await
                .ok()?
            {
                if message.is_own(&discord.cache).await {
                    return Some(message);
                }
            }
        }
    }
    None
}

async fn ranks_from_roles(
    guild_id: GuildId,
    roles: impl IntoIterator<Item = Role> + Clone,
    discord: &CacheAndHttp,
) -> Result<impl Iterator<Item = Role>, &'static str> {
    let bot_member = guild_id
        .member(discord.clone(), discord.cache.current_user_id().await)
        .await
        .map_err(|_| "can't fetch bot member")?;

    let cutoff_position = roles
        .clone()
        .into_iter()
        .filter_map(|role| {
            if bot_member.roles.contains(&role.id) {
                Some(role.position)
            } else {
                None
            }
        })
        .min()
        .ok_or("empty roles for bot")?;

    Ok(roles
        .into_iter()
        .filter(move |role| role.position < cutoff_position && !role.name.starts_with('@')))
}

pub async fn ranks_from_guild(
    guild_id: GuildId,
    discord: &CacheAndHttp,
) -> Result<impl Iterator<Item = Role>, &'static str> {
    let roles: Vec<_> = guild_roles(guild_id, discord)
        .await
        .ok_or("can't find roles for guild")?
        .into_values()
        .collect();
    Ok(ranks_from_roles(guild_id, roles, discord).await?)
}
