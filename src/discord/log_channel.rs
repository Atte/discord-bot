use super::{get_data, limits::EMBED_DESC_LENGTH, DiscordConfigKey};
use crate::{eyre::Report, util::ellipsis_string, Result};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        channel::{Channel, GuildChannel, Message},
        guild::Member,
        id::GuildId,
        user::User,
    },
    utils::{Colour, MessageBuilder},
};

async fn send_log(
    ctx: &Context,
    guild_id: GuildId,
    create_embed: impl Fn(&mut CreateEmbed),
) -> Result<()> {
    let mut result = Ok(());
    for channel_id in get_data::<DiscordConfigKey>(&ctx).await?.log_channels {
        match channel_id.to_channel(&ctx).await {
            Ok(Channel::Guild(channel)) if channel.guild_id == guild_id => {
                channel_id
                    .send_message(&ctx, |builder| {
                        builder.embed(|mut embed| {
                            create_embed(&mut embed);
                            embed
                        })
                    })
                    .await?;
            }
            Ok(_) => {} // ignore deletions outside guilds, and in irrelevant guilds
            Err(err) => result = Err(Report::new(err)),
        }
    }
    result
}

pub async fn message_deleted(
    ctx: &Context,
    channel: &GuildChannel,
    message: Message,
) -> Result<()> {
    // don't log deletions of logs
    if get_data::<DiscordConfigKey>(&ctx)
        .await?
        .log_channels
        .contains(&channel.id)
    {
        return Ok(());
    }
    send_log(&ctx, channel.guild_id, |embed| {
        embed.color(Colour::RED);
        embed.author(|author| {
            author
                .name(message.author.tag())
                .icon_url(message.author.face())
        });
        embed.description(ellipsis_string(
            MessageBuilder::new()
                .push_bold_line(
                    MessageBuilder::new()
                        .push("Message sent by ")
                        .mention(&message.author)
                        .push(" on ")
                        .mention(channel)
                        .push(" was deleted")
                        .build(),
                )
                .push(&message.content)
                .build(),
            EMBED_DESC_LENGTH,
        ));
        embed.footer(|footer| footer.text("Originally posted"));
        embed.timestamp(&message.timestamp);
    })
    .await?;
    Ok(())
}

pub async fn member_added(ctx: &Context, guild_id: GuildId, user: &User) -> Result<()> {
    send_log(&ctx, guild_id, |embed| {
        embed.color(Colour::RED);
        embed.author(|author| author.name(user.tag()).icon_url(user.face()));
        embed.description(
            MessageBuilder::new()
                .push_bold(MessageBuilder::new().mention(user).push(" joined").build())
                .build(),
        );
    })
    .await?;
    Ok(())
}

pub async fn member_removed(ctx: &Context, guild_id: GuildId, user: &User) -> Result<()> {
    send_log(&ctx, guild_id, |embed| {
        embed.color(Colour::RED);
        embed.author(|author| author.name(user.tag()).icon_url(user.face()));
        embed.description(
            MessageBuilder::new()
                .push_bold(
                    MessageBuilder::new()
                        .mention(user)
                        .push(" left (or was kicked)")
                        .build(),
                )
                .build(),
        );
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
        send_log(&ctx, new_member.guild_id, |embed| {
            embed.color(Colour::RED);
            embed.author(|author| {
                author
                    .name(new_member.user.tag())
                    .icon_url(new_member.user.face())
            });
            embed.description(
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
            );
        })
        .await?;
    }
    Ok(())
}
