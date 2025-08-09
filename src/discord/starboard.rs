use bson::doc;
use color_eyre::eyre::{OptionExt, Result};
use log::warn;
use rand::{Rng, SeedableRng, distr::Uniform};
use rand_pcg::Pcg32;
use serde::{Deserialize, Serialize};
use serenity::all::{
    Context, CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, CreateMessage, GuildId,
    Message, MessageBuilder, Reaction, ReactionType,
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

    for role_id in &config.starboard.ignore_messages {
        if message
            .author
            .has_role(&ctx, guild_id, role_id)
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
    for role_id in &config.starboard.ignore_stars {
        if guild_id.role(ctx, *role_id).await.is_err() {
            continue;
        }
        for user in &reaction_users {
            if !user
                .has_role(&ctx, guild_id, role_id)
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

    let Some(board_channel_id) = ({
        let guild = guild_id
            .to_guild_cached(ctx)
            .ok_or_eyre("Guild not in cache")?;
        config
            .starboard
            .channels
            .iter()
            .find(|id| guild.channels.contains_key(id))
    }) else {
        log::trace!("no starboard channel found");
        return Ok(());
    };

    let mut pin = CreateMessage::new()
        .allowed_mentions(CreateAllowedMentions::new())
        .content(MessageBuilder::new().push(message.link()).build());
    if let Some(ref reply) = message.referenced_message {
        pin = pin.add_embed(create_embed_from_message(ctx, guild_id, reply).await?);
    }
    pin = pin.add_embed(create_embed_from_message(ctx, guild_id, &message).await?);
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
    guild_id: GuildId,
    msg: &Message,
) -> Result<CreateEmbed> {
    let member = guild_id.member(ctx, msg.author.id).await?;
    Ok(CreateEmbed::new()
        .author(CreateEmbedAuthor::new(member.display_name()).icon_url(member.face()))
        .url(msg.link())
        .description(&msg.content)
        .timestamp(msg.timestamp))
}
