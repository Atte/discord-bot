use serenity::framework::standard::StandardFramework;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::CACHE;
use std::env;

use super::commands;

struct Handler;
impl EventHandler for Handler {}

pub fn run_forever() {
    let mut client = Client::new(
        &env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN missing from env"),
        Handler,
    ).expect("Error creating client");

    let framework = StandardFramework::new()
        .configure(|conf| {
            conf.allow_dm(false)
                .allow_whitespace(false)
                .depth(1)
                .ignore_bots(true)
                .ignore_webhooks(true)
                .on_mention(false)
                .prefix("!")
                .case_insensitivity(true)
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
            } else {
                warn!("Ignored command on non-guild channel ({}).", msg.channel_id);
            }
            false
        });

    client.with_framework(commands::register(framework));

    if let Err(err) = client.start() {
        error!("An running the client: {}", err);
    }
}
