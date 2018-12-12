use super::super::{util, CONFIG};
use lazy_static::lazy_static;
use meval;
use rand::{self, Rng};
use regex::{Captures, Regex};
use serenity::framework::standard::{Args, CommandError};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;
use serenity::CACHE;

pub fn ping(_: &mut Context, message: &Message, _: Args) -> Result<(), CommandError> {
    message.reply(&format!("Pong! {}", util::use_emoji(None, "DIDNEYWORL")))?;
    Ok(())
}

pub fn roll(_: &mut Context, message: &Message, args: Args) -> Result<(), CommandError> {
    lazy_static! {
        static ref DIE_RE: Regex = Regex::new(r"(\d+)?d(\d+)").expect("Invalid DIE_RE");
    }

    let original = if args.is_empty() { "1d6" } else { args.full() };
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
        message.reply(&format!("{} \u{2192} **{}**", original, result))?;
    } else {
        message.reply(&output)?;
    }
    Ok(())
}

pub fn info(_: &mut Context, message: &Message, _: Args) -> Result<(), CommandError> {
    let avatar = CACHE.read().user.face();
    message.channel_id.send_message(|msg| {
        msg.embed(|e| {
            e.colour(Colour::GOLD)
                .thumbnail(avatar)
                .field("Author", "<@119122043923988483>", false)
                .field(
                    "Source code",
                    "https://gitlab.com/AtteLynx/flutterbitch",
                    false,
                )
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
