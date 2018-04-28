use serenity::framework::standard::{help_commands, StandardFramework};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::CACHE;

use super::commands;
use super::CONFIG;

struct Handler;
impl EventHandler for Handler {
    fn ready(&self, context: Context, _: Ready) {
        let wanted_name: &str = CONFIG.discord.username.as_ref();
        if CACHE.read().user.name != wanted_name {
            if let Err(err) = context.edit_profile(|p| p.username(wanted_name)) {
                warn!("Error settings username: {:?}", err);
            }
        }
        context.set_game(Game::listening(&format!("{}help", CONFIG.discord.command_prefix.as_ref() as &str)));
    }
}

pub fn run_forever() {
    let mut client =
        Client::new(CONFIG.discord.token.as_ref(), Handler).expect("Error making Discord client");

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
                .ungrouped_label("Commands")
        })
        .before(|_context, msg, cmd_name| {
            if let Some(channel) = msg.channel().and_then(|ch| ch.guild()) {
                if let Ok(perms) = channel.read().permissions_for(CACHE.read().user.id) {
                    if perms.contains(Permissions::SEND_MESSAGES) {
                        info!(
                            "Running command {} for @{} ({}) on #{} ({})",
                            cmd_name,
                            msg.author.tag(),
                            msg.author.id,
                            channel.read().name(),
                            msg.channel_id
                        );
                        return true;
                    }
                }
                info!(
                    "Ignored command because couldn't respond on #{} ({}) anyways.",
                    channel.read().name(),
                    msg.channel_id
                );
                false
            } else {
                true
            }
        })
        .after(|_context, _msg, cmd_name, result| {
            trace!("Command {} done", cmd_name);
            if let Err(err) = result {
                error!("Error during command {}: {:?}", cmd_name, err);
            }
        });

    client.with_framework(commands::register(framework));

    if let Err(err) = client.start() {
        error!("An running the client: {}", err);
    }
}
