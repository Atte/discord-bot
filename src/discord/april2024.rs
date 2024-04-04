use async_recursion::async_recursion;
use color_eyre::eyre::Result;
use futures::StreamExt;
use itertools::Itertools;
use log::warn;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serenity::all::{
    Context, CreateAllowedMentions, CreateMessage, Member, Message, MessageBuilder, User, UserId,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tokio::sync::{mpsc::Sender, Mutex};
use tokio::task::JoinHandle;

use super::{get_data, ConfigKey};

pub const MIN_PLAYERS: usize = 2;
pub const MAX_IDLE_ROUNDS: usize = 3;

static ROUND_ID: AtomicUsize = AtomicUsize::new(MAX_IDLE_ROUNDS);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundPhase {
    Idle,
    Pending,
    Active,
}

#[derive(Debug)]
pub struct RoundState {
    id: usize,
    pub phase: RoundPhase,
    pub tx_end: Option<Sender<()>>,
    players: Vec<PlayerState>,
    player_last_rounds: BTreeMap<UserId, usize>,
    requests: Option<Sender<(Vec<ApiRequest>, oneshot::Sender<Vec<ApiResponse>>)>>,
    request_task: Option<JoinHandle<()>>,
}

impl RoundState {
    #[inline]
    pub const fn new() -> Self {
        Self {
            id: 0,
            phase: RoundPhase::Idle,
            tx_end: None,
            players: Vec::new(),
            player_last_rounds: BTreeMap::new(),
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
    pub fn new(member: Member, last_message: Instant) -> Self {
        Self {
            member,
            last_message,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum AnnounceTarget {
    Lobby,
    Arena,
    Both,
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
        target: AnnounceTarget,
    },
}

async fn announce(ctx: &Context, target: AnnounceTarget, message: CreateMessage) -> Result<()> {
    let config = get_data::<ConfigKey>(ctx).await?;
    match target {
        AnnounceTarget::Lobby => {
            config
                .april2024
                .lobby_channel
                .send_message(ctx, message)
                .await?;
        }
        AnnounceTarget::Arena => {
            config
                .april2024
                .arena_channel
                .send_message(ctx, message)
                .await?;
        }
        AnnounceTarget::Both => {
            config
                .april2024
                .lobby_channel
                .send_message(ctx, message.clone())
                .await?;
            config
                .april2024
                .arena_channel
                .send_message(ctx, message)
                .await?;
        }
    }
    Ok(())
}

pub async fn message(ctx: &Context, message: &Message) -> Result<()> {
    let mut state = STATE.lock().await;
    if state.phase != RoundPhase::Active {
        return Ok(());
    }

    let mut found = false;
    let member = message.member(ctx).await?;
    for player in &mut state.players {
        if player.member.user.id == member.user.id {
            player.last_message = Instant::now();
            found = true;
        }
    }

    *state.player_last_rounds.entry(member.user.id).or_default() = state.id;

    drop(state);
    if !found {
        return Ok(());
    }

    api(
        ctx,
        vec![ApiRequest::Message {
            user: (&member).into(),
            text: message.content_safe(ctx),
        }],
    )
    .await
}

async fn request_task(
    url: Option<Url>,
    mut rx: Receiver<(Vec<ApiRequest>, oneshot::Sender<Vec<ApiResponse>>)>,
) {
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("failed to build API client");
    while let Some((request, tx)) = rx.recv().await {
        if STATE.lock().await.requests.is_none() {
            return;
        }

        if let Some(ref url) = url {
            for _ in 0..3 {
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
                        if !err.is_timeout() {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
            }
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tx.send(Vec::new());
        }
    }
}

#[async_recursion]
async fn api(ctx: &Context, request: Vec<ApiRequest>) -> Result<()> {
    let config = get_data::<ConfigKey>(ctx).await?;
    log::debug!("{request:?}");
    if config.april2024.debug {
        announce(
            ctx,
            AnnounceTarget::Arena,
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
        match rx.await {
            Ok(response) => response,
            Err(err) => {
                warn!("API response error: {err:?}");
                return Ok(());
            }
        }
    };

    let state = STATE.lock().await;
    if state.phase != RoundPhase::Active {
        return Ok(());
    }

    log::debug!("{response:?}");
    if config.april2024.debug {
        announce(
            ctx,
            AnnounceTarget::Arena,
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
                    if state
                        .players
                        .iter()
                        .any(|player| player.member.user.id == id)
                    {
                        eliminations
                            .entry(reason)
                            .or_default()
                            .push(UserId::new(id));
                    }
                }
            }
            ApiResponse::Announce { text, here, target } => {
                if here {
                    announce(
                        ctx,
                        target,
                        CreateMessage::new()
                            .allowed_mentions(CreateAllowedMentions::new().everyone(true))
                            .content(format!("@here {text}")),
                    )
                    .await?;
                } else {
                    announce(ctx, target, CreateMessage::new().content(text)).await?;
                }
            }
        }
    }

    drop(state);
    for (reason, user_ids) in eliminations {
        eliminate(
            ctx,
            user_ids,
            reason.unwrap_or_else(|| "They broke the rules.".to_owned()),
        )
        .await?;
    }

    Ok(())
}

pub async fn start_round(ctx: &Context) -> Result<bool> {
    let mut state = STATE.lock().await;

    let now = Instant::now();

    let config = get_data::<ConfigKey>(ctx).await?;
    let mut players: Vec<PlayerState> = Vec::new();
    let mut members = config.april2024.guild.members_iter(ctx).boxed();
    while let Some(member) = members.next().await {
        let member = member?;

        if member.roles.contains(&config.april2024.player_role) {
            players.push(PlayerState::new(member, now));
        }
    }

    if players.len() < MIN_PLAYERS {
        log::info!(
            "not enough players, have {}, need at least {MIN_PLAYERS}",
            players.len()
        );
        // announce(
        //     ctx,
        //     AnnounceTarget::Lobby,
        //     CreateMessage::new().content(format!(
        //         "Not enough players! Need at least {MIN_PLAYERS} to start a round."
        //     )),
        // )
        // .await?;
        return Ok(false);
    }

    state.id = ROUND_ID.fetch_add(1, Ordering::SeqCst);
    let last_round_id = state.id.saturating_sub(1);
    state.phase = RoundPhase::Active;

    let (tx, rx) = tokio::sync::mpsc::channel(128);
    state.requests = Some(tx);
    state.request_task = Some(tokio::spawn(request_task(
        config.april2024.api.and_then(|url| url.parse().ok()),
        rx,
    )));

    state.players = Vec::new();
    for player in players {
        match player
            .member
            .add_role(ctx, config.april2024.playing_role)
            .await
        {
            Ok(_) => {
                state
                    .player_last_rounds
                    .entry(player.member.user.id)
                    .or_insert(last_round_id);
                state.players.push(player);
            }
            Err(err) => {
                warn!("Granting playing_role: {err:?}");
            }
        }
    }

    let api_players: Vec<ApiUser> = state
        .players
        .iter()
        .map(|player| (&player.member).into())
        .collect();
    drop(state);
    api(ctx, vec![ApiRequest::RoundStart { users: api_players }]).await?;

    // announce(
    //     ctx,
    //     false,
    //     CreateMessage::new().content("@here New round has started!"),
    // )
    // .await?;

    // `add_rule` is called by initial tokio interval

    Ok(true)
}

pub async fn end_round(ctx: &Context) -> Result<()> {
    let mut state = STATE.lock().await;

    if state.phase != RoundPhase::Active {
        return Ok(());
    }
    state.phase = RoundPhase::Pending;

    let mut message = MessageBuilder::new();
    match state.players.len() {
        0 => {
            message.push("Everyone was eliminated; no one wins!");
        }
        1 => {
            message.mention(&state.players[0].member);
            message.push(" wins!");
        }
        _ => {
            message.push("It's a tie! The winners are: ");
            for player in &state.players {
                message.mention(&player.member);
                message.push(" ");
            }
        }
    }

    let idle_user_ids: BTreeSet<UserId> = state
        .player_last_rounds
        .iter()
        .filter_map(|(id, last)| {
            if *last <= state.id.saturating_sub(MAX_IDLE_ROUNDS) {
                Some(*id)
            } else {
                None
            }
        })
        .collect();

    let config = get_data::<ConfigKey>(ctx).await?;
    for player in &state.players {
        player
            .member
            .remove_role(ctx, config.april2024.playing_role)
            .await?;
    }

    announce(
        ctx,
        AnnounceTarget::Lobby,
        CreateMessage::new()
            .allowed_mentions(
                CreateAllowedMentions::new().users(
                    state
                        .players
                        .iter()
                        .map(|player| player.member.user.id)
                        .unique(),
                ),
            )
            .content(message.build()),
    )
    .await?;

    if !idle_user_ids.is_empty() {
        for user_id in &idle_user_ids {
            let member = config.april2024.guild.member(ctx, user_id).await?;
            member
                .remove_role(ctx, config.april2024.player_role)
                .await?;
        }

        let mut message = MessageBuilder::new();
        match idle_user_ids.len() {
            1 => {
                let user_id = idle_user_ids.iter().next().unwrap();
                state.player_last_rounds.remove(&user_id);
                message.mention(user_id);
                message.push(" hasn't ");
            }
            _ => {
                for user_id in &idle_user_ids {
                    state.player_last_rounds.remove(&user_id);
                    message.mention(user_id);
                    message.push(" ");
                }
                message.push(" haven't ");
            }
        }
        message.push(format!("said anything in {MAX_IDLE_ROUNDS} rounds, so won't be added to the next round. To join back, use "));
        message.push_mono(format!("{}rank btbg", config.discord.command_prefix));
        announce(
            ctx,
            AnnounceTarget::Lobby,
            CreateMessage::new()
                .allowed_mentions(CreateAllowedMentions::new()) //.users(idle_user_ids))
                .content(message.build()),
        )
        .await?;
    }

    state.requests.take();
    if let Some(task) = state.request_task.take() {
        task.abort();
        let _ = task.await;
    }

    Ok(())
}

pub async fn eliminate(ctx: &Context, user_ids: Vec<UserId>, reason: String) -> Result<()> {
    let user_ids = {
        let mut state = STATE.lock().await;
        let user_ids = user_ids
            .into_iter()
            .unique()
            .filter(|uid| {
                state
                    .players
                    .iter()
                    .any(|player| player.member.user.id == *uid)
            })
            .collect_vec();
        state
            .players
            .retain(|player| !user_ids.contains(&player.member.user.id));
        user_ids
    };
    if user_ids.is_empty() {
        return Ok(());
    }

    let mut message = MessageBuilder::new();
    if user_ids.len() == 1 {
        message.mention(&user_ids[0]).push(" has been eliminated! ");
    } else {
        for user_id in &user_ids {
            message.mention(user_id);
            message.push(" ");
        }
        message.push("have been eliminated! ");
    }
    message.push(reason);

    let config = get_data::<ConfigKey>(ctx).await?;
    announce(
        ctx,
        AnnounceTarget::Both,
        CreateMessage::new()
            .allowed_mentions(CreateAllowedMentions::new()) //.users(&user_ids))
            .content(message.build()),
    )
    .await?;

    let mut members = Vec::new();
    for user_id in &user_ids {
        let member = config.april2024.guild.member(ctx, user_id).await?;
        member
            .remove_role(ctx, config.april2024.playing_role)
            .await?;
        members.push(member);
    }

    api(
        ctx,
        members
            .into_iter()
            .map(|member| ApiRequest::Eliminated {
                user: (&member).into(),
            })
            .collect(),
    )
    .await?;

    if STATE.lock().await.players.len() <= 1 {
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
        user_ids,
        format!(
            "They didn't post anything for {}.",
            humantime::format_duration(time_to_post)
        ),
    )
    .await?;
    Ok(())
}

pub async fn add_rule(ctx: &Context) -> Result<()> {
    api(ctx, vec![ApiRequest::AddRule]).await?;
    Ok(())
}
