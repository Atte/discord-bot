use super::READ_TIMEOUT;
use crate::CONFIG;
use lazy_static::lazy_static;
use rand::{self, Rng};
use regex::{Captures, Regex};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::Colour,
};

#[command]
#[description("pong")]
#[num_args(0)]
pub fn ping(context: &mut Context, message: &Message, _: Args) -> CommandResult {
    message.reply(&context, "Pong! <:DIDNEYWORL:365990182610272266>")?;
    Ok(())
}

#[command]
#[description("Cast die")]
#[usage("1d6 + 2d20")]
pub fn roll(context: &mut Context, message: &Message, args: Args) -> CommandResult {
    lazy_static! {
        static ref DIE_RE: Regex = Regex::new(r"(\d+)?d(\d+)").expect("Invalid DIE_RE");
    }

    let original = if args.is_empty() {
        "1d6"
    } else {
        args.message()
    };
    let rolled = DIE_RE.replace_all(original, |caps: &Captures<'_>| {
        let rolls: usize = caps
            .get(1)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(1);
        let sides: usize = caps
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(6);
        if rolls < 1 {
            String::new()
        } else if sides < 1 {
            "0".to_owned()
        } else {
            let results: Vec<String> = (0..rolls)
                .map(|_| rand::thread_rng().gen_range(1, sides + 1).to_string())
                .collect();
            results.join(" + ")
        }
    });
    let result = meval::eval_str(&rolled)?;
    let output = format!("{} \u{2192} {} \u{2192} **{}**", original, rolled, result);
    if result.to_string() == rolled
        || original == rolled
        || output.len() > CONFIG.discord.long_msg_threshold
    {
        message.reply(&context, &format!("{} \u{2192} **{}**", original, result))?;
    } else {
        message.reply(&context, &output)?;
    }
    Ok(())
}

#[command]
#[description("Show information about the bot")]
#[num_args(0)]
pub fn info(context: &mut Context, message: &Message, _: Args) -> CommandResult {
    let avatar = context
        .cache
        .try_read_for(READ_TIMEOUT)
        .and_then(|cache| cache.user.avatar_url());
    message.channel_id.send_message(&context, |msg| {
        msg.embed(|mut e| {
            if let Some(avatar) = avatar {
                e = e.thumbnail(avatar);
            }
            e.colour(Colour::GOLD)
                .field("Author", "<@119122043923988483>", false)
                .field("Source code", "https://github.com/Atte/discord-bot", false)
                .footer(|f| {
                    f.text(&format!(
                        "Use {}help for a list of available commands.",
                        CONFIG.discord.command_prefix
                    ))
                })
        })
    })?;
    Ok(())
}
