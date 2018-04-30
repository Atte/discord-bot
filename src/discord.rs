use rand::{self, Rng};
use serenity::framework::standard::{help_commands, StandardFramework};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::CACHE;

use super::discord_eventhandler as handler;
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
            if let Some(channel) = message.channel().and_then(|ch| ch.guild()) {
                if let Ok(perms) = channel.read().permissions_for(CACHE.read().user.id) {
                    if perms.contains(Permissions::SEND_MESSAGES) {
                        info!(
                            "Running command {} for @{} ({}) on #{} ({})",
                            cmd_name,
                            message.author.tag(),
                            message.author.id,
                            channel.read().name(),
                            message.channel_id
                        );
                        return true;
                    }
                }
                info!(
                    "Ignored command because couldn't respond on #{} ({}) anyways.",
                    channel.read().name(),
                    message.channel_id
                );
                false
            } else {
                true
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
        });

    client.with_framework(commands::register(framework));

    if let Err(err) = client.start() {
        error!("An running the client: {}", err);
    }
}
