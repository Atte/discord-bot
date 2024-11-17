use async_openai::types::RunCompletionUsage;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use serenity::all::{ChannelId, Context, GuildId, MessageId, UserId};

use super::MESSAGE_CLEANUP_RE;

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: bson::DateTime,
    pub user: User,
    pub message: Message,
    pub channel: Channel,
    pub guild: Guild,
    pub responses: Vec<Message>,
    pub errors: Vec<String>,
    pub thread: Thread,
    pub usage: Option<RunCompletionUsage>,
}

impl LogEntry {
    pub async fn new(ctx: &Context, msg: &serenity::all::Message, thread_id: String) -> Self {
        Self {
            time: bson::DateTime::now(),
            user: User {
                id: msg.author.id,
                name: msg.author.name.clone(),
                nick: msg.author_nick(ctx).await,
            },
            message: Message {
                id: msg.id,
                length: MESSAGE_CLEANUP_RE.replace_all(&msg.content, "").len(),
            },
            channel: Channel { id: msg.channel_id },
            guild: Guild {
                id: msg.guild_id.unwrap_or_default(),
            },
            responses: Vec::new(),
            errors: Vec::new(),
            thread: Thread { id: thread_id },
            usage: None,
        }
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde_as(as = "DisplayFromStr")]
    pub id: UserId,
    pub name: String,
    pub nick: Option<String>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Channel {
    #[serde_as(as = "DisplayFromStr")]
    pub id: ChannelId,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Guild {
    #[serde_as(as = "DisplayFromStr")]
    pub id: GuildId,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    #[serde_as(as = "DisplayFromStr")]
    pub id: MessageId,
    pub length: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
}
