use super::DiscordConfigKey;
use crate::{eyre::Report, Result};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        channel::{Channel, GuildChannel, Message},
        guild::Member,
        id::GuildId,
        user::User,
    },
    prelude::Mentionable,
    utils::Colour,
};

#[derive(Debug, Clone)]
pub enum Person {
    Member(Member),
    User(User),
}

async fn send_log(
    ctx: &Context,
    guild_id: GuildId,
    create_embed: impl Fn(&mut CreateEmbed),
) -> Result<()> {
    for channel_id in DiscordConfigKey::get(&ctx).await.log_channels {
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
            Err(err) => return Err(Report::new(err)),
        }
    }
    Ok(())
}

pub async fn message_deleted(
    ctx: &Context,
    channel: &GuildChannel,
    message: Message,
) -> Result<()> {
    let content = message.content_safe(&ctx).await;
    send_log(&ctx, channel.guild_id, |embed| {
        embed.color(Colour::RED);
        embed.author(|author| {
            author
                .name(message.author.tag())
                .icon_url(message.author.face())
        });
        embed.title(format!(
            "Message sent by {} deleted in {}",
            message.author.mention(),
            channel.mention()
        ));
        embed.description(&content);
        embed.timestamp(&message.timestamp);
    })
    .await?;
    Ok(())
}

pub async fn member_added(ctx: &Context, member: &Member) -> Result<()> {
    Ok(())
}

pub async fn member_removed(ctx: &Context, guild_id: GuildId, person: Person) -> Result<()> {
    Ok(())
}

pub async fn member_updated(
    ctx: &Context,
    old_member: Option<Member>,
    new_member: &Member,
) -> Result<()> {
    Ok(())
}
