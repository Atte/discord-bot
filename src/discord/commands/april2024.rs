use crate::discord::{april2024::RoundPhase, get_data, ConfigKey};

use super::super::april2024::{RoundState, STATE};
use serenity::all::{
    standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    Context, CreateMessage, Message, MessageBuilder,
};
use tokio::{
    select,
    sync::{mpsc::Sender, Mutex},
};

pub static TX_END_ROUND: Mutex<Option<Sender<()>>> = Mutex::const_new(None);

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("Start a new round")]
#[usage("duration")]
#[num_args(1)]
async fn start(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if STATE.lock().await.phase != RoundPhase::Idle {
        msg.reply(ctx, "A round is already in progress!").await?;
        return Ok(());
    }

    let Ok(duration) = humantime::parse_duration(args.rest()) else {
        msg.reply(
            ctx,
            "Invalid duration! Use something along the lines of: 1h 30min",
        )
        .await?;
        return Ok(());
    };

    {
        let mut state = RoundState::new();
        state.phase = RoundPhase::Starting;
        *STATE.lock().await = state;
    }

    let config = get_data::<ConfigKey>(ctx).await?;
    config
        .discord
        .april2024
        .lobby
        .send_message(
            ctx,
            CreateMessage::new().content(format!(
                "A round is about to start! Use {}battle to join in.",
                config.discord.command_prefix
            )),
        )
        .await?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    *TX_END_ROUND.lock().await = Some(tx);

    let sleep = tokio::time::sleep(duration);
    select! {
        _ = rx.recv() => { }
        _ = sleep => {
            STATE.lock().await.phase = RoundPhase::Running;
            config
            .discord
            .april2024.arena.send_message(&ctx, CreateMessage::new().content("Round has started!")).await?;
        }
    }

    rx.recv().await;

    STATE.lock().await.phase = RoundPhase::Idle;

    Ok(())
}

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("End the current round")]
#[num_args(0)]
async fn end(ctx: &Context, msg: &Message) -> CommandResult {
    if STATE.lock().await.phase == RoundPhase::Idle {
        msg.reply(ctx, "No round is in progress!").await?;
        return Ok(());
    }

    let config = get_data::<ConfigKey>(ctx).await?;
    config
        .discord
        .april2024
        .arena
        .send_message(&ctx, CreateMessage::new().content("Round ended manually"))
        .await?;

    if let Some(tx) = TX_END_ROUND.lock().await.take() {
        tx.send(()).await?;
    }

    Ok(())
}

#[command]
#[description("Join in on the current round")]
#[num_args(0)]
async fn battle(ctx: &Context, msg: &Message) -> CommandResult {
    match STATE.lock().await.phase {
        RoundPhase::Idle => {
            msg.reply(ctx, "There is currently no joinable round!")
                .await?;
        }
        RoundPhase::Starting => {
            let config = get_data::<ConfigKey>(ctx).await?;
            let member = msg.member(ctx).await?;
            member.add_role(ctx, config.discord.april2024.role).await?;
        }
        RoundPhase::Running => {
            msg.reply(
                ctx,
                "The round has already started! Wait for the next round to join.",
            )
            .await?;
        }
    }
    Ok(())
}

#[group]
#[only_in(guilds)]
#[commands(start, end, battle)]
pub struct April2024;
