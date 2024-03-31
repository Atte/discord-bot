use async_recursion::async_recursion;
use color_eyre::eyre::{OptionExt, Result};
use futures::StreamExt;
use itertools::Itertools;
use log::warn;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serenity::all::{
    Context, CreateAllowedMentions, CreateMessage, Member, Message, MessageBuilder, PartialMember,
    User, UserId,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tokio::sync::{mpsc::Sender, Mutex};
use tokio::task::JoinHandle;

use super::{get_data, ConfigKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundPhase {
    Inactive,
    Active,
}

#[derive(Debug)]
pub struct RoundState {
    pub phase: RoundPhase,
    pub tx_end: Option<Sender<()>>,
    players: Vec<PlayerState>,
    requests: Option<Sender<(ApiRequest, oneshot::Sender<Vec<ApiResponse>>)>>,
    request_task: Option<JoinHandle<()>>,
}

impl RoundState {
    #[inline]
    pub const fn new() -> Self {
        Self {
            phase: RoundPhase::Inactive,
            tx_end: None,
            players: Vec::new(),
            requests: None,
            request_task: None,
        }
    }
}

#[derive(Debug)]
struct PlayerState {
    pub member: Member,
    pub last_message: Instant,
}

impl PlayerState {
    #[inline]
    pub fn new(member: Member) -> Self {
        Self {
            member,
            last_message: Instant::now(),
        }
    }
}

pub static STATE: Mutex<RoundState> = Mutex::const_new(RoundState::new());

#[derive(Debug, Serialize, Deserialize)]
struct ApiUser {
    id: String,
    username: Option<String>,
    display_name: Option<String>,
}

impl From<&User> for ApiUser {
    #[inline]
    fn from(user: &User) -> Self {
        Self {
            id: user.id.to_string(),
            username: Some(user.name.clone()),
            display_name: None,
        }
    }
}

impl From<&Member> for ApiUser {
    #[inline]
    fn from(member: &Member) -> Self {
        Self {
            id: member.user.id.to_string(),
            username: Some(member.user.name.clone()),
            display_name: Some(member.display_name().to_owned()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum ApiRequest {
    Message { user: ApiUser, text: String },
    Eliminated { user: ApiUser },
    RoundStart { users: Vec<ApiUser> },
    RoundEnd,
    AddRule,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
#[serde(rename_all = "snake_case")]
enum ApiResponse {
    Eliminate {
        user: ApiUser,
        reason: Option<String>,
    },
    Announce {
        text: String,
        #[serde(rename = "@here")]
        #[serde(default)]
        here: bool,
    },
}

async fn announce(ctx: &Context, message: CreateMessage) -> Result<()> {
    let config = get_data::<ConfigKey>(ctx).await?;
    config
        .april2024
        .arena_channel
        .send_message(ctx, message)
        .await?;
    Ok(())
}

pub async fn message(ctx: &Context, message: &Message) -> Result<()> {
    let member = message.member(ctx).await?;
    api(
        ctx,
        ApiRequest::Message {
            user: (&member).into(),
            text: message.content_safe(ctx),
        },
    )
    .await
}

async fn request_task(
    url: Option<Url>,
    mut rx: Receiver<(ApiRequest, oneshot::Sender<Vec<ApiResponse>>)>,
) {
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("failed to build API client");
    while let Some((request, tx)) = rx.recv().await {
        if let Some(ref url) = url {
            for _ in 0..10 {
                match client.post(url.clone()).json(&request).send().await {
                    Ok(response) => match response.json::<Vec<ApiResponse>>().await {
                        Ok(response) => {
                            let _ = tx.send(response);
                            break;
                        }
                        Err(err) => {
                            warn!("request_task parse: {err:?}");
                        }
                    },
                    Err(err) => {
                        warn!("request_task send: {err:?}");
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tx.send(Vec::new());
        }
    }
}

#[async_recursion]
async fn api(ctx: &Context, request: ApiRequest) -> Result<()> {
    let config = get_data::<ConfigKey>(ctx).await?;
    if config.april2024.debug {
        announce(
            ctx,
            CreateMessage::new().content(
                MessageBuilder::new()
                    .push_codeblock_safe(serde_json::to_string_pretty(&request)?, Some("json"))
                    .build(),
            ),
        )
        .await?;
    }

    let response = {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Some(ref requests_tx) = STATE.lock().await.requests {
            requests_tx.send((request, tx)).await?;
        }
        rx.await?
    };

    if config.april2024.debug {
        announce(
            ctx,
            CreateMessage::new().content(
                MessageBuilder::new()
                    .push_codeblock_safe(serde_json::to_string_pretty(&response)?, Some("json"))
                    .build(),
            ),
        )
        .await?;
    }

    let mut eliminations: HashMap<Option<String>, Vec<UserId>> = HashMap::new();
    for action in response {
        match action {
            ApiResponse::Eliminate { user, reason } => {
                if let Ok(id) = user.id.parse::<u64>() {
                    eliminations
                        .entry(reason)
                        .or_default()
                        .push(UserId::new(id));
                }
            }
            ApiResponse::Announce { text, here } => {
                if here {
                    announce(
                        ctx,
                        CreateMessage::new()
                            .allowed_mentions(CreateAllowedMentions::new().everyone(true))
                            .content(format!("@here {text}")),
                    )
                    .await?;
                } else {
                    announce(ctx, CreateMessage::new().content(text)).await?;
                }
            }
        }
    }

    for (reason, user_ids) in eliminations {
        eliminate(
            ctx,
            &user_ids,
            reason.unwrap_or_else(|| "They broke the rules.".to_owned()),
        )
        .await?;
    }

    Ok(())
}

pub async fn start_round(ctx: &Context) -> Result<()> {
    let players: Vec<ApiUser> = {
        let config = get_data::<ConfigKey>(ctx).await?;

        let mut state = STATE.lock().await;
        state.phase = RoundPhase::Active;

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        state.requests = Some(tx);
        state.request_task = Some(tokio::spawn(request_task(config.april2024.api, rx)));

        state.players = Vec::new();
        let mut members = config.april2024.guild.members_iter(ctx).boxed();
        while let Some(member) = members.next().await {
            let member = member?;

            if member.roles.contains(&config.april2024.player_role) {
                match member.add_role(ctx, config.april2024.playing_role).await {
                    Ok(_) => {
                        state.players.push(PlayerState::new(member));
                    }
                    Err(err) => {
                        warn!("Granting playing_role: {err:?}");
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        state
            .players
            .iter()
            .map(|player| (&player.member).into())
            .collect()
    };

    api(ctx, ApiRequest::RoundStart { users: players }).await?;

    announce(
        ctx,
        CreateMessage::new().content("@here New round has started!"),
    )
    .await?;

    add_rule(ctx).await?;

    Ok(())
}

pub async fn end_round(ctx: &Context) -> Result<()> {
    let mut state = STATE.lock().await;

    state.requests.take();
    if let Some(task) = state.request_task.take() {
        task.abort();
        let _ = task.await;
    }

    if state.phase == RoundPhase::Inactive {
        return Ok(());
    }
    state.phase = RoundPhase::Inactive;

    let mut message = MessageBuilder::new();
    message.push("Round has ended! ");
    match state.players.len() {
        0 => {
            message.push("Everyone was eliminated; no one wins!");
        }
        1 => {
            message.mention(&state.players[0].member);
            message.push("wins!");
        }
        _ => {
            message.push("The winners are: ");
            for player in &state.players {
                message.mention(&player.member);
                message.push(" ");
            }
        }
    }

    announce(
        ctx,
        CreateMessage::new()
            .allowed_mentions(
                CreateAllowedMentions::new()
                    .users(state.players.iter().map(|player| player.member.user.id)),
            )
            .content(message.build()),
    )
    .await?;

    api(ctx, ApiRequest::RoundEnd).await?;

    Ok(())
}

pub async fn eliminate(ctx: &Context, user_ids: &[UserId], reason: String) -> Result<()> {
    let mut message = MessageBuilder::new();
    if user_ids.len() == 1 {
        message.mention(&user_ids[0]).push(" has been eliminated! ");
    } else {
        for user_id in user_ids {
            message.mention(user_id);
            message.push(" ");
        }
        message.push("have been eliminated! ");
    }
    message.push(reason);

    let config = get_data::<ConfigKey>(ctx).await?;
    announce(
        ctx,
        CreateMessage::new()
            .allowed_mentions(CreateAllowedMentions::new().users(user_ids))
            .content(message.build()),
    )
    .await?;

    for user_id in user_ids {
        let member = config.april2024.guild.member(ctx, user_id).await?;
        member
            .remove_role(ctx, config.april2024.playing_role)
            .await?;
        api(
            ctx,
            ApiRequest::Eliminated {
                user: (&member).into(),
            },
        )
        .await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let should_end = {
        let mut state = STATE.lock().await;
        state
            .players
            .retain(|player| !user_ids.contains(&player.member.user.id));
        state.players.len() <= 1
    };

    if should_end {
        end_round(ctx).await?;
    }

    Ok(())
}

pub async fn idle_check(ctx: &Context, time_to_post: Duration) -> Result<()> {
    let cutoff = Instant::now() - time_to_post;
    let user_ids = STATE
        .lock()
        .await
        .players
        .iter()
        .filter_map(|player| {
            if player.last_message > cutoff {
                None
            } else {
                Some(player.member.user.id)
            }
        })
        .collect_vec();
    eliminate(
        ctx,
        &user_ids,
        format!(
            "They didn't post anything for {}.",
            humantime::format_duration(time_to_post)
        ),
    )
    .await?;
    Ok(())
}

pub async fn add_rule(ctx: &Context) -> Result<()> {
    api(ctx, ApiRequest::AddRule).await?;
    Ok(())
}
