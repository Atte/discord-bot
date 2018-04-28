use serenity::framework::standard::{help_commands, StandardFramework};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;
use serenity::CACHE;

use super::commands;
use super::CONFIG;

lazy_static! {
    static ref MESSAGE_CACHE: RwLock<Vec<Message>> =
        RwLock::new(Vec::with_capacity(CONFIG.discord.deleted_msg_cache));
}

fn get_log_channels() -> Vec<(ChannelId, GuildId)> {
    CONFIG
        .discord
        .log_channels
        .iter()
        .filter_map(|id| {
            CACHE
                .read()
                .channels
                .get(id)
                .map(|chan| (*id, chan.read().guild_id))
        })
        .collect()
}

struct Handler;
impl EventHandler for Handler {
    fn ready(&self, context: Context, _: Ready) {
        let wanted_name: &str = CONFIG.discord.username.as_ref();
        if CACHE.read().user.name != wanted_name {
            if let Err(err) = context.edit_profile(|p| p.username(wanted_name)) {
                warn!("Error settings username: {:?}", err);
            }
        }
        context.set_game(Game::listening(&format!(
            "{}help",
            CONFIG.discord.command_prefix.as_ref() as &str
        )));
    }

    fn message(&self, _context: Context, message: Message) {
        let mut cache = MESSAGE_CACHE.write();
        cache.insert(0, message);
        cache.truncate(CONFIG.discord.deleted_msg_cache);
    }

    fn message_update(&self, _context: Context, update: MessageUpdateEvent) {
        let mut cache = MESSAGE_CACHE.write();
        if let Some(message) = cache.iter_mut().find(|msg| msg.id == update.id) {
            if let Some(content) = update.content {
                message.content = content;
            }
        }
    }

    fn message_delete(&self, _context: Context, channel_id: ChannelId, message_id: MessageId) {
        if CONFIG.discord.log_channels.contains(&channel_id) {
            return;
        }

        if let Ok(Channel::Guild(channel)) = channel_id.get() {
            let channel = channel.read();
            if let Some(message) = MESSAGE_CACHE.read().iter().find(|msg| msg.id == message_id) {
                for (log_channel, log_guild) in get_log_channels() {
                    if log_guild == channel.guild_id {
                        if let Err(err) = log_channel.send_message(|msg| {
                            msg.embed(|e| {
                                e.colour(Colour::red())
                                    .title(format!("Message deleted in #{}", channel.name))
                                    .description(message.content_safe())
                                    .author(|a| {
                                        a.name(&message.author.tag())
                                            .icon_url(&message.author.face())
                                    })
                                    .timestamp(&message.timestamp)
                            })
                        }) {
                            warn!("Unable to add message deletion to log channel: {:?}", err);
                        }
                    }
                }
            } else {
                info!("Unable to find deleted message in cache!");
            }
        } else {
            warn!("Unable to get channel for deleted message!");
        }
    }
}

pub fn run_forever() {
    ::lazy_static::initialize(&MESSAGE_CACHE);

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
