use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::CACHE;
use std::sync::Arc;

pub fn guild_from_message(msg: &Message) -> Option<Arc<RwLock<Guild>>> {
    if let Some(Channel::Guild(channel)) = msg.channel() {
        channel.read().guild()
    } else {
        None
    }
}

pub fn use_emoji(name: &str) -> String {
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
    String::new()
}
