#![allow(clippy::incorrect_partial_ord_impl_on_ord_type)] // derivative

use super::super::{
    get_data,
    limits::{EMBED_DESC_LENGTH, REPLY_LENGTH},
    ConfigKey,
};
use crate::util::ellipsis_string;
use color_eyre::eyre::{eyre, OptionExt, Result};
use derivative::Derivative;
use itertools::{EitherOrBoth, Itertools};
use serenity::{
    all::{CreateEmbed, CreateEmbedFooter, CreateMessage, EditMember},
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::{
        channel::Message,
        guild::{Member, Role},
        id::{GuildId, RoleId, UserId},
    },
    utils::MessageBuilder,
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

    async fn from_guild(ctx: &Context, guild_id: impl Into<GuildId>) -> Result<Self> {
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

    async fn from_message(ctx: &Context, msg: &Message) -> Result<Self> {
        Self::from_guild(
            ctx,
            msg.guild_id
                .ok_or_else(|| eyre!("No guild_id on Message!"))?,
        )
        .await
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
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

    fn names(&self) -> Vec<String> {
        self.0.iter().map(|rank| rank.role.name.clone()).collect()
    }

    fn member_counts(&self) -> Vec<(String, usize)> {
        self.0
            .iter()
            .map(|rank| (rank.role.name.clone(), rank.members.len()))
            .collect()
    }

    #[cfg(feature = "colors")]
    pub async fn colors(&mut self, ctx: &Context, guild_id: GuildId) -> Result<Vec<Rank>> {
        let config = get_data::<ConfigKey>(ctx).await?;

        let guild = guild_id
            .to_guild_cached(ctx)
            .ok_or_eyre("Guild not in cache")?;

        let highest_position = guild
            .roles
            .values()
            .filter(|role| config.discord.colors.start_roles.contains(&role.id))
            .map(|role| role.position)
            .max()
            .ok_or_else(|| eyre!("Colors start marker not found!"))?;
        let lowest_position = guild
            .roles
            .values()
            .filter(|role| config.discord.colors.end_roles.contains(&role.id))
            .map(|role| role.position)
            .min()
            .ok_or_else(|| eyre!("Colors end marker not found!"))?;

        Ok(self
            .0
            .iter()
            .filter(|rank| {
                rank.role.position > lowest_position && rank.role.position < highest_position
            })
            .cloned()
            .collect())
    }
}

async fn handle_joinleave(
    ctx: &Context,
    msg: &Message,
    mut args: Args,
    mut on_join: impl FnMut(&Rank, &mut MessageBuilder) -> bool,
    mut on_leave: impl FnMut(&Rank, &mut MessageBuilder) -> bool,
) -> CommandResult {
    let guild_id = msg
        .guild_id
        .ok_or_else(|| eyre!("No guild_id on Message!"))?;
    let mut ranks = Ranks::from_guild(ctx, guild_id).await?;
    let mut user_role_ids: HashSet<RoleId> = msg
        .member
        .as_ref()
        .ok_or_else(|| eyre!("No Member on Message!"))?
        .roles
        .iter()
        .copied()
        .collect();

    #[cfg(feature = "colors")]
    let colors = ranks.colors(ctx, guild_id).await?;

    let config = get_data::<ConfigKey>(ctx).await?;
    let mut response = MessageBuilder::new();
    'outer: for arg in args.iter::<String>().map(Result::unwrap) {
        let name = arg.trim();
        if let Some(rank) = ranks.by_name(name) {
            if user_role_ids.contains(&rank.role.id) {
                if on_leave(&rank, &mut response) {
                    user_role_ids.remove(&rank.role.id);
                }
            } else {
                for (key, restricted) in &config.discord.restricted_ranks {
                    let key = RoleId::new(key.parse()?);
                    if rank.role.id == key
                        || (restricted.contains(&rank.role.id) && !user_role_ids.contains(&key))
                    {
                        response
                            .push("You are not allowed to join ")
                            .push_line_safe(&rank.role.name);
                        continue 'outer;
                    }
                }

                #[cfg(feature = "colors")]
                if colors.iter().any(|color| color.role.id == rank.role.id) {
                    for color in colors.iter().filter(|color| color.role.id != rank.role.id) {
                        if user_role_ids.contains(&color.role.id) {
                            user_role_ids.remove(&color.role.id);
                        }
                    }
                }

                if on_join(&rank, &mut response) {
                    // TODO: leave other restricted ranks
                    user_role_ids.insert(rank.role.id);
                }
            }
        } else {
            response.push("No such rank: ").push_line_safe(name);
        }
    }
    guild_id
        .edit_member(&ctx, &msg.author, EditMember::new().roles(user_role_ids))
        .await?;
    msg.reply(ctx, response.build()).await?;
    Ok(())
}

#[command]
#[aliases(gain)]
#[description("Join a rank")]
#[min_args(1)]
async fn join(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    handle_joinleave(
        ctx,
        msg,
        args,
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

#[command]
#[description("Leave a rank")]
#[min_args(1)]
async fn leave(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    handle_joinleave(
        ctx,
        msg,
        args,
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

#[command]
#[aliases(role)]
#[description("Join/leave a rank")]
#[help_available(false)]
#[min_args(1)]
async fn rank(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    handle_joinleave(
        ctx,
        msg,
        args,
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

#[command]
#[aliases(roles)]
#[description("List all available ranks, and which ones you currently have")]
#[num_args(0)]
async fn ranks(ctx: &Context, msg: &Message) -> CommandResult {
    let ranks = Ranks::from_message(ctx, msg).await?;

    let rank_list = {
        let mut tw = TabWriter::new(Vec::new());
        let counts = ranks.member_counts();
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

    let prefix = get_data::<ConfigKey>(ctx).await?.discord.command_prefix;
    msg.channel_id
        .send_message(
            &ctx,
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

    let user_ranks = ranks.of_user(&msg.author);
    msg.reply(
        ctx,
        if user_ranks.is_empty() {
            format!("You currently have no ranks. Use the {prefix}join command to join some.")
        } else {
            ellipsis_string(
                format!("Your ranks are: {}", user_ranks.names().join(", ")),
                REPLY_LENGTH,
            )
        },
    )
    .await?;
    Ok(())
}
