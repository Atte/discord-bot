use crate::discord::{get_data, ConfigKey, DbKey};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use chrono::{SubsecRound, Utc};
use color_eyre::eyre::eyre;
use lazy_regex::regex_captures;
use mongodb::bson::{doc, Document};
use serenity::{
    all::{CreateAttachment, CreateMessage},
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::channel::Message,
    utils::MessageBuilder,
};
use std::{
    io::{Cursor, Write},
    time::Duration,
};
use tokio::time::sleep;
use zip::{write::SimpleFileOptions, ZipWriter};

const DELAY: Duration = Duration::from_millis(100);

#[command]
#[required_permissions(MANAGE_EMOJIS_AND_STICKERS)]
#[description("Approve an emote submission")]
#[usage("emotename")]
#[num_args(1)]
async fn emote(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let Some((_, emotename)) = regex_captures!(
        r"^(?:<a?:)?([A-Za-z0-9_]{2,})(?::\d+>)?$",
        args.message().trim()
    ) else {
        msg.reply(ctx, "Invalid emote name!").await?;
        return Ok(());
    };

    let guild = msg
        .guild(&ctx.cache)
        .ok_or_else(|| eyre!("Guild not found"))?
        .clone();

    let Some((channel_id, message_id)) = msg
        .message_reference
        .as_ref()
        .and_then(|r| r.message_id.map(|id| (r.channel_id, id)))
    else {
        msg.reply(ctx, "No message replied to!").await?;
        return Ok(());
    };

    let replied;
    if let Some(referenced) = ctx
        .cache
        .message(channel_id, message_id)
        .map(|msg| msg.clone())
    {
        replied = referenced.clone();
    } else if let Ok(referenced) = ctx.http.get_message(channel_id, message_id).await {
        replied = referenced;
    } else {
        msg.reply(ctx, "Replied to message not found!").await?;
        return Ok(());
    }

    let Some(image) = replied.attachments.iter().find(|a| {
        if let Some(ref ct) = a.content_type {
            ["image/png", "image/jpeg", "image/gif"].contains(&ct.as_str())
        } else {
            false
        }
    }) else {
        msg.reply(ctx, "No image attachments in replied to message!")
            .await?;
        return Ok(());
    };

    if image.size > 256_000 {
        msg.reply(ctx, "Image too large! (max 256 kB)").await?;
        return Ok(());
    }

    let data = BASE64_STANDARD.encode(&image.download().await?);

    let old_emojis: Vec<_> = guild
        .emojis
        .values()
        .filter(|e| e.name == emotename)
        .collect();

    let emoji = guild
        .create_emoji(
            ctx,
            emotename,
            &format!(
                "data:{};base64,{data}",
                image
                    .content_type
                    .as_ref()
                    .ok_or_else(|| eyre!("Missing content-type header"))?
            ),
        )
        .await?;

    for old in old_emojis {
        sleep(DELAY).await;
        old.delete(&ctx).await?;
    }

    let config = get_data::<ConfigKey>(ctx).await?;
    let rewards: Vec<_> = config
        .discord
        .emote_reward_roles
        .into_iter()
        .filter(|role| guild.roles.contains_key(role))
        .collect();
    if !rewards.is_empty() {
        let member = guild.member(ctx, replied.author.id).await?;
        for reward in &rewards {
            let _: Result<_, _> = member.add_role(ctx, reward).await;
        }
    }

    let collection = get_data::<DbKey>(ctx)
        .await?
        .collection::<Document>("emote-submissions");
    collection
        .insert_one(doc! {
            "time": Utc::now(),
            "guild_id": guild.id.to_string(),
            "emote_id": emoji.id.to_string(),
            "emote_name": &emoji.name,
            "user_id": replied.author.id.to_string(),
            "user_name": replied.author.name,
            "approver_id": msg.author.id.to_string(),
            "approver_name": &msg.author.name
        })
        .await?;

    msg.reply(
        &ctx,
        MessageBuilder::new()
            .push(config.discord.emote_reward_message.unwrap_or_default())
            .push_safe(" ")
            .emoji(&emoji)
            .build(),
    )
    .await?;

    Ok(())
}

#[command]
#[required_permissions(MANAGE_EMOJIS_AND_STICKERS)]
#[description("Download a ZIP file of all emotes")]
#[num_args(0)]
async fn download_emotes(ctx: &Context, msg: &Message) -> CommandResult {
    let _typing = msg.channel_id.start_typing(&ctx.http);

    let mut count_static = 0;
    let mut count_animated = 0;
    let mut buffer: Vec<u8> = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));

        let client = reqwest::Client::new();
        for emoji in msg
            .guild_id
            .ok_or_else(|| eyre!("Guild not found"))?
            .emojis(ctx)
            .await?
        {
            let response = client.get(emoji.url()).send().await?.error_for_status()?;
            let filetype = response
                .headers()
                .get("content-type")
                .ok_or_else(|| eyre!("Missing content-type header"))?
                .to_str()?
                .split('/')
                .last()
                .ok_or_else(|| eyre!("Invalid content-type header"))?;

            zip.start_file(
                format!("{}.{filetype}", emoji.name),
                SimpleFileOptions::default(),
            )?;
            zip.write_all(&response.bytes().await?)?;

            if emoji.animated {
                count_animated += 1;
            } else {
                count_static += 1;
            }

            sleep(DELAY).await;
        }

        zip.finish()?;
    }

    let filename = format!("emotes_{}.zip", Utc::now().round_subsecs(0));
    msg.channel_id
        .send_files(
            ctx,
            std::iter::once(CreateAttachment::bytes(buffer, filename)),
            CreateMessage::new().reference_message(msg).content(format!(
                "{} emotes ({count_static} static + {count_animated} animated)",
                count_static + count_animated
            )),
        )
        .await?;

    Ok(())
}

#[group]
#[only_in(guilds)]
#[commands(emote, download_emotes)]
pub struct Emotes;
