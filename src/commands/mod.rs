use crate::{util::can_respond_to, CONFIG};
use serenity::{
    framework::standard::{
        help_commands,
        macros::{group, help},
        Args, CommandGroup, CommandResult, HelpOptions,
    },
    model::prelude::*,
    prelude::*,
};
use std::collections::HashSet;

mod derp;
mod misc;
mod pin;
mod ranks;

use derp::*;
use misc::*;
use pin::*;
use ranks::*;

group!({
    name: "horse",
    commands: [gib],
    options: {
        help_available: true
    }
});

group!({
    name: "discord",
    commands: [ranks, rank, pin],
    options: {
        help_available: true
    }
});

group!({
    name: "misc",
    commands: [roll, ping, info],
    options: {
        help_available: true
    }
});

#[help]
#[wrong_channel("hide")]
#[lacking_role("hide")]
#[lacking_ownership("hide")]
#[lacking_permissions("hide")]
fn help(
    context: &mut Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

pub fn is_allowed(message: &Message, cmd: &str) -> bool {
    match cmd {
        "pin" => CONFIG.discord.pin_channels.contains(&message.channel_id),
        _ => can_respond_to(&message),
    }
}
