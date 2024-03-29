use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use serenity::all::{Context, Message, User};
use tokio::sync::Mutex;

use super::{get_data, ConfigKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundPhase {
    Idle,
    Starting,
    Running,
}

#[derive(Debug)]
pub struct RoundState {
    pub phase: RoundPhase,
}

impl RoundState {
    #[inline]
    pub const fn new() -> Self {
        Self {
            phase: RoundPhase::Idle,
        }
    }
}

pub static STATE: Mutex<RoundState> = Mutex::const_new(RoundState::new());

#[derive(Debug, Serialize, Deserialize)]
struct ApiUser {
    id: String,
    name: Option<String>,
}

impl From<&User> for ApiUser {
    #[inline]
    fn from(value: &User) -> Self {
        Self {
            id: value.id.to_string(),
            name: Some(value.name.clone()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum ApiRequest {
    Message { user: ApiUser, text: String },
    RoundStart,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
#[serde(rename_all = "snake_case")]
enum ApiResponse {
    Eliminate { user: ApiUser },
}

pub async fn message(ctx: &Context, message: &Message) -> Result<()> {
    let request = ApiRequest::Message {
        user: (&message.author).into(),
        text: message.content_safe(ctx),
    };

    // TODO: call API
    let response = ApiResponse::Eliminate {
        user: ApiUser {
            id: "1234".to_owned(),
            name: None,
        },
    };

    match response {
        ApiResponse::Eliminate { user } => {
            let config = get_data::<ConfigKey>(ctx).await?;
            if let Some(guild_id) = message.guild(&ctx.cache).map(|guild| guild.id) {
                let member = guild_id.member(ctx, user.id.parse::<u64>()?).await?;
                member
                    .remove_role(ctx, config.discord.april2024.role)
                    .await?;
            }
        }
    }

    Ok(())
}
