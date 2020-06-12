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
use std::{collections::HashSet, time::Duration};

const READ_TIMEOUT: Duration = Duration::from_secs(2);

mod derp;
mod misc;
mod pin;
mod ranks;

use derp::*;
use misc::*;
use pin::*;
use ranks::*;

#[group]
#[commands(gib)]
struct Horse;

#[group]
#[commands(ranks, rank, join, leave, pin)]
struct Discord;

#[group]
#[commands(roll, ping, info)]
struct Misc;

#[help]
#[lacking_conditions("hide")]
#[lacking_ownership("hide")]
#[lacking_permissions("hide")]
#[lacking_role("hide")]
#[wrong_channel("hide")]
fn help_command(
    context: &mut Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

pub fn is_allowed(context: &Context, message: &Message, cmd: &str) -> bool {
    match cmd {
        "pin" => CONFIG.discord.pin_channels.contains(&message.channel_id),
        _ => can_respond_to(&context, &message),
    }
}
