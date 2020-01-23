use crate::{berrytube::NowPlayingKey, db, util, CONFIG};
use log::{info, warn};
use rand::{self, seq::SliceRandom};
use serenity::{model::prelude::*, prelude::*, utils::Colour};
use std::collections::HashSet;

struct MessageCacheKey;

impl TypeMapKey for MessageCacheKey {
    type Value = Vec<Message>;
}

pub fn read_message_cache<T, F>(context: &Context, f: F) -> T
where
    F: FnOnce(&Vec<Message>) -> T,
{
    let data = context.data.read();
    if let Some(ref cache) = data.get::<MessageCacheKey>() {
        f(cache)
    } else {
        f(&Vec::new())
    }
}

pub fn write_message_cache<T, F>(context: &Context, f: F) -> T
where
    F: FnOnce(&mut Vec<Message>) -> T,
{
    let mut data = context.data.write();
    let mut cache = data.entry::<MessageCacheKey>().or_insert_with(Vec::new);
    f(&mut cache)
}

pub fn get_log_channels(context: &Context, guild_id: GuildId) -> Vec<ChannelId> {
    CONFIG
        .discord
        .log_channels
        .iter()
        .filter_map(|id| {
            if id
                .to_channel(context)
                .ok()
                .and_then(Channel::guild)
                .map_or(false, |guild| guild.read().guild_id == guild_id)
            {
                Some(*id)
            } else {
                None
            }
        })
        .collect()
}

pub struct Handler;

impl EventHandler for Handler {
    fn ready(&self, context: Context, _: Ready) {
        if let Some(nowplaying) = context.data.read().get::<NowPlayingKey>() {
            context.set_presence(Some(Activity::playing(nowplaying)), OnlineStatus::Online);
        } else {
            context.set_presence(
                Some(Activity::listening(&format!(
                    "{}help",
                    CONFIG.discord.command_prefix.as_ref() as &str
                ))),
                OnlineStatus::Online,
            );
        }
    }

    fn guild_create(&self, context: Context, guild: Guild, _is_new: bool) {
        for member in guild.members.values() {
            if guild
                .presences
                .get(&member.user.read().id)
                .map_or(false, |presence| presence.status != OnlineStatus::Offline)
            {
                let _ = db::with_db(&context, |conn| db::member_online(&conn, &member));
            }
        }
    }

    fn message(&self, context: Context, message: Message) {
        let _ = db::with_db(&context, |conn| {
            db::user_online(&conn, &message.author)?;
            db::user_message(&conn, message.author.id)
        });

        let uid = context.cache.read().user.id;
        if util::can_respond_to(&message) && message.mentions.iter().any(|user| user.id == uid) {
            if let Some(insult) = CONFIG.bulk.insults.choose(&mut rand::thread_rng()) {
                message.reply(&context, insult).ok();
            }
        }

        write_message_cache(&context, move |cache| {
            cache.insert(0, message);
            cache.truncate(CONFIG.discord.deleted_msg_cache);
        });
    }

    fn message_update(
        &self,
        context: Context,
        _old: Option<Message>,
        _new: Option<Message>,
        update: MessageUpdateEvent,
    ) {
        write_message_cache(&context, |cache| {
            if let Some(message) = cache.iter_mut().find(|msg| msg.id == update.id) {
                // TODO: update embeds
                if let Some(content) = update.content {
                    message.content = content;
                }
                if let Some(attachments) = update.attachments {
                    message.attachments = attachments;
                }
                if let Some(edited_timestamp) = update.edited_timestamp {
                    message.edited_timestamp = Some(edited_timestamp);
                }
            }
        });
    }

