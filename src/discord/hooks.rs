use super::{get_data, stats::update_stats, ConfigKey};
use crate::util::format_duration_long;
use log::{error, warn};
use serenity::{
    client::Context,
    framework::standard::{macros::hook, CommandError, DispatchError},
    model::channel::Message,
};

#[hook]
pub async fn normal_message(ctx: &Context, msg: &Message) {
    if let Err(err) = update_stats(ctx, msg).await {
        error!("Error in update_stats for normal_message: {:?}", err);
    }
}

#[hook]
pub async fn unrecognised_command(ctx: &Context, msg: &Message, _command_name: &str) {
    if let Ok(config) = get_data::<ConfigKey>(ctx).await {
        if config.discord.command_channels.contains(&msg.channel_id) {
            let _result = msg.reply(&ctx, "That's not a command!").await;
        }
    }
}

#[hook]
pub async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError, command: &str) {
    match error {
        DispatchError::CheckFailed(desc, reason) => {
            warn!("Custom check failed: {} ({:?})", desc, reason);
        }
        DispatchError::Ratelimited(wait) => {
            let _result = msg
                .reply(
                    &ctx,
                    format!(
                        "Don't spam! Wait {} before trying again.",
                        format_duration_long(&wait.rate_limit)
                    ),
                )
                .await;
        }
        DispatchError::CommandDisabled => {
            warn!("Refused to dispatch disabled command: {}", command);
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
            let _result = msg
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
            let _result = msg
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
        err => {
            error!("Dispatch error: {:?}", err);
        }
    }
}

#[hook]
pub async fn after(ctx: &Context, msg: &Message, command: &str, error: Result<(), CommandError>) {
    if let Err(err) = error {
        println!("Error during {command}: {err:?}");
        let _result = msg.reply(&ctx, "Something went horribly wrong!").await;
    }
}
