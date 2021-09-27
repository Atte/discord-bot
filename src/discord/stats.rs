use super::{get_data, DbKey};
use anyhow::{bail, Result};
use chrono::Utc;
use lazy_static::lazy_static;
use mongodb::{
    bson::{doc, Document},
    options::UpdateOptions,
};
use regex::Regex;
use serenity::{
    client::Context,
    model::{
        channel::{Channel, Message},
        id::{ChannelId, EmojiId, RoleId, UserId},
    },
};

const COLLECTION_NAME: &str = "stats";

#[allow(clippy::too_many_lines)]
pub async fn update_stats(ctx: &Context, msg: &Message) -> Result<()> {
    lazy_static! {
        static ref USER_MENTION_RE: Regex =
            Regex::new(r#"<@!?(?P<id>[0-9]+)>"#).expect("Invalid regex for USER_MENTION_RE");
        static ref CHANNEL_MENTION_RE: Regex =
            Regex::new(r#"<#(?P<id>[0-9]+)>"#).expect("Invalid regex for CHANNEL_MENTION_RE");
        static ref ROLE_MENTION_RE: Regex =
            Regex::new(r#"<@#(?P<id>[0-9]+)>"#).expect("Invalid regex for ROLE_MENTION_RE");
        static ref EMOJI_RE: Regex = Regex::new(r#"<a?:(?P<name>[^:]+):(?P<id>[0-9]+)>"#)
            .expect("Invalid regex for EMOJI_RE");
    }

    let channel = match msg.channel(&ctx).await {
        Some(Channel::Guild(inner)) => inner,
        Some(_) => bail!("Not a guild channel"),
        None => bail!("Channel not in cache"),
    };

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
                .and_then(|c| c.as_str().parse::<u64>().map(EmojiId).ok())
                .zip(cap.name("name").map(|c| c.as_str()))
        })
        .collect();

    let now = Utc::now();
    let collection = get_data::<DbKey>(ctx)
        .await?
        .collection::<Document>(COLLECTION_NAME);

    collection
        .update_one(
            doc! {
                "type": "channel",
                "id": channel.id.to_string(),
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
                    "user_mention_count": user_mentions.len() as i64,
                    "channel_mention_count": channel_mentions.len() as i64,
                    "role_mention_count": role_mentions.len() as i64,
                    "emoji_count": emojis.len() as i64,
                },
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;

    collection
        .update_one(
            doc! {
                "type": "user",
                "id": msg.author.id.to_string(),
            },
            doc! {
                "$set": {
                    "name": &msg.author.name,
                    "discriminator": i32::from(msg.author.discriminator),
                    "nick": &nick,
                    "last_message": now,
                },
                "$addToSet": {
                    "nicks": nick,
                },
                "$setOnInsert": {
                    "first_message": now,
                },
                "$inc": {
                    "message_count": 1,
                    "user_mention_count": user_mentions.len() as i64,
                    "channel_mention_count": channel_mentions.len() as i64,
                    "role_mention_count": role_mentions.len() as i64,
                    "emoji_count": emojis.len() as i64,
                },
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;

    for (EmojiId(id), name) in emojis {
        collection
            .update_one(
                doc! {
                    "type": "emoji",
                    "id": id.to_string(),
                },
                doc! {
                    "$set": {
                        "name": name,
                        "last_message": now,
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
