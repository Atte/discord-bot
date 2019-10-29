use crate::discord_eventhandler::get_log_channels;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::Colour,
};

#[command]
#[description("Manage the public pin on the current channel.")]
#[usage("new_text\u{2026}")]
#[only_in("guilds")]
#[help_available(false)]
pub fn pin(context: &mut Context, message: &Message, args: Args) -> CommandResult {
    let pintext = args.message();
    let uid = context.cache.read().user.id;

    if let Some(channel) = message.channel(&context).and_then(Channel::guild) {
        if let Some(mut pinned) = channel
            .read()
            .pins(&context)?
            .into_iter()
            .find(|msg| msg.author.id == uid)
        {
            pinned.edit(&context, |edit| edit.content(pintext))?;
        } else {
            channel
                .read()
                .send_message(&context, |msg| msg.content(pintext))?
                .pin(&context)?;
        }

        for log_channel in get_log_channels(&context, channel.read().guild_id) {
            log_channel.send_message(&context, |msg| {
                msg.embed(|e| {
                    e.colour(Colour::BLUE)
                        .description(format!(
                            "**<@{}> changed the public pin on <#{}>**\n{}",
                            message.author.id,
                            channel.read().id,
                            pintext
                        ))
                        .author(|a| {
                            a.name(&message.author.tag())
                                .icon_url(&message.author.face())
                        })
                        .timestamp(&message.timestamp)
                })
            })?;
        }
    }
    Ok(())
}
