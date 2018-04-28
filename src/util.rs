use serenity::model::prelude::*;
use serenity::prelude::*;
use std::sync::Arc;

pub fn guild_from_message(msg: &Message) -> Option<Arc<RwLock<Guild>>> {
    if let Some(Channel::Guild(channel)) = msg.channel() {
        channel.read().guild()
    } else {
        None
    }
}
