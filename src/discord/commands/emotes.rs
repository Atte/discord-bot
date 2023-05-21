use crate::discord::{get_data, ConfigKey};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use chrono::{SubsecRound, Utc};
use color_eyre::eyre::eyre;
use serenity::{
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
use zip::ZipWriter;

#[command]
#[required_permissions(MANAGE_EMOJIS_AND_STICKERS)]
#[description("Approve an emote submission")]
#[usage("emotename")]
#[num_args(1)]
async fn emote(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).ok_or_else(|| eyre!("Guild not found"))?;

    let (channel_id, message_id) = msg
        .message_reference
        .as_ref()
        .and_then(|r| r.message_id.map(|id| (r.channel_id, id)))
        .ok_or_else(|| eyre!("No message replied to!"))?;

    let replied = ctx
        .cache
        .message(channel_id, message_id)
        .ok_or_else(|| eyre!("Replied to message not found!"))?;

    let image = replied
        .attachments
        .iter()
        .find(|a| a.dimensions().is_some())
        .ok_or_else(|| eyre!("No image attachments in replied to message!"))?;

    if image.size > 256_000 {
        Err(eyre!("Image too large! (max 256 kB)"))?;
    }

    let data = BASE64_STANDARD.encode(&image.download().await?);

    let emotename = args.message().trim();
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

    let config = get_data::<ConfigKey>(ctx).await?;
    let rewards: Vec<_> = config
        .discord
        .emote_reward_roles
        .into_iter()
        .filter(|role| guild.roles.contains_key(role))
        .collect();
    if !rewards.is_empty() {
        guild
            .member(ctx, msg.author.id)
            .await?
            .add_roles(ctx, rewards.as_slice())
            .await?;
    }

    msg.reply(&ctx, MessageBuilder::new().emoji(&emoji).build())
        .await?;

    Ok(())
}

#[command]
#[required_permissions(MANAGE_EMOJIS_AND_STICKERS)]
#[num_args(0)]
async fn download_emotes(ctx: &Context, msg: &Message) -> CommandResult {
    let _typing = msg.channel_id.start_typing(&ctx.http)?;

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
            zip.start_file(format!("{}.{filetype}", emoji.name), Default::default())?;
            zip.write_all(&response.bytes().await?)?;

            if emoji.animated {
                count_animated += 1;
            } else {
                count_static += 1;
            }

            sleep(Duration::from_millis(100)).await;
        }

        zip.finish()?;
    }

    let filename = format!("emotes_{}.zip", Utc::now().round_subsecs(0));
    msg.channel_id
        .send_files(ctx, vec![(buffer.as_slice(), filename.as_str())], |m| {
            m.reference_message(msg).content(format!(
                "{} emotes ({count_static} static + {count_animated} animated)",
                count_static + count_animated
            ))
        })
        .await?;

    Ok(())
}

#[command]
#[required_permissions(ADMINISTRATOR)]
#[num_args(0)]
async fn nuke_emotes(ctx: &Context, msg: &Message) -> CommandResult {
    download_emotes(ctx, msg, Args::new("", &[])).await?;
    return Err(eyre!("This command is disabled!"))?;

    let _typing = msg.channel_id.start_typing(&ctx.http)?;
    for emoji in msg
        .guild_id
        .ok_or_else(|| eyre!("Guild not found"))?
        .emojis(ctx)
        .await?
    {
        emoji.delete(ctx).await?;
        sleep(Duration::from_millis(100)).await;
    }
    msg.reply(&ctx, "All emotes deleted!").await?;

    Ok(())
}

#[group]
#[only_in(guilds)]
#[commands(emote, download_emotes, nuke_emotes)]
pub struct Emotes;
