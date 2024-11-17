use super::super::limits::REPLY_LENGTH;
use crate::{discord::Context, util::separate_thousands_floating, Result};
use itertools::Itertools;
use lazy_regex::{regex_is_match, regex_replace_all};
use poise::command;
use rand::{distributions::Uniform, thread_rng, Rng};
use serenity::utils::MessageBuilder;

/// Cast die and/or do math
///
/// Example: 1d6 + 2d20 - 3
#[command(
    prefix_command,
    category = "Misc",
    aliases("calc", "calculate", "calculator")
)]
pub async fn roll(ctx: Context<'_>, #[rest] expression: String) -> Result<()> {
    let expression = expression.trim();

    let input = regex_replace_all!(
        r"(?P<rolls>[1-9][0-9]*)?d(?P<sides>[1-9][0-9]*)",
        expression,
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
                    .push_safe(expression)
                    .push(" \u{2192} ")
                    .push_bold_safe(&result)
                    .build()
            } else {
                MessageBuilder::new()
                    .push_safe(expression)
                    .push(" \u{2192} ")
                    .push_safe(input.clone())
                    .push(" = ")
                    .push_bold_safe(&result)
                    .build()
            };
            if response.len() > REPLY_LENGTH || input == expression {
                response = MessageBuilder::new()
                    .push_safe(expression)
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
            ctx.reply(response).await?
        }
        Err(err) => ctx.reply(format!("{err:?}")).await?,
    };
    Ok(())
}