    fn message_delete(&self, context: Context, channel_id: ChannelId, message_id: MessageId) {
        if CONFIG.discord.log_channels.contains(&channel_id) {
            return;
        }

        if let Ok(Channel::Guild(channel)) = channel_id.to_channel(&context) {
            let channel = channel.read();
            if let Some(message) = read_message_cache(&context, |cache| {
                cache.iter().find(|msg| msg.id == message_id).cloned()
            }) {
                for log_channel in get_log_channels(&context, channel.guild_id) {
                    if let Err(err) = log_channel.send_message(&context, |msg| {
                        msg.embed(|mut e| {
                            if let Some(embed) = message.embeds.iter().next() {
                                if let Some(ref thumb) = embed.thumbnail {
                                    e = e.thumbnail(&thumb.proxy_url);
                                }
                                if let Some(ref image) = embed.image {
                                    e = e.image(&image.proxy_url);
                                }
                            } else if let Some(attach) = message.attachments.iter().next() {
                                e = e.image(&attach.proxy_url);
                            }
                            e.colour(Colour::RED)
                                .description(format!(
                                    "**Message sent by <@{}> deleted in <#{}>**\n{}",
                                    message.author.id,
                                    channel_id,
                                    message.content_safe(&context)
                                ))
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
            } else {
                info!("Unable to find deleted message in cache!");
            }
        } else {
            warn!("Unable to get channel for deleted message!");
        }
    }

    fn guild_member_addition(&self, context: Context, guild_id: GuildId, mut member: Member) {
        let _ = db::with_db(&context, |conn| db::member_online(&conn, &member));

        for log_channel in get_log_channels(&context, guild_id) {
            if let Err(err) = log_channel.send_message(&context, |msg| {
                let user = member.user.read();
                msg.embed(|e| {
                    e.colour(Colour::FOOYOO)
                        .description(format!("**<@{}> joined**", user.id))
                        .author(|a| a.name(&user.tag()).icon_url(&user.face()))
                })
            }) {
                warn!("Unable to add member join to log channel: {:?}", err);
            }
        }

        if let Ok(roles) = db::with_db(&context, |conn| {
            db::get_sticky_roles(&conn, member.user.read().id)
        }) {
            for role in roles {
                if let Err(err) = member.add_role(&context, role) {
                    warn!("Unable to restore a sticky role: {:?}", err);
                }
            }
        }
    }

    fn guild_member_removal(
        &self,
        context: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        for log_channel in get_log_channels(&context, guild_id) {
            if let Err(err) = log_channel.send_message(&context, |msg| {
                msg.embed(|e| {
                    e.colour(Colour::RED)
                        .description(format!("**<@{}> left**", user.id))
                        .author(|a| a.name(&user.tag()).icon_url(&user.face()))
                })
            }) {
                warn!("Unable to add member leave to log channel: {:?}", err);
            }
        }
    }

    fn guild_member_update(
        &self,
        context: Context,
        old_member: Option<Member>,
        new_member: Member,
    ) {
        let _ = db::with_db(&context, |conn| db::member_online(&conn, &new_member));

        let new_user = new_member.user.read();
        let new_nick = new_member.nick.unwrap_or_else(|| new_user.name.clone());
        let sticky_roles: HashSet<RoleId> = new_member
            .roles
            .into_iter()
            .filter(|id| CONFIG.discord.sticky_roles.contains(id))
            .collect();

        let _ = db::with_db(&context, |conn| {
            db::set_sticky_roles(&conn, new_user.id, sticky_roles)
        });

        if let Some(old_member) = old_member {
            let old_user = old_member.user.read();
            let old_nick = old_member.nick.unwrap_or_else(|| old_user.name.clone());

            if new_nick != old_nick {
                for log_channel in get_log_channels(&context, old_member.guild_id) {
                    if let Err(err) = log_channel.send_message(&context, |msg| {
                        msg.embed(|e| {
                            e.colour(Colour::RED)
                                .description(format!(
                                    "**<@{}> changed their nick**\n{} \u{2192} {}",
                                    new_user.id, old_nick, new_nick
                                ))
                                .author(|a| a.name(&new_user.tag()).icon_url(&new_user.face()))
                        })
                    }) {
                        warn!("Unable to add nick change to log channel: {:?}", err);
                    }
                }
            }
        }
    }

    fn presence_update(&self, context: Context, update: PresenceUpdateEvent) {
        if let Some(user) = update.presence.user {
            let _ = db::with_db(&context, |conn| db::user_online(&conn, &user.read()));
        }
    }
}
