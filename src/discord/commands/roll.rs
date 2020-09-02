use super::super::limits::REPLY_LENGTH;
use itertools::Itertools;
use lazy_static::lazy_static;
use rand::{distributions::Uniform, thread_rng, Rng};
use regex::{Captures, Regex};
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    utils::MessageBuilder,
};

#[command]
#[aliases(calc)]
#[description("Cast die and/or do math")]
#[usage("1d6 + 2d20")]
#[min_args(1)]
async fn roll(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"(?P<rolls>[1-9][0-9]*)?d(?P<sides>[1-9][0-9]*)"#).unwrap();
    }

    let original_input = args.message().trim();
    let input = RE.replace_all(&original_input, |caps: &Captures| {
        let distribution = Uniform::new(
            1_usize,
            caps.name("sides")
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(6_usize),
        );
        let mut rolls = (0..caps
            .name("rolls")
            .and_then(|m| m.as_str().parse::<usize>().ok())
            .unwrap_or(1_usize))
            .map(|_| thread_rng().sample(distribution).to_string());
        format!("({})", rolls.join(" + "))
    });

    match meval::eval_str(&input) {
        Ok(result) => {
            let mut response = MessageBuilder::new()
                .push_safe(original_input)
                .push(" \u{2192} ")
                .push_safe(&input)
                .push(" = ")
                .push_bold_safe(result)
                .build();
            if response.len() > REPLY_LENGTH || input == original_input {
                response = MessageBuilder::new()
                    .push_safe(original_input)
                    .push(" = ")
                    .push_bold_safe(result)
                    .build();
            }
            if response.len() > REPLY_LENGTH {
                response = MessageBuilder::new()
                    .push_italic("(input too long to repeat)")
                    .push(" = ")
                    .push_bold_safe(result)
                    .build();
            }
            msg.reply(ctx, response).await?
        }
        Err(err) => msg.reply(ctx, format!("{}", err)).await?,
    };
    Ok(())
}
