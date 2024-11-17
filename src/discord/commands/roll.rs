use super::super::limits::REPLY_LENGTH;
use crate::util::separate_thousands_floating;
use itertools::Itertools;
use lazy_regex::{regex_is_match, regex_replace_all};
use rand::{distributions::Uniform, thread_rng, Rng};
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    utils::MessageBuilder,
};

#[command]
#[aliases(calc, calculate, calculator)]
#[description("Cast die and/or do math")]
#[usage("1d6 + 2d20")]
#[min_args(1)]
async fn roll(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let original_input = args.message().trim();
    let input = regex_replace_all!(
        r"(?P<rolls>[1-9][0-9]*)?d(?P<sides>[1-9][0-9]*)",
        original_input,
        |_, rolls: &str, sides: &str| {
            let distribution =
                Uniform::new(1_usize, sides.parse::<usize>().unwrap_or(6_usize) + 1_usize);
            let mut rolls =
                (0..std::cmp::min(100_usize, rolls.parse::<usize>().unwrap_or(1_usize)))
                    .map(|_| thread_rng().sample(distribution).to_string());
            format!("({})", rolls.join(" + "))
        },
    );

    match mexprp::eval::<f64>(&input) {
        Ok(result) => {
            let result = separate_thousands_floating(*result.to_vec().first().unwrap());
            let mut response = if regex_is_match!(r"^\(?\d+\)?$", &input) {
                MessageBuilder::new()
                    .push_safe(original_input)
                    .push(" \u{2192} ")
                    .push_bold_safe(&result)
                    .build()
            } else {
                MessageBuilder::new()
                    .push_safe(original_input)
                    .push(" \u{2192} ")
                    .push_safe(input.clone())
                    .push(" = ")
                    .push_bold_safe(&result)
                    .build()
            };
            if response.len() > REPLY_LENGTH || input == original_input {
                response = MessageBuilder::new()
                    .push_safe(original_input)
                    .push(" = ")
                    .push_bold_safe(&result)
                    .build();
            }
            if response.len() > REPLY_LENGTH {
                response = MessageBuilder::new()
                    .push_italic("(input too long to repeat)")
                    .push(" = ")
                    .push_bold_safe(&result)
                    .build();
            }
            msg.reply(ctx, response).await?
        }
        Err(err) => msg.reply(ctx, format!("{err:?}")).await?,
    };
    Ok(())
}
