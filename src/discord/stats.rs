use super::{get_data, DbKey};
use crate::Result;
use chrono::Utc;
use futures::join;
use lazy_static::lazy_static;
use mongodb::{bson::doc, options::UpdateOptions};
use regex::Regex;
use serenity::{
    client::Context,
    model::{
        channel::{Channel, Message},
        id::{ChannelId, EmojiId, RoleId, UserId},
    },
};

const COLLECTION_NAME: &str = "stats";

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

    if let (Some(Channel::Guild(channel)), Some(nick), Ok(database)) = join!(
        msg.channel(&ctx),
        msg.author_nick(&ctx),
        get_data::<DbKey>(&ctx),
    ) {
        let now = Utc::now();
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
        let collection = database.collection(COLLECTION_NAME);
        join!(
            collection.update_one(
                doc! {
                    "type": "channel",
                    "id": channel.id,
                },
                doc! {
                    "$set": {
                        "name": channel.name,
                        "last_message": now,
                    },
                    "$addToSet": {
                        "names": channel.name,
                    },
                    "$setOnInsert": {
                        "first_message": now,
                    },
                    "$inc": {
                        "message_count": 1,
                        "user_mention_count": user_mentions.len(),
                        "channel_mention_count": channel_mentions.len(),
                        "role_mention_count": role_mentions.len(),
                        "emoji_count": emojis.len(),
                    },
                },
                UpdateOptions::builder().upsert(true).build(),
            ),
            collection.update_one(
                doc! {
                    "type": "user",
                    "id": msg.author.id,
                },
                doc! {
                    "$set": {
                        "name": msg.author.name,
                        "discriminator": msg.author.discriminator,
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
                        "user_mention_count": user_mentions.len(),
                        "channel_mention_count": channel_mentions.len(),
                        "role_mention_count": role_mentions.len(),
                        "emoji_count": emojis.len(),
                    },
                },
                UpdateOptions::builder().upsert(true).build(),
            ),
        );
        for (id, name) in emojis {
            collection.update_one(
                doc! {
                    "type": "emoji",
                    "id": id,
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
            );
        }
    }
    Ok(())
}
