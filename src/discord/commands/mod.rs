use crate::Result;
use poise::{command, samples::HelpConfiguration, Command};

mod gib;
mod ranks;
mod roll;

use super::{Context, PoiseData};

/// pong
#[command(prefix_command, category = "Misc", track_deletion)]
async fn ping(ctx: Context<'_>) -> Result<()> {
    ctx.reply("Pong!").await?;
    Ok(())
}

/// List commands, or show help for a specific command
#[command(prefix_command, category = "Misc", invoke_on_edit, track_deletion)]
async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<()> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        HelpConfiguration {
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

pub(super) fn get_all() -> Vec<Command<PoiseData, crate::Error>> {
    vec![
        gib::gib(),
        ranks::join(),
        ranks::leave(),
        ranks::rank(),
        ranks::ranks(),
        roll::roll(),
        ping(),
        help(),
    ]
}
