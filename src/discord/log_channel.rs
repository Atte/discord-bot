use super::{get_data, limits::EMBED_DESC_LENGTH, ConfigKey};
use crate::util::ellipsis_string;
use color_eyre::eyre::{Error, Result};
use serenity::{
    all::{Colour, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage},
    builder::CreateEmbed,
    client::Context,
    model::{
        channel::{Channel, Message},
        guild::Member,
        id::{ChannelId, GuildId},
        user::User,
    },
    utils::MessageBuilder,
};

async fn send_log(
    ctx: &Context,
    guild_id: GuildId,
    create_embed: impl Fn() -> CreateEmbed,
) -> Result<()> {
    let mut result = Ok(());
    for channel_id in get_data::<ConfigKey>(ctx).await?.discord.log_channels {
        match channel_id.to_channel(&ctx).await {
            Ok(Channel::Guild(channel)) if channel.guild_id == guild_id => {
                channel_id
                    .send_message(&ctx, CreateMessage::new().embed(create_embed()))
                    .await?;
            }
            Ok(_) => {} // ignore deletions outside guilds, and in irrelevant guilds
            Err(err) => result = Err(Error::new(err)),
        }
    }
    result
}

pub async fn message_deleted(
    ctx: &Context,
    channel_id: ChannelId,
    guild_id: GuildId,
    message: Message,
) -> Result<()> {
    // don't log deletions of logs
    if get_data::<ConfigKey>(ctx)
        .await?
        .discord
        .log_channels
        .contains(&channel_id)
    {
        return Ok(());
    }

    // don't log halloween stuff
    #[allow(clippy::unreadable_literal)]
    if message.author.id == 755580145078632508
        || message.content == "h!treat"
        || message.content == "h!trick"
    {
        return Ok(());
    }

    send_log(ctx, guild_id, || {
        CreateEmbed::new()
            .color(Colour::RED)
            .author({
                CreateEmbedAuthor::new(message.author.tag()).icon_url(message.author.face())
            })
            .description(ellipsis_string(
                MessageBuilder::new()
                    .push_bold_line(
                        MessageBuilder::new()
                            .push("Message sent by ")
                            .mention(&message.author)
                            .push(" on ")
                            .mention(&channel_id)
                            .push(" was deleted")
                            .build(),
                    )
                    .push(&message.content)
                    .build(),
                EMBED_DESC_LENGTH,
            ))
            .footer(CreateEmbedFooter::new("Originally posted"))
            .timestamp(message.timestamp)
    })
    .await?;
    Ok(())
}

pub async fn member_added(ctx: &Context, guild_id: GuildId, user: &User) -> Result<()> {
    send_log(ctx, guild_id, || {
        CreateEmbed::new()
            .color(Colour::DARK_GREEN)
            .author(CreateEmbedAuthor::new(user.tag()).icon_url(user.face()))
            .description(
                MessageBuilder::new()
                    .push_bold(MessageBuilder::new().mention(user).push(" joined").build())
                    .build(),
            )
    })
    .await?;
    Ok(())
}

pub async fn member_removed(ctx: &Context, guild_id: GuildId, user: &User) -> Result<()> {
    send_log(ctx, guild_id, || {
        CreateEmbed::new()
            .color(Colour::RED)
            .author(CreateEmbedAuthor::new(user.tag()).icon_url(user.face()))
            .description(
                MessageBuilder::new()
                    .push_bold(
                        MessageBuilder::new()
                            .mention(user)
                            .push(" left (or was kicked)")
                            .build(),
                    )
                    .build(),
            )
    })
    .await?;
    Ok(())
}

pub async fn member_updated(
    ctx: &Context,
    old_member: Option<&Member>,
    new_member: &Member,
) -> Result<()> {
    let old_name = old_member.map_or_else(
        || String::from("(unknown)"),
        |member| member.display_name().to_string(),
    );
    let new_name = new_member.display_name().to_string();

    if old_name != new_name {
        send_log(ctx, new_member.guild_id, || {
            CreateEmbed::new()
                .color(Colour::DARK_BLUE)
                .author(
                    CreateEmbedAuthor::new(new_member.user.tag()).icon_url(new_member.user.face()),
                )
                .description(
                    MessageBuilder::new()
                        .push_bold_line(
                            MessageBuilder::new()
                                .mention(new_member)
                                .push("'s nickname was changed (by them or by an admin)")
                                .build(),
                        )
                        .push_safe(&old_name)
                        .push(" \u{2192} ") // right arrow
                        .push_safe(&new_name)
                        .build(),
                )
        })
        .await?;
    }
    Ok(())
}

pub async fn rules_accepted(ctx: &Context, guild_id: GuildId, user: &User) -> Result<()> {
    send_log(ctx, guild_id, || {
        CreateEmbed::new()
            .color(Colour::DARK_GREEN)
            .author(CreateEmbedAuthor::new(user.tag()).icon_url(user.face()))
            .description(
                MessageBuilder::new()
                    .push_bold(
                        MessageBuilder::new()
                            .mention(user)
                            .push(" accepted the rules")
                            .build(),
                    )
                    .build(),
            )
    })
    .await?;
    Ok(())
}
