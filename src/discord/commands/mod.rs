use crate::Result;
use poise::{Command, command, samples::HelpConfiguration};

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

/// Toggle showing "thinking" when generating LLM responses to you (if supported by the current model)
#[cfg(feature = "openai")]
#[command(prefix_command, category = "Misc", track_deletion)]
async fn think(ctx: Context<'_>) -> Result<()> {
    use crate::discord::{DbKey, get_data};
    use bson::{Document, doc};

    let collection = get_data::<DbKey>(ctx.serenity_context())
        .await?
        .collection::<Document>("openai-thinkers");

    let is_thinker = collection
        .find_one(doc! {
            "user.id": ctx.author().id.to_string(),
            "think": true
        })
        .await?
        .is_some();

    if is_thinker {
        collection
            .update_one(
                doc! {
                    "user.id": ctx.author().id.to_string(),
                },
                doc! {
                    "$set": {
                        "think": false
                    }
                },
            )
            .await?;
        ctx.reply("No thoughts. Head empty.").await?;
    } else {
        collection
            .update_one(
                doc! {
                    "user.id": ctx.author().id.to_string(),
                },
                doc! {
                    "$set": {
                        "think": true
                    }
                },
            )
            .upsert(true)
            .await?;
        ctx.reply("<:hmmm:1125492264492335134>").await?;
    }

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
        #[cfg(feature = "openai")]
        think(),
    ]
}
