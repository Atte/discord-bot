use super::super::discord_eventhandler::get_log_channels;
use super::super::util;
use serenity::framework::standard::{Args, CommandError};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;

pub fn pin(_: &mut Context, message: &Message, args: Args) -> Result<(), CommandError> {
    let content = args.full();

    if let Some(channel) = message.channel().and_then(|ch| ch.guild()) {
        if let Some(mut pinned) = channel
            .read()
            .pins()?
            .into_iter()
            .find(|msg| msg.author.id == util::uid())
        {
            pinned.edit(|edit| edit.content(content))?;
        } else {
            channel
                .read()
                .send_message(|msg| msg.content(content))?
                .pin()?;
        }

        for log_channel in get_log_channels(channel.read().guild_id) {
            log_channel.send_message(|msg| {
                msg.embed(|e| {
                    e.colour(Colour::blue())
                        .description(format!(
                            "**<@{}> changed the public pin on <#{}>**\n{}",
                            message.author.id,
                            channel.read().id,
                            content
                        )).author(|a| {
                            a.name(&message.author.tag())
                                .icon_url(&message.author.face())
                        }).timestamp(&message.timestamp)
                })
            })?;
        }
    }
    Ok(())
}
