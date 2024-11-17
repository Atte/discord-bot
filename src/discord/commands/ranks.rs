#![allow(clippy::non_canonical_partial_ord_impl)] // derivative

use super::super::{
    get_data,
    limits::{EMBED_DESC_LENGTH, REPLY_LENGTH},
    ConfigKey,
};
use crate::{discord::Context, util::ellipsis_string};
use color_eyre::eyre::{bail, eyre, OptionExt, Result};
use derivative::Derivative;
use itertools::{EitherOrBoth, Itertools};
use poise::command;
use serenity::all::{
    CreateEmbed, CreateEmbedFooter, CreateMessage, EditMember, GuildId, Member, MessageBuilder,
    Role, RoleId, UserId,
};
use std::{cmp::Ordering, collections::HashSet, io::Write};
use tabwriter::TabWriter;

pub fn cmp_roles(a: &Role, b: &Role) -> Option<Ordering> {
    a.name.partial_cmp(&b.name)
    // a.name.to_lowercase().partial_cmp(&b.name.to_lowercase())
}

#[derive(Derivative, Debug, Clone)]
#[derivative(PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    #[derivative(PartialOrd(compare_with = "cmp_roles"))]
    role: Role,
    #[derivative(PartialEq = "ignore", PartialOrd = "ignore", Ord = "ignore")]
    members: Vec<Member>,
}

#[derive(Debug, Clone)]
struct Ranks(Vec<Rank>);

impl Ranks {
    #[inline]
    fn new(mut ranks: Vec<Rank>) -> Self {
        ranks.sort_unstable();
        Self(ranks)
    }

    async fn from_guild(
        ctx: &serenity::all::Context,
        guild_id: impl Into<GuildId>,
    ) -> Result<Self> {
        let config = get_data::<ConfigKey>(ctx).await?;
        let guild = guild_id
            .into()
            .to_guild_cached(ctx)
            .ok_or_else(|| eyre!("Guild not found!"))?
            .clone();

        let highest_position = guild
            .roles
            .values()
            .filter(|role| config.discord.rank_start_roles.contains(&role.id))
            .map(|role| role.position)
            .max()
            .ok_or_else(|| eyre!("Ranks start marker not found!"))?;
        let lowest_position = guild
            .roles
            .values()
            .filter(|role| config.discord.rank_end_roles.contains(&role.id))
            .map(|role| role.position)
            .min()
            .ok_or_else(|| eyre!("Ranks end marker not found!"))?;

        Ok(Self::new(
            guild
                .roles
                .values()
                .filter(|role| {
                    !config.discord.rank_start_roles.contains(&role.id)
                        && !config.discord.rank_end_roles.contains(&role.id)
                        && role.position > lowest_position
                        && role.position < highest_position
                        && !role.name.starts_with('@')
                })
                .cloned()
                .map(|role| Rank {
                    members: guild
                        .members
                        .values()
                        .filter(|member| member.roles.iter().any(|id| id == &role.id))
                        .cloned()
                        .collect(),
                    role,
                })
                .collect(),
        ))
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    fn iter(&self) -> impl Iterator<Item = &Rank> + '_ {
        self.0.iter()
    }

    #[inline]
    fn contains(&self, rank: &Rank) -> bool {
        self.0.contains(rank)
    }

    fn of_user(&self, user: impl Into<UserId>) -> Self {
        let user_id = user.into();
        Self::new(
            self.0
                .iter()
                .filter(|rank| rank.members.iter().any(|member| member.user.id == user_id))
                .cloned()
                .collect(),
        )
    }

    fn by_name(&self, name: impl AsRef<str>) -> Option<Rank> {
        let search = name.as_ref().to_lowercase();
        self.0
            .iter()
            .find(|rank| rank.role.name.to_lowercase() == search)
            .cloned()
    }

    fn names(&self) -> impl Iterator<Item = String> + '_ {
        self.0.iter().map(|rank| rank.role.name.clone())
    }

    fn member_counts(&self) -> impl Iterator<Item = (String, usize)> + '_ {
        self.0
            .iter()
            .map(|rank| (rank.role.name.clone(), rank.members.len()))
    }
}

async fn handle_joinleave(
    ctx: &Context<'_>,
    args: Vec<String>,
    mut on_join: impl FnMut(&Rank, &mut MessageBuilder) -> bool,
    mut on_leave: impl FnMut(&Rank, &mut MessageBuilder) -> bool,
) -> Result<()> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| eyre!("No guild_id on Message!"))?;
    let ranks = Ranks::from_guild(ctx.serenity_context(), guild_id).await?;
    let mut user_role_ids: HashSet<RoleId> = ctx
        .author_member()
        .await
        .as_ref()
        .ok_or_else(|| eyre!("No Member on Message!"))?
        .roles
        .iter()
        .copied()
        .collect();

    let mut response = MessageBuilder::new();
    for arg in args {
        let name = arg.trim();
        if let Some(rank) = ranks.by_name(name) {
            if user_role_ids.contains(&rank.role.id) {
                if on_leave(&rank, &mut response) {
                    user_role_ids.remove(&rank.role.id);
                }
            } else {
                if on_join(&rank, &mut response) {
                    user_role_ids.insert(rank.role.id);
                }
            }
        } else {
            response.push("No such rank: ").push_line_safe(name);
        }
    }
    guild_id
        .edit_member(
            &ctx,
            ctx.author().id,
            EditMember::new().roles(user_role_ids),
        )
        .await?;
    ctx.reply(response.build()).await?;
    Ok(())
}

