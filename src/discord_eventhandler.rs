use crate::{berrytube::NowPlayingKey, db, util, CONFIG};
use log::{info, warn};
use rand::{self, seq::SliceRandom};
use serenity::{model::prelude::*, prelude::*, utils::Colour};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

const READ_TIMEOUT: Duration = Duration::from_secs(3);

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
        if let Some(data) = context.data.try_read_for(READ_TIMEOUT) {
            if let Some(nowplaying) = data.get::<NowPlayingKey>() {
                context.set_presence(Some(Activity::playing(nowplaying)), OnlineStatus::Online);
                return;
            }
        }

        context.set_presence(
            Some(Activity::listening(&format!(
                "{}help",
                CONFIG.discord.command_prefix.as_ref() as &str
            ))),
            OnlineStatus::Online,
        );
    }

    fn guild_create(&self, _context: Context, guild: Guild, _is_new: bool) {
        for channel in guild.channels.values() {
            if let Some(channel) = channel.try_read_for(READ_TIMEOUT) {
                let _ = db::with_db(|conn| db::channel_exists(&conn, &channel));
            }
        }

        for member in guild.members.values() {
            if let Some(user) = member.user.try_read_for(READ_TIMEOUT) {
                if guild
                    .presences
                    .get(&user.id)
                    .map_or(false, |presence| presence.status != OnlineStatus::Offline)
                {
                    let _ = db::with_db(|conn| db::member_online(&conn, &user, &member));
                } else {
                    let _ = db::with_db(|conn| db::member_offline(&conn, &user, &member));
                }
            }
        }
    }

    fn channel_create(&self, _context: Context, channel: Arc<RwLock<GuildChannel>>) {
        if let Some(channel) = channel.try_read_for(READ_TIMEOUT) {
            let _ = db::with_db(|conn| db::channel_exists(&conn, &channel));
        }
    }

    fn guild_members_chunk(
        &self,
        _context: Context,
        _guild_id: GuildId,
        offline_members: HashMap<UserId, Member>,
    ) {
        for member in offline_members.values() {
            if let Some(user) = member.user.try_read_for(READ_TIMEOUT) {
                let _ = db::with_db(|conn| db::member_offline(&conn, &user, &member));
            }
        }
    }

    fn message(&self, context: Context, message: Message) {
        let _ = db::with_db(|conn| {
            db::user_online(&conn, &message.author)?;
            db::user_message(&conn, message.author.id)?;
            db::cache_message(&conn, &message)
        });

        if let Some(uid) = context
            .cache
            .try_read_for(READ_TIMEOUT)
            .map(|cache| cache.user.id)
        {
            if util::can_respond_to(&message) && message.mentions.iter().any(|user| user.id == uid)
            {
                if let Some(insult) = CONFIG.bulk.insults.choose(&mut rand::thread_rng()) {
                    message.reply(&context, insult).ok();
                }
            }
        }
    }

    fn message_update(
        &self,
        _context: Context,
        _old: Option<Message>,
        new: Option<Message>,
        _update: MessageUpdateEvent,
    ) {
        if let Some(msg) = new {
            let _ = db::with_db(|conn| db::cache_message(&conn, &msg));
        }
    }

    fn message_delete(&self, context: Context, channel_id: ChannelId, message_id: MessageId) {
        if CONFIG.discord.log_channels.contains(&channel_id) {
            return;
        }

        if let Ok(Channel::Guild(channel)) = channel_id.to_channel(&context) {
            if let Ok(Some(message)) = db::with_db(|conn| db::get_message(&conn, message_id)) {
                for log_channel in get_log_channels(&context, channel.read().guild_id) {
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
        if let Some(user) = member.user.try_read_for(READ_TIMEOUT) {
            let _ = db::with_db(|conn| db::member_online(&conn, &user, &member));
        }

        for log_channel in get_log_channels(&context, guild_id) {
            if let Some(user) = member.user.try_read_for(READ_TIMEOUT) {
                if let Err(err) = log_channel.send_message(&context, |msg| {
                    msg.embed(|e| {
                        e.colour(Colour::FOOYOO)
                            .description(format!("**<@{}> joined**", user.id))
                            .author(|a| a.name(&user.tag()).icon_url(&user.face()))
                    })
                }) {
                    warn!("Unable to add member join to log channel: {:?}", err);
                }
            }
        }

        if let Ok(roles) = db::with_db(|conn| {
            // mandatory lock for sticky role restoration
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
        // mandatory lock for sticky role updates
        let new_user = new_member.user.read();
        let _ = db::with_db(|conn| db::member_online(&conn, &new_user, &new_member));

        let new_nick = new_member.nick.unwrap_or_else(|| new_user.name.clone());
        let sticky_roles: HashSet<RoleId> = new_member
            .roles
            .into_iter()
            .filter(|id| CONFIG.discord.sticky_roles.contains(id))
            .collect();

        let _ = db::with_db(|conn| db::set_sticky_roles(&conn, new_user.id, sticky_roles));

        if let Some(old_member) = old_member {
            if let Some(old_user) = old_member.user.try_read_for(READ_TIMEOUT) {
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
    }

    fn presence_update(&self, _context: Context, update: PresenceUpdateEvent) {
        if let Some(user) = update.presence.user {
            if let Some(user) = user.try_read_for(READ_TIMEOUT) {
                let _ = db::with_db(|conn| db::user_online(&conn, &user));
            }
        }
    }
}
