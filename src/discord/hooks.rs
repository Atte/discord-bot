use super::{get_data, DbKey};
use crate::Result;
use chrono::Utc;
use futures::join;
use humantime::format_duration;
use lazy_static::lazy_static;
use log::{error, warn};
use mongodb::{bson::doc, options::UpdateOptions};
use regex::Regex;
use serenity::{
    client::Context,
    framework::standard::{macros::hook, DispatchError},
    model::channel::{Channel, Message},
};
use std::{convert::TryInto, time::Duration};

async fn update_stats(ctx: &Context, msg: &Message) -> Result<()> {
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
        let user_mentions: Vec<&str> = USER_MENTION_RE
            .captures_iter(&msg.content)
            .filter_map(|cap| cap.name("id").map(|m| m.as_str()))
            .collect();
        let channel_mentions: Vec<&str> = CHANNEL_MENTION_RE
            .captures_iter(&msg.content)
            .filter_map(|cap| cap.name("id").map(|m| m.as_str()))
            .collect();
        let role_mentions: Vec<&str> = ROLE_MENTION_RE
            .captures_iter(&msg.content)
            .filter_map(|cap| cap.name("id").map(|m| m.as_str()))
            .collect();
        let emojis: Vec<(&str, &str)> = EMOJI_RE
            .captures_iter(&msg.content)
            .filter_map(|cap| {
                cap.name("id")
                    .and_then(|id| cap.name("name").map(|name| (id.as_str(), name.as_str())))
            })
            .collect();
        let collection = database.collection("stats");
        join!(
            collection.update_one(
                doc! {
                    "type": "channel",
                    "id": channel.id,
                },
                doc! {
                    "name": channel.name,
                    { "$addToSet": {
                        "names": channel.name,
                    } },
                    "last_message": now,
                    { "$setOnInsert": {
                        "first_message": now,
                    } },
                    { "$inc": {
                        "message_count": 1,
                        "user_mention_count": user_mentions.len(),
                        "channel_mention_count": channel_mentions.len(),
                        "role_mention_count": role_mentions.len(),
                        "emoji_count": emojis.len(),
                    } },
                },
                UpdateOptions::builder().upsert(true).build(),
            ),
            collection.update_one(
                doc! {
                    "type": "user",
                    "id": msg.author.id,
                },
                doc! {
                    "name": msg.author.name,
                    "discriminator": msg.author.discriminator,
                    { "$addToSet": {
                        "nicks": nick,
                    } },
                    "last_message": now,
                    { "$setOnInsert": {
                        "first_message": now,
                    } },
                    { "$inc": {
                        "message_count": 1,
                        "user_mention_count": user_mentions.len(),
                        "channel_mention_count": channel_mentions.len(),
                        "role_mention_count": role_mentions.len(),
                        "emoji_count": emojis.len(),
                    } },
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
                    "name": name,
                    "last_message": now,
                    { "$setOnInsert": {
                        "first_message": now,
                    } },
                    { "$inc": {
                        "use_count": 1,
                    } },
                },
                UpdateOptions::builder().upsert(true).build(),
            );
        }
    }
    Ok(())
}

#[hook]
pub async fn normal_message(ctx: &Context, msg: &Message) {
    if let Err(err) = update_stats(&ctx, &msg).await {
        error!("Error in log_message: {:?}", err);
    }
}

#[hook]
pub async fn unrecognised_command(ctx: &Context, msg: &Message, _command_name: &str) {
    let _ = msg.reply(&ctx, "That's not a command!").await;
}

#[hook]
pub async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::CheckFailed(desc, reason) => {
            warn!("Custom check failed: {} ({:?})", desc, reason);
        }
        DispatchError::Ratelimited(wait) => {
            let _ = msg
                .reply(
                    &ctx,
                    format!(
                        "Ratelimited! Wait {} before trying again.",
                        format_duration(Duration::from_secs(wait.try_into().unwrap_or(0)))
                    ),
                )
                .await;
        }
        DispatchError::CommandDisabled(something) => {
            warn!("Refused to dispatch disabled command: {}", something);
        }
        DispatchError::BlockedUser => {
            warn!("Refused to dispatch for blocked user");
        }
        DispatchError::BlockedGuild => {
            warn!("Refused to dispatch for blocked guild");
        }
        DispatchError::BlockedChannel => {
            warn!("Refused to dispatch for blocked channel");
        }
        DispatchError::OnlyForDM => {
            warn!("Refused to dispatch command that's only for DMs");
        }
        DispatchError::OnlyForGuilds => {
            warn!("Refused to dispatch command that's only for guilds");
        }
        DispatchError::OnlyForOwners => {
            warn!("Refused to dispatch command that's only for owners");
        }
        DispatchError::LackingRole => {
            warn!("Refused to dispatch command due to lacking role");
        }
        DispatchError::LackingPermissions(perms) => {
            warn!(
                "Refused to dispatch command due to lacking permissions: {:?}",
                perms
            );
        }
        DispatchError::NotEnoughArguments { min, given } => {
            let _ = msg
                .reply(
                    &ctx,
                    format!(
                        "Need at least {} argument{}, got {}",
                        min,
                        if min == 1 { "" } else { "s" },
                        given
                    ),
                )
                .await;
        }
        DispatchError::TooManyArguments { max, given } => {
            let _ = msg
                .reply(
                    &ctx,
                    format!(
                        "At most {} argument{} allowed, got {}",
                        max,
                        if max == 1 { "" } else { "s" },
                        given
                    ),
                )
                .await;
        }
        DispatchError::IgnoredBot => {
            warn!("Ignored command dispatch for bot");
        }
        DispatchError::WebhookAuthor => {
            warn!("Ignored command dispatch for webhook");
        }
        err => {
            error!("Dispatch error: {:?}", err);
        }
    }
}
