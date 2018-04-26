use serenity::framework::standard::{help_commands, StandardFramework};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::CACHE;

use super::commands;
use super::CONFIG;

struct Handler;
impl EventHandler for Handler {}

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
                .prefix("!")
                .case_insensitivity(true)
        })
        .customised_help(help_commands::with_embeds, |help| {
            help.striked_commands_tip(Some("Some commands are only available to mods.".to_owned()))
                .dm_only_text("Only in DM")
                .guild_only_text("Only on channels")
                .dm_and_guilds_text("In DM and on channels")
                .ungrouped_label("Commands")
        })
        .before(|_context, msg, cmd| {
            if let Some(channel) = msg.channel().and_then(|ch| ch.guild()) {
                if let Ok(perms) = channel.read().permissions_for(CACHE.read().user.id) {
                    if perms.contains(Permissions::SEND_MESSAGES) {
                        info!(
                            "Running command {} for @{}#{} ({}) on #{} ({})",
                            cmd,
                            msg.author.name,
                            msg.author.discriminator,
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
        });

    client.with_framework(commands::register(framework));

    if let Err(err) = client.start() {
        error!("An running the client: {}", err);
    }
}
