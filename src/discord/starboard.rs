use bson::doc;
use color_eyre::eyre::{OptionExt, Result};
use log::warn;
use rand::{Rng, SeedableRng, distr::Uniform};
use rand_pcg::Pcg32;
use serde::{Deserialize, Serialize};
use serenity::all::{
    Context, CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, CreateMessage, Guild, Message,
    MessageBuilder, PermissionOverwriteType, Permissions, Reaction, ReactionType, Role, RoleId,
};

use crate::{
    config::Config,
    discord::{DbKey, get_data},
};

const COLLECTION_NAME: &str = "starboard";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StarredMessage {
    message_id: String,
}

fn is_star_emoji(config: &Config, emoji: &ReactionType) -> bool {
    match emoji {
        ReactionType::Custom { id, name, .. } => {
            id.to_string() == *config.starboard.emoji
                || name
                    .as_ref()
                    .is_some_and(|name| name == &*config.starboard.emoji)
        }
        ReactionType::Unicode(s) => s == &*config.starboard.emoji,
        emoji => {
            warn!("Unknown reaction type: {emoji:?}");
            false
        }
    }
}

#[allow(clippy::too_many_lines)]
pub async fn on_reaction_change(ctx: &Context, reaction: Reaction) -> Result<()> {
    // log::trace!("{reaction:?}");

    let guild_id = reaction.guild_id.ok_or_eyre("no guild id")?;
    let guild = guild_id
        .to_guild_cached(ctx)
        .ok_or_eyre("Guild not in cache")?
        .clone();

    let config = super::get_data::<super::ConfigKey>(ctx).await?;
    if !is_star_emoji(&config, &reaction.emoji) {
        log::trace!("not a star emoji");
        return Ok(());
    }

    let collection = get_data::<DbKey>(ctx)
        .await?
        .collection::<StarredMessage>(COLLECTION_NAME);
    if collection
        .find_one(doc! {
            "message_id": reaction.message_id.to_string()
        })
        .await?
        .is_some()
    {
        log::trace!("already on board");
        return Ok(());
    }

    let message = if let Some(msg) = ctx.cache.message(reaction.channel_id, reaction.message_id) {
        msg.to_owned()
    } else {
        ctx.http
            .get_message(reaction.channel_id, reaction.message_id)
            .await?
    };

    let mut channels = vec![
        message
            .channel(&ctx)
            .await?
            .guild()
            .ok_or_eyre("Message not in guild channel")?,
    ];
    while let Some(parent_id) = channels.last().unwrap().parent_id {
        channels.push(
            parent_id
                .to_channel(&ctx)
                .await?
                .guild()
                .ok_or_eyre("Parent channel not a guild channel")?,
        );
    }

    if !config
        .starboard
        .channels
        .is_disjoint(&channels.iter().map(|c| c.id).collect())
    {
        log::trace!("starboard channel, ignoring");
        return Ok(());
    }

    for role in guild_roles(&guild, &config.starboard.ignore_channels) {
        for channel in &channels {
            if channel.permission_overwrites.iter().any(|overwrite| {
                overwrite.kind == PermissionOverwriteType::Role(role.id)
                    && overwrite.deny.contains(Permissions::VIEW_CHANNEL)
            }) {
                log::trace!("channel ignored by role");
                return Ok(());
            }
        }
    }

    let threshold = if let Some(max_threshold) = config.starboard.max_threshold {
        let distr = Uniform::new_inclusive(config.starboard.threshold, max_threshold)?;
        let mut rand = Pcg32::seed_from_u64(reaction.message_id.get());
        rand.sample(distr)
    } else {
        config.starboard.threshold
    };

    let Some(message_reaction) = message
        .reactions
        .iter()
        .find(|r| is_star_emoji(&config, &r.reaction_type))
    else {
        return Ok(());
    };
    if message_reaction.count < threshold {
        log::trace!("total {} < {threshold}", message_reaction.count);
        return Ok(());
    }

    for role in guild_roles(&guild, &config.starboard.ignore_messages) {
        if message
            .author
            .has_role(&ctx, guild_id, role.id)
            .await
            .unwrap_or(false)
        {
            log::trace!("author ignored");
            return Ok(());
        }
    }

    let reaction_users = message
        .reaction_users(ctx, message_reaction.reaction_type.clone(), Some(100), None)
        .await?;
    let mut reaction_count = 0;
    for role in guild_roles(&guild, &config.starboard.ignore_stars) {
        for user in &reaction_users {
            if !user
                .has_role(&ctx, guild_id, role.id)
                .await
                .unwrap_or(false)
            {
                reaction_count += 1;
            }
        }
    }
    if reaction_count < threshold {
        log::trace!("filtered {reaction_count} < {threshold}");
        return Ok(());
    }

    let Some(board_channel_id) = config
        .starboard
        .channels
        .iter()
        .find(|id| guild.channels.contains_key(id))
    else {
        log::trace!("no starboard channel found");
        return Ok(());
    };
    let board_channel = board_channel_id
        .to_channel(ctx)
        .await?
        .guild()
        .ok_or_eyre("Starboard not a guild channel")?;

    if channels.iter().any(|c| c.nsfw) && !board_channel.nsfw {
        log::trace!("message is NSFW, but starboard is not");
        return Ok(());
    }

    let mut pin = CreateMessage::new()
        .allowed_mentions(CreateAllowedMentions::new())
        .content(MessageBuilder::new().push(message.link()).build());
    if let Some(ref replied) = message.referenced_message {
        pin = pin.add_embed(create_embed_from_message(ctx, &guild, replied).await?);
    }
    pin = pin.add_embed(create_embed_from_message(ctx, &guild, &message).await?);
    board_channel_id.send_message(ctx, pin).await?;

    collection
        .insert_one(StarredMessage {
            message_id: reaction.message_id.to_string(),
        })
        .await?;

    Ok(())
}

async fn create_embed_from_message(
    ctx: &Context,
    guild: &Guild,
    msg: &Message,
) -> Result<CreateEmbed> {
    let member = guild.member(ctx, msg.author.id).await?;
    Ok(CreateEmbed::new()
        .author(CreateEmbedAuthor::new(member.display_name()).icon_url(member.face()))
        .url(msg.link())
        .description(&msg.content)
        .timestamp(msg.timestamp))
}

fn guild_roles<'id>(
    guild: &Guild,
    role_ids: impl IntoIterator<Item = &'id RoleId>,
) -> impl Iterator<Item = &Role> {
    role_ids
        .into_iter()
        .filter_map(move |role_id| guild.roles.get(role_id))
}
