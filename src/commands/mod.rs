use crate::{util::can_respond_to, CONFIG};
use serenity::{framework::standard::StandardFramework, model::prelude::*};

mod derp;
mod misc;
mod pin;
mod ranks;

pub fn is_allowed(message: &Message, cmd: &str) -> bool {
    match cmd {
        "pin" => CONFIG.discord.pin_channels.contains(&message.channel_id),
        _ => can_respond_to(&message),
    }
}

pub fn register(framework: StandardFramework) -> StandardFramework {
    framework
        .command("ping", |cmd| {
            cmd.desc("Replies with a pong.").num_args(0).cmd(misc::ping)
        })
        .command("ranks", |cmd| {
            cmd.desc("Lists all available ranks, as well as the current user's active ones.")
                .num_args(0)
                .guild_only(true)
                .cmd(ranks::list)
        })
        .command("rank", |cmd| {
            cmd.desc("Joins/leaves a rank.")
                .usage("rankname")
                .num_args(1)
                .guild_only(true)
                .cmd(ranks::joinleave)
        })
        .command("roll", |cmd| {
            cmd.desc("Rolls dice.").usage("1d6 + 2d20").cmd(misc::roll)
        })
        .command("info", |cmd| {
            cmd.desc("Shows information about the bot.")
                .num_args(0)
                .cmd(misc::info)
        })
        .command("gib", |cmd| {
            cmd.desc("Gibs pics from derpibooru.")
                .usage("[tags\u{2026}]")
                .cmd(derp::gib)
        })
        .command("pin", |cmd| {
            cmd.desc("Manage the public pin on the current channel.")
                .usage("new_text\u{2026}")
                .guild_only(true)
                .cmd(pin::pin)
        })
}
