use super::{get_data, DbKey};
use chrono::{DateTime, Utc};
use color_eyre::eyre::{eyre, Result};
use conv::{UnwrapOrSaturate, ValueFrom};
use lazy_static::lazy_static;
use mongodb::{bson::doc, options::UpdateOptions};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::{
    client::Context,
    model::{
        channel::Message,
        id::{ChannelId, EmojiId, RoleId, UserId},
    },
};

pub const COLLECTION_NAME: &str = "stats";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Stats {
    Member {
        guild_id: String,
        id: String,
        tag: String,
        tags: Vec<String>,
        nick: String,
        nicks: Vec<String>,
        first_message: DateTime<Utc>,
        last_message: DateTime<Utc>,
        emoji_count: usize,
        message_count: usize,
        channel_mention_count: usize,
        role_mention_count: usize,
        user_mention_count: usize,
    },
    Channel {
        guild_id: String,
        id: String,
        name: String,
        names: Vec<String>,
        first_message: DateTime<Utc>,
        last_message: DateTime<Utc>,
        emoji_count: usize,
        message_count: usize,
        channel_mention_count: usize,
        role_mention_count: usize,
        user_mention_count: usize,
    },
    Emoji {
        guild_id: String,
        id: String,
        name: String,
        names: Vec<String>,
        first_message: DateTime<Utc>,
        last_message: DateTime<Utc>,
        use_count: usize,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn update_stats(ctx: &Context, msg: &Message) -> Result<()> {
    lazy_static! {
        static ref USER_MENTION_RE: Regex =
            Regex::new(r"<@!?(?P<id>[0-9]+)>").expect("Invalid regex for USER_MENTION_RE");
        static ref CHANNEL_MENTION_RE: Regex =
            Regex::new(r"<#(?P<id>[0-9]+)>").expect("Invalid regex for CHANNEL_MENTION_RE");
        static ref ROLE_MENTION_RE: Regex =
            Regex::new(r"<@&(?P<id>[0-9]+)>").expect("Invalid regex for ROLE_MENTION_RE");
        static ref EMOJI_RE: Regex =
            Regex::new(r"<a?:(?P<name>[^:]+):(?P<id>[0-9]+)>").expect("Invalid regex for EMOJI_RE");
    }

    let channel = msg
        .channel(&ctx)
        .await?
        .guild()
        .ok_or_else(|| eyre!("Not a guild channel!"))?;

    let nick = msg
        .author_nick(&ctx)
        .await
        .unwrap_or_else(|| msg.author.name.clone());

    let user_mentions: Vec<UserId> = USER_MENTION_RE
        .captures_iter(&msg.content)
        .filter_map(|cap| cap.name("id").and_then(|c| c.as_str().parse().ok()))
        .collect();
    let channel_mentions: Vec<ChannelId> = CHANNEL_MENTION_RE
        .captures_iter(&msg.content)
        .filter_map(|cap| cap.name("id").and_then(|c| c.as_str().parse().ok()))
        .collect();
    let role_mentions: Vec<RoleId> = ROLE_MENTION_RE
        .captures_iter(&msg.content)
        .filter_map(|cap| cap.name("id").and_then(|c| c.as_str().parse().ok()))
        .collect();
    let emojis: Vec<(EmojiId, &str)> = EMOJI_RE
        .captures_iter(&msg.content)
        .filter_map(|cap| {
            cap.name("id")
                .and_then(|c| c.as_str().parse::<u64>().map(EmojiId::new).ok())
                .zip(cap.name("name").map(|c| c.as_str()))
        })
        .collect();

    let now = Utc::now();
    let collection = get_data::<DbKey>(ctx)
        .await?
        .collection::<Stats>(COLLECTION_NAME);

    collection
        .update_one(
            doc! {
                "type": "channel",
                "id": channel.id.to_string(),
                "guild_id": channel.guild_id.to_string(),
            },
            doc! {
                "$set": {
                    "name": &channel.name,
                    "last_message": now,
                },
                "$addToSet": {
                    "names": &channel.name,
                },
                "$setOnInsert": {
                    "first_message": now,
                },
                "$inc": {
                    "message_count": 1,
                    "user_mention_count": i64::value_from(user_mentions.len()).unwrap_or_saturate(),
                    "channel_mention_count": i64::value_from(channel_mentions.len()).unwrap_or_saturate(),
                    "role_mention_count": i64::value_from(role_mentions.len()).unwrap_or_saturate(),
                    "emoji_count": i64::value_from(emojis.len()).unwrap_or_saturate(),
                },
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;

    collection
        .update_one(
            doc! {
                "type": "member",
                "id": msg.author.id.to_string(),
                "guild_id": channel.guild_id.to_string(),
            },
            doc! {
                "$set": {
                    "tag": &msg.author.tag(),
                    "nick": &nick,
                    "last_message": now,
                },
                "$addToSet": {
                    "tags": &msg.author.tag(),
                    "nicks": nick,
                },
                "$setOnInsert": {
                    "first_message": now,
                },
                "$inc": {
                    "message_count": 1,
                    "user_mention_count": i64::value_from(user_mentions.len()).unwrap_or_saturate(),
                    "channel_mention_count": i64::value_from(channel_mentions.len()).unwrap_or_saturate(),
                    "role_mention_count": i64::value_from(role_mentions.len()).unwrap_or_saturate(),
                    "emoji_count": i64::value_from(emojis.len()).unwrap_or_saturate(),
                },
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;

    for (id, name) in emojis {
        collection
            .update_one(
                doc! {
                    "type": "emoji",
                    "id": id.to_string(),
                    "guild_id": channel.guild_id.to_string(),
                },
                doc! {
                    "$set": {
                        "name": name,
                        "last_message": now,
                    },
                    "$addToSet": {
                        "names": name,
                    },
                    "$setOnInsert": {
                        "first_message": now,
                    },
                    "$inc": {
                        "use_count": 1,
                    },
                },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;
    }

    Ok(())
}
