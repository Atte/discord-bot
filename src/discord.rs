use crate::{commands, discord_eventhandler as handler, util::can_respond_to, CONFIG};
use log::{error, info, trace};
use rand::{self, seq::SliceRandom};
use serenity::{
    framework::standard::{DispatchError, StandardFramework},
    prelude::*,
};

#[allow(clippy::too_many_lines)]
pub fn create_client() -> Client {
    ::lazy_static::initialize(&handler::MESSAGE_CACHE);

    let framework = StandardFramework::new()
        .bucket("derp", |b| b.delay(10).time_span(10).limit(10))
        .group(&commands::HORSE_GROUP)
        .group(&commands::DISCORD_GROUP)
        .group(&commands::MISC_GROUP)
        .help(&commands::HELP)
        .configure(|conf| {
            conf.owners(CONFIG.discord.owners.clone())
                .prefix(CONFIG.discord.command_prefix.as_ref())
        })
        .before(|_context, message, cmd_name| {
            if commands::is_allowed(message, cmd_name) {
                info!(
                    "Running command {} for @{} ({})",
                    cmd_name,
                    message.author.tag(),
                    message.author.id,
                );
                true
            } else {
                false
            }
        })
        .after(|context, message, cmd_name, result| {
            trace!("Command {} done", cmd_name);
            if let Err(err) = result {
                error!("Error during command {}: {:?}", cmd_name, err);
                message
                    .reply(
                        &context,
                        &format!(
                            "That's not a valid command! {}",
                            CONFIG
                                .bulk
                                .insults
                                .choose(&mut rand::thread_rng())
                                .map_or("", |insult| insult.as_ref())
                        ),
                    )
                    .ok();
            }
        })
        .unrecognised_command(|context, message, _cmd_name| {
            if !can_respond_to(&message) {
                return;
            }
            message
                .reply(
                    &context,
                    &format!(
                        "That's not even a command! {}",
                        CONFIG
                            .bulk
                            .insults
                            .choose(&mut rand::thread_rng())
                            .map_or("", |insult| insult.as_ref())
                    ),
                )
                .ok();
        })
        .on_dispatch_error(|context, message, error| {
            if !can_respond_to(&message) {
                return;
            }
            let reason = match error {
                DispatchError::LackingPermissions(_)
                | DispatchError::LackingRole
                | DispatchError::OnlyForOwners => {
                    "You're not good enough to use that command.".to_owned()
                }
                DispatchError::OnlyForGuilds => {
                    "That command is only available on a server!".to_owned()
                }
                DispatchError::OnlyForDM => "That command is only available in a DM!".to_owned(),
                DispatchError::NotEnoughArguments { min, .. } => format!(
                    "That command needs at least {} argument{}!",
                    min,
                    if min == 1 { "" } else { "s" }
                ),
                DispatchError::TooManyArguments { max, .. } => format!(
                    "That command can take at most {} argument{}!",
                    max,
                    if max == 1 { "" } else { "s" }
                ),
                DispatchError::CheckFailed(msg, _) => msg.to_owned(),
                DispatchError::CommandDisabled(_) => "That command is disabled!".to_owned(),
                DispatchError::Ratelimited(secs) => format!(
                    "Don't spam! Try again in {} second{}...",
                    secs,
                    if secs == 1 { "" } else { "s" }
                ),
                DispatchError::BlockedUser
                | DispatchError::BlockedGuild
                | DispatchError::BlockedChannel
                | DispatchError::IgnoredBot
                | DispatchError::WebhookAuthor => return,
                _ => "An impossible error happened!".to_owned(),
            };
            message
                .reply(
                    &context,
                    &format!(
                        "{} {}",
                        reason,
                        CONFIG
                            .bulk
                            .insults
                            .choose(&mut rand::thread_rng())
                            .map_or("", |insult| insult.as_ref())
                    ),
                )
                .ok();
        });

    let mut client =
        Client::new(&CONFIG.discord.token, handler::Handler).expect("Error making Discord client");
    client.with_framework(framework);
    client
}
