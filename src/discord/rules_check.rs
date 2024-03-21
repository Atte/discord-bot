use super::{get_data, log_channel, ConfigKey};
use color_eyre::eyre::{eyre, Result};
use serenity::{
    all::{CreateMessage, Message},
    client::Context,
    model::{
        channel::{Reaction, ReactionType},
        guild::Member,
        id::RoleId,
    },
    utils::MessageBuilder,
};

// ✅
pub const EMOJI: &str = "\u{2705}";

pub async fn post_welcome(ctx: Context, member: Member) -> Result<()> {
    let guild = member
        .guild_id
        .to_guild_cached(&ctx)
        .ok_or_else(|| eyre!("Guild not found!"))?
        .clone();

    let config = get_data::<ConfigKey>(&ctx).await?;
    let url = config
        .discord
        .rules_url
        .map_or_else(|| "missing link".to_owned(), |url| url.to_string());

    for channel_id in config.discord.rules_channels {
        if let Some(channel) = guild.channels.get(&channel_id) {
            // TODO: pull text from DB
            let message = channel.send_message(&ctx, CreateMessage::new().content(
                    MessageBuilder::new()
                        .mention(&member)
                        .push_line_safe(":")
                        .push_bold_line_safe(format!("Welcome to {}!", &guild.name))
                        .push_line_safe(format!("To access all the channels, please confirm you have read and accepted the rules: <{url}>"))
                        .push_safe(format!("Confirm by clicking the {EMOJI} reaction on this message."))
                        .build()
                )
            ).await?;
            message.react(&ctx, ReactionType::try_from(EMOJI)?).await?;
        }
    }

    Ok(())
}

pub async fn handle_reaction(ctx: Context, reaction: Reaction) -> Result<()> {
    let message: Option<Message> = ctx
        .cache
        .message(reaction.channel_id, reaction.message_id)
        .map(|msg| msg.clone());

    let message = match message {
        Some(msg) => msg,
        None => reaction.message(&ctx).await?.clone(),
    };

    if !message.is_own(&ctx) {
        // not bot message
        return Ok(());
    }

    let config = get_data::<ConfigKey>(&ctx).await?;
    if !config
        .discord
        .rules_url
        .map_or(false, |url| message.content.contains(&url.to_string()))
    {
        // not rules message
        return Ok(());
    }

    if !reaction.emoji.unicode_eq(EMOJI) {
        // wrong emote
        reaction.delete_all(&ctx).await?;
        return Ok(());
    }

    let guild = reaction
        .guild_id
        .ok_or_else(|| eyre!("No guild_id in reaction!"))?
        .to_guild_cached(&ctx)
        .ok_or_else(|| eyre!("Guild not found!"))?
        .clone();

    let user = reaction.user(&ctx).await?;
    let member = guild.member(&ctx, user.id).await?;

    let missing_roles: Vec<RoleId> = config
        .discord
        .rules_roles
        .into_iter()
        .filter(|role_id| guild.roles.contains_key(role_id) && !member.roles.contains(role_id))
        .collect();

    if missing_roles.is_empty() {
        reaction.delete(&ctx).await?;
    } else {
        member.add_roles(&ctx, &missing_roles).await?;
        log_channel::rules_accepted(&ctx, guild.id, &user).await?;
    }

    Ok(())
}