/// Join a rank
#[command(prefix_command, category = "Ranks", aliases("gain"))]
pub async fn join(ctx: Context<'_>, ranks: Vec<String>) -> Result<()> {
    handle_joinleave(
        &ctx,
        ranks,
        |rank, response| {
            response.push("Joined ").push_line_safe(&rank.role.name);
            true
        },
        |rank, response| {
            response.push("Already in ").push_line_safe(&rank.role.name);
            false
        },
    )
    .await
}

/// Leave a rank
#[command(prefix_command, category = "Ranks")]
pub async fn leave(ctx: Context<'_>, ranks: Vec<String>) -> Result<()> {
    handle_joinleave(
        &ctx,
        ranks,
        |rank, response| {
            response
                .push("Already not in ")
                .push_line_safe(&rank.role.name);
            false
        },
        |rank, response| {
            response.push("Left ").push_line_safe(&rank.role.name);
            true
        },
    )
    .await
}

/// Join/leave a rank
#[command(prefix_command, category = "Ranks")]
pub async fn rank(ctx: Context<'_>, ranks: Vec<String>) -> Result<()> {
    handle_joinleave(
        &ctx,
        ranks,
        |rank, response| {
            response.push("Joined ").push_line_safe(&rank.role.name);
            true
        },
        |rank, response| {
            response.push("Left ").push_line_safe(&rank.role.name);
            true
        },
    )
    .await
}

/// List all available ranks, and which ones you currently have
#[command(prefix_command, category = "Ranks")]
pub async fn ranks(ctx: Context<'_>) -> Result<()> {
    let ranks = Ranks::from_guild(
        ctx.serenity_context(),
        ctx.guild_id().ok_or_eyre("no guild ID")?,
    )
    .await?;

    let rank_list = {
        let mut tw = TabWriter::new(Vec::new());
        let counts: Vec<_> = ranks.member_counts().collect();
        for row in counts
            .iter()
            .take((counts.len() + 1) / 2)
            .zip_longest(counts.iter().skip((counts.len() + 1) / 2))
        {
            match row {
                EitherOrBoth::Both((left_name, left_count), (right_name, right_count)) => {
                    write!(
                        &mut tw,
                        "{left_name} ({left_count})\t{right_name} ({right_count})"
                    )?;
                }
                EitherOrBoth::Left((name, count)) => {
                    write!(&mut tw, "{name} ({count})")?;
                }
                EitherOrBoth::Right((name, count)) => {
                    write!(&mut tw, "\t{name} ({count})")?;
                }
            }
            writeln!(&mut tw)?;
        }
        String::from_utf8(tw.into_inner()?)?
    };

    let prefix = get_data::<ConfigKey>(ctx.serenity_context())
        .await?
        .discord
        .command_prefix;
    ctx.channel_id()
        .send_message(
            ctx,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title("Ranks")
                    .footer(CreateEmbedFooter::new(format!(
                        "Use {prefix}join and {prefix}leave to change your ranks"
                    )))
                    .description(ellipsis_string(
                        MessageBuilder::new()
                            .push_codeblock_safe(rank_list, None)
                            .build(),
                        EMBED_DESC_LENGTH,
                    )),
            ),
        )
        .await?;

    let user_ranks = ranks.of_user(ctx.author().id);
    ctx.reply(if user_ranks.is_empty() {
        format!("You currently have no ranks. Use the {prefix}join command to join some.")
    } else {
        ellipsis_string(
            format!("Your ranks are: {}", user_ranks.names().join(", ")),
            REPLY_LENGTH,
        )
    })
    .await?;

    #[cfg(feature = "dropdowns")]
    {
        use serenity::all::{
            CreateActionRow, CreateAllowedMentions, CreateSelectMenu, CreateSelectMenuKind,
            CreateSelectMenuOption,
        };

        let components = ranks
            .iter()
            .map(|rank| {
                CreateSelectMenuOption::new(rank.role.name.clone(), rank.role.id.to_string())
                    .default_selection(user_ranks.contains(&rank))
            })
            .chunks(25)
            .into_iter()
            .enumerate()
            .map(|(i, chunk)| {
                let options: Vec<_> = chunk.collect();
                let options_len = u8::try_from(options.len()).unwrap_or(25);
                CreateActionRow::SelectMenu(
                    CreateSelectMenu::new(
                        format!("ranks-{i}"),
                        CreateSelectMenuKind::String { options },
                    )
                    .min_values(1)
                    .max_values(options_len),
                )
            })
            .collect();

        ctx.channel_id()
            .send_message(
                ctx,
                CreateMessage::new()
                    .reference_message(match ctx {
                        poise::Context::Application(_) => {
                            bail!("command not a message");
                        }
                        poise::Context::Prefix(prefix_context) => prefix_context.msg,
                    })
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .components(components),
            )
            .await?;
    }

    Ok(())
}
