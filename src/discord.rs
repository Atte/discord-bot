use rand::{self, Rng};
use serenity::framework::standard::{help_commands, DispatchError, StandardFramework};
use serenity::prelude::*;

use super::discord_eventhandler as handler;
use super::util::can_respond_to;
use super::{commands, CONFIG};

pub fn run_forever() {
    ::lazy_static::initialize(&handler::MESSAGE_CACHE);

    let mut client = Client::new(CONFIG.discord.token.as_ref(), handler::Handler)
        .expect("Error making Discord client");

    let framework = StandardFramework::new()
        .configure(|conf| {
            conf.allow_dm(true)
                .allow_whitespace(false)
                .depth(1)
                .ignore_bots(true)
                .ignore_webhooks(true)
                .on_mention(false)
                .owners(CONFIG.discord.owners.clone())
                .prefix(CONFIG.discord.command_prefix.as_ref())
                .case_insensitivity(true)
        })
        .customised_help(help_commands::with_embeds, |help| {
            help.dm_only_text("Only in DM")
                .guild_only_text("Only on channels")
                .dm_and_guilds_text("In DM and on channels")
                .striked_commands_tip(None)
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
        .after(|_context, message, cmd_name, result| {
            trace!("Command {} done", cmd_name);
            if let Err(err) = result {
                error!("Error during command {}: {:?}", cmd_name, err);
                message
                    .reply(&format!(
                        "That's not a valid command! {}",
                        rand::thread_rng()
                            .choose(&CONFIG.bulk.insults)
                            .map_or("", |insult| insult.as_ref())
                    ))
                    .ok();
            }
        })
        .unrecognised_command(|_context, message, _cmd_name| {
            if !can_respond_to(&message) {
                return;
            }
            message
                .reply(&format!(
                    "That's not even a command! {}",
                    rand::thread_rng()
                        .choose(&CONFIG.bulk.insults)
                        .map_or("", |insult| insult.as_ref())
                ))
                .ok();
        })
        .on_dispatch_error(|_context, message, error| {
            if !can_respond_to(&message) {
                return;
            }
            let reason = match error {
                DispatchError::LackOfPermissions(_) => {
                    "You're not good enough to use that command."
                }
                DispatchError::OnlyForGuilds => "That command is only available on a server!",
                DispatchError::NotEnoughArguments { .. } => "That command needs more arguments!",
                DispatchError::TooManyArguments { .. } => {
                    "That command can't take that many arguments!"
                }
                _ => "That's not a valid command!",
            };
            message
                .reply(&format!(
                    "{} {}",
                    reason,
                    rand::thread_rng()
                        .choose(&CONFIG.bulk.insults)
                        .map_or("", |insult| insult.as_ref())
                ))
                .ok();
        });

    client.with_framework(commands::register(framework));

    if let Err(err) = client.start() {
        error!("An running the client: {}", err);
    }
}
