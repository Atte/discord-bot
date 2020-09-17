use super::stats::update_stats;
use crate::util::format_duration;
use log::{error, warn};
use serenity::{
    client::Context,
    framework::standard::{macros::hook, DispatchError},
    model::channel::Message,
};

#[hook]
pub async fn normal_message(ctx: &Context, msg: &Message) {
    if let Err(err) = update_stats(&ctx, &msg).await {
        error!("Error in update_stats for normal_message: {:?}", err);
    }
}

#[hook]
pub async fn unrecognised_command(ctx: &Context, msg: &Message, _command_name: &str) {
    let _ = msg.reply(&ctx, "That's not a command!").await;
}

#[hook]
pub async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::CheckFailed(desc, reason) => {
            warn!("Custom check failed: {} ({:?})", desc, reason);
        }
        DispatchError::Ratelimited(wait) => {
            let _ = msg
                .reply(
                    &ctx,
                    format!(
                        "Ratelimited! Wait {} before trying again.",
                        format_duration(&wait)
                    ),
                )
                .await;
        }
        DispatchError::CommandDisabled(something) => {
            warn!("Refused to dispatch disabled command: {}", something);
        }
        DispatchError::BlockedUser => {
            warn!("Refused to dispatch for blocked user");
        }
        DispatchError::BlockedGuild => {
            warn!("Refused to dispatch for blocked guild");
        }
        DispatchError::BlockedChannel => {
            warn!("Refused to dispatch for blocked channel");
        }
        DispatchError::OnlyForDM => {
            warn!("Refused to dispatch command that's only for DMs");
        }
        DispatchError::OnlyForGuilds => {
            warn!("Refused to dispatch command that's only for guilds");
        }
        DispatchError::OnlyForOwners => {
            warn!("Refused to dispatch command that's only for owners");
        }
        DispatchError::LackingRole => {
            warn!("Refused to dispatch command due to lacking role");
        }
        DispatchError::LackingPermissions(perms) => {
            warn!(
                "Refused to dispatch command due to lacking permissions: {:?}",
                perms
            );
        }
        DispatchError::NotEnoughArguments { min, given } => {
            let _ = msg
                .reply(
                    &ctx,
                    format!(
                        "Need at least {} argument{}, got {}",
                        min,
                        if min == 1 { "" } else { "s" },
                        given
                    ),
                )
                .await;
        }
        DispatchError::TooManyArguments { max, given } => {
            let _ = msg
                .reply(
                    &ctx,
                    format!(
                        "At most {} argument{} allowed, got {}",
                        max,
                        if max == 1 { "" } else { "s" },
                        given
                    ),
                )
                .await;
        }
        DispatchError::IgnoredBot => {
            warn!("Ignored command dispatch for bot");
        }
        DispatchError::WebhookAuthor => {
            warn!("Ignored command dispatch for webhook");
        }
        err => {
            error!("Dispatch error: {:?}", err);
        }
    }
}
