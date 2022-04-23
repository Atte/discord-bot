use super::super::{
    get_data,
    limits::{EMBED_DESC_LENGTH, REPLY_LENGTH},
    ConfigKey,
};
use crate::util::ellipsis_string;
use color_eyre::eyre::{eyre, Result};
use derivative::Derivative;
use itertools::Itertools;
use serenity::{
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
    a.name.to_lowercase().partial_cmp(&b.name.to_lowercase())
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
        let guild = guild_id
            .into()
            .to_guild_cached(&ctx)
            .ok_or_else(|| eyre!("Guild not found!"))?;
        let cutoff_position = guild
            .member(&ctx, ctx.cache.current_user_id())
            .await?
            .roles(&ctx)
            .ok_or_else(|| eyre!("Roles for bot not found!"))?
            .into_iter()
            .map(|role| role.position)
            .min()
            .ok_or_else(|| eyre!("Empty roles for bot!"))?;
        Ok(Self::new(
            guild
                .roles
                .values()
                .filter(|role| role.position < cutoff_position && !role.name.starts_with('@'))
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
    let ranks = Ranks::from_guild(ctx, guild_id).await?;
    let mut user_role_ids: HashSet<RoleId> = msg
        .member
        .as_ref()
        .ok_or_else(|| eyre!("No Member on Message!"))?
        .roles
        .iter()
        .copied()
        .collect();

    let mut response = MessageBuilder::new();
    for arg in args.iter::<String>().map(|arg| arg.unwrap()) {
        let name = arg.trim();
        if let Some(rank) = ranks.by_name(&name) {
            if user_role_ids.contains(&rank.role.id) {
                if on_leave(&rank, &mut response) {
                    user_role_ids.remove(&rank.role.id);
                }
            } else if on_join(&rank, &mut response) {
                user_role_ids.insert(rank.role.id);
            }
        } else {
            response.push("No such rank: ").push_line_safe(name);
        }
    }
    guild_id
        .edit_member(&ctx, &msg.author, |edit| edit.roles(user_role_ids))
        .await?;
    msg.reply(ctx, response.build()).await?;
    Ok(())
}

#[command]
#[description("Join a rank")]
#[min_args(1)]
#[delimiters(',')]
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
#[delimiters(',')]
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
#[delimiters(',')]
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
        for mut chunk in &ranks.member_counts().into_iter().chunks(2) {
            if let Some((name, count)) = chunk.next() {
                write!(&mut tw, "{} ({})", name, count)?;
            }
            for (name, count) in chunk {
                write!(&mut tw, "\t{} ({})", name, count)?;
            }
            writeln!(&mut tw)?;
        }
        String::from_utf8(tw.into_inner()?)?
    };

    let prefix = get_data::<ConfigKey>(ctx).await?.discord.command_prefix;
    msg.channel_id
        .send_message(&ctx, |message| {
            message.embed(|embed| {
                embed
                    .title("Ranks")
                    .footer(|footer| {
                        footer.text(format!(
                            "Use {0}join and {0}leave to change your ranks",
                            prefix
                        ))
                    })
                    .description(ellipsis_string(
                        MessageBuilder::new()
                            .push_codeblock_safe(rank_list, None)
                            .build(),
                        EMBED_DESC_LENGTH,
                    ))
            })
        })
        .await?;

    let user_ranks = ranks.of_user(&msg.author);
    msg.reply(
        ctx,
        if user_ranks.is_empty() {
            format!(
                "You currently have no ranks. Use the {}join command to join some.",
                prefix
            )
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
