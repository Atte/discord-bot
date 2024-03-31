use crate::discord::{
    april2024::{add_rule, end_round, idle_check, start_round, RoundPhase},
    get_data, ConfigKey,
};

use super::super::april2024::STATE;
use log::error;
use serenity::all::{
    standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    Context, CreateMessage, Message,
};
use tokio::select;

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("Start a new round")]
#[usage("time_between_rounds time_to_post new_rule_interval")]
#[num_args(3)]
async fn btbgstart(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if STATE.lock().await.phase != RoundPhase::Idle {
        msg.reply(ctx, "A round is already in progress!").await?;
        return Ok(());
    }

    let Ok(time_between_rounds) = humantime::parse_duration(&args.single::<String>()?) else {
        msg.reply(
            ctx,
            "Invalid time_between_rounds! Use something along the lines of: 15min",
        )
        .await?;
        return Ok(());
    };

    let Ok(time_to_post) = humantime::parse_duration(&args.single::<String>()?) else {
        msg.reply(
            ctx,
            "Invalid time_to_post! Use something along the lines of: 3min",
        )
        .await?;
        return Ok(());
    };

    let Ok(new_rule_interval) = humantime::parse_duration(&args.single::<String>()?) else {
        msg.reply(
            ctx,
            "Invalid new_rule_interval! Use something along the lines of: 5min",
        )
        .await?;
        return Ok(());
    };

    let (tx_end, mut rx_end) = tokio::sync::mpsc::channel::<()>(1);
    STATE.lock().await.tx_end = Some(tx_end);

    let ctx = ctx.clone();
    tokio::spawn(async move {
        'outer: loop {
            if let Err(err) = start_round(&ctx).await {
                error!("start_round: {err:?}");
                break;
            }

            let mut new_rule_interval = tokio::time::interval(new_rule_interval);
            loop {
                let second = tokio::time::sleep(tokio::time::Duration::from_secs(1));
                select! {
                    _ = rx_end.recv() => { break 'outer; }
                    _ = second => {
                        if let Err(err) = idle_check(&ctx, time_to_post).await {
                            error!("idle_check: {err:?}");
                        }
                    }
                    _ = new_rule_interval.tick() => {
                        if let Err(err) = add_rule(&ctx).await {
                            error!("add_rule: {err:?}");
                        }
                    }
                }

                if STATE.lock().await.phase != RoundPhase::Active {
                    break;
                }
            }

            if let Ok(config) = get_data::<ConfigKey>(&ctx).await {
                let _ = config
                    .april2024
                    .arena_channel
                    .send_message(
                        &ctx,
                        CreateMessage::new().content(format!(
                            "Next round will start in {}",
                            humantime::format_duration(time_between_rounds)
                        )),
                    )
                    .await;
            }

            let between_rounds = tokio::time::sleep(time_between_rounds);
            select! {
                _ = rx_end.recv() => { break 'outer; }
                _ = between_rounds => { }
            }
        }

        if let Err(err) = end_round(&ctx).await {
            error!("end_round: {err:?}");
        };
        STATE.lock().await.tx_end.take();
    });

    Ok(())
}

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("End the current round")]
#[num_args(0)]
async fn btbgend(ctx: &Context, msg: &Message) -> CommandResult {
    let mut state = STATE.lock().await;
    if state.phase == RoundPhase::Idle {
        msg.reply(ctx, "No round is in progress or pending!")
            .await?;
        return Ok(());
    }

    end_round(ctx).await?;

    let config = get_data::<ConfigKey>(ctx).await?;
    config
        .april2024
        .lobby_channel
        .send_message(
            &ctx,
            CreateMessage::new().content(if state.phase == RoundPhase::Pending {
                "Cancelled automatic start of next round."
            } else {
                "Round ended manually. Cancelled automatic start of next round."
            }),
        )
        .await?;

    if let Some(tx) = state.tx_end.take() {
        tx.send(()).await?;
    }

    Ok(())
}

#[group("BTBG")]
#[only_in(guilds)]
#[commands(btbgstart, btbgend)]
pub struct April2024;
