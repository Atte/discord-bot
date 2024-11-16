use async_openai::types::RunCompletionUsage;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use serenity::all::{ChannelId, GuildId, MessageId, UserId};

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: bson::DateTime,
    pub user: LogEntryUser,
    pub message: LogEntryMessage,
    pub channel: LogEntryChannel,
    pub guild: LogEntryGuild,
    pub responses: Vec<LogEntryMessage>,
    pub errors: Vec<String>,
    pub thread: LogEntryThread,
    pub usage: Option<RunCompletionUsage>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntryUser {
    #[serde_as(as = "DisplayFromStr")]
    pub id: UserId,
    pub name: String,
    pub nick: Option<String>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntryChannel {
    #[serde_as(as = "DisplayFromStr")]
    pub id: ChannelId,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntryGuild {
    #[serde_as(as = "DisplayFromStr")]
    pub id: GuildId,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntryMessage {
    #[serde_as(as = "DisplayFromStr")]
    pub id: MessageId,
    pub length: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntryThread {
    pub id: String,
}
