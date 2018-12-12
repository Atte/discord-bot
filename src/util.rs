use crate::CONFIG;
use serenity::{model::prelude::*, prelude::*, CACHE};
use std::sync::Arc;

pub fn uid() -> UserId {
    CACHE.read().user.id
}

pub fn guild_from_message(msg: &Message) -> Option<Arc<RwLock<Guild>>> {
    if let Some(Channel::Guild(channel)) = msg.channel() {
        channel.read().guild()
    } else {
        None
    }
}

pub fn use_emoji(guild: Option<&Guild>, name: &str) -> String {
    if let Some(guild) = guild {
        if let Some(emoji) = guild.emojis.values().find(|emoji| emoji.name == name) {
            return emoji.to_string();
        }
    } else {
        for guild in CACHE.read().guilds.values() {
            if let Some(emoji) = guild
                .read()
                .emojis
                .values()
                .find(|emoji| emoji.name == name)
            {
                return emoji.to_string();
            }
        }
    }
    String::new()
}

pub fn can_talk_in(channel: &GuildChannel) -> bool {
    channel
        .permissions_for(uid())
        .ok()
        .map_or(true, |perms| perms.contains(Permissions::SEND_MESSAGES))
        && !CONFIG.discord.channel_blacklist.contains(&channel.id)
        && (CONFIG.discord.channel_whitelist.is_empty()
            || CONFIG.discord.channel_whitelist.contains(&channel.id))
}

pub fn can_respond_to(message: &Message) -> bool {
    if let Some(channel) = message.channel().and_then(|ch| ch.guild()) {
        can_talk_in(&channel.read())
    } else {
        true
    }
}
