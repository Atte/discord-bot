use cached::proc_macro::cached;
use color_eyre::eyre::{eyre, Result};
use itertools::Itertools;
use lazy_regex::regex::{self, Regex};
use serenity::all::{
    automod::Action, AutomodEventType, CacheHttp, Context, GuildId, Message, MessageBuilder, Rule,
    Trigger,
};

use super::{get_data, log_channel, ConfigKey};

#[cached(time = 60, sync_writes = true, key = "GuildId", convert = "{ guild }")]
async fn get_rules(ctx: &Context, guild: GuildId) -> Result<Vec<Rule>, String> {
    guild
        .automod_rules(ctx.http())
        .await
        .map_err(|err| format!("{:?}", eyre!(err)))
}

fn wildcards_to_regex(s: impl AsRef<str>) -> String {
    let s = s.as_ref();
    match (s.starts_with('*'), s.ends_with('*')) {
        (true, true) => format!("{}", regex::escape(&s)),
        (true, false) => format!("{}\\b", regex::escape(&s)),
        (false, true) => format!("\\b{}", regex::escape(&s)),
        (false, false) => format!("\\b{}\\b", regex::escape(&s)),
    }
}

fn parse_regex(re: impl AsRef<str>) -> Option<Regex> {
    let re = re.as_ref();
    match Regex::new(&format!("(?i){re}")) {
        Ok(re) => Some(re),
        Err(err) => {
            log::warn!("Invalid automod regex '{re}': {err:?}");
            None
        }
    }
}

pub async fn enforce(ctx: &Context, message: &Message) -> Result<()> {
    let config = get_data::<ConfigKey>(ctx).await?;
    let Some(guild_id) = message.guild_id else {
        return Ok(());
    };
    if !config.discord.enforce_automods.contains(&guild_id) {
        return Ok(());
    }
    let member = message.member(ctx).await?;

    let rules = get_rules(ctx, guild_id).await.map_err(|err| eyre!(err))?;
    for rule in rules {
        if rule.event_type != AutomodEventType::MessageSend
            || rule.exempt_channels.contains(&message.channel_id)
            || member
                .roles
                .iter()
                .any(|role| rule.exempt_roles.contains(role))
        {
            continue;
        }

        let Trigger::Keyword {
            strings,
            regex_patterns,
            allow_list,
        } = rule.trigger
        else {
            continue;
        };

        let mut block_regexes = strings
            .into_iter()
            .map(wildcards_to_regex)
            .chain(regex_patterns.into_iter())
            .filter_map(parse_regex);
        let allow_regexes = allow_list
            .into_iter()
            .map(wildcards_to_regex)
            .filter_map(parse_regex)
            .collect_vec();

        if !block_regexes.any(|block| {
            block.is_match(&message.content)
                && !allow_regexes
                    .iter()
                    .any(|allow| allow.is_match(&message.content))
        }) {
            continue;
        }

        let title = rule.name;
        for action in rule.actions {
            match action {
                Action::BlockMessage { custom_message } => {
                    if let Err(err) = message
                        .reply_ping(
                            ctx,
                            MessageBuilder::new()
                                .push(
                                    custom_message.unwrap_or_else(|| "AutoMod blocked".to_string()),
                                )
                                .build(),
                        )
                        .await
                    {
                        log::warn!("Failed to yell at modmin for automod match: {err:?}");
                    }
                    if let Err(err) = message.delete(ctx).await {
                        log::error!("Failed to delete automod matched message: {err:?}");
                    }
                }
                Action::Alert(channel_id) => {
                    if config.discord.log_channels.contains(&channel_id) {
                        if let Err(err) =
                            log_channel::automod_enforced(ctx, guild_id, &message, &title).await
                        {
                            log::error!("Failed to log automod enforcement: {err:?}");
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
