use super::super::util;
use serenity::framework::standard::{Args, CommandError};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;

fn get_ranks(guild: &Guild) -> Result<Vec<(&Role, Vec<&Member>)>, SerenityError> {
    let bot = guild
        .members
        .values()
        .find(|member| member.user.read().id == util::uid())
        .ok_or_else(|| SerenityError::Other("Can't find bot as a guild member"))?;
    trace!(
        "Found bot as a guild member (bot has {} roles)",
        bot.roles.len()
    );

    let max_pos = guild
        .roles
        .values()
        .filter_map(|role| {
            if !role.name.starts_with('@') && bot.roles.contains(&role.id) {
                Some(role.position)
            } else {
                None
            }
        })
        .min()
        .ok_or_else(|| SerenityError::Other("Can't find bot roles"))?;
    trace!("Rank positions are less than {}", max_pos);

    let ranks: Vec<_> = guild
        .roles
        .values()
        .filter_map(|role| {
            if !role.name.starts_with('@') && role.position < max_pos {
                Some(role)
            } else {
                None
            }
        })
        .collect();
    trace!("Found {} ranks", ranks.len());

    /*
    if let Some(rank) = ranks.iter().find(|rank| !rank.permissions.is_empty()) {
        trace!("Removing permissions from rank {}", rank.name);
        rank.edit(|role| role.permissions(Permissions::empty()))?;
    }
    */

    Ok(ranks
        .into_iter()
        .map(|rank| {
            let rank_members: Vec<_> = guild
                .members
                .values()
                .filter_map(|member| {
                    if member.roles.contains(&rank.id) {
                        Some(member)
                    } else {
                        None
                    }
                })
                .collect();
            (rank, rank_members)
        })
        .collect())
}

pub fn list(_: &mut Context, message: &Message, _: Args) -> Result<(), CommandError> {
    let (reply, rank_text) = if let Some(guild) = util::guild_from_message(&message) {
        let guild = guild.read();
        let ranks = get_ranks(&guild)?;
        if ranks.is_empty() {
            (Some("There are no ranks on the server!".to_owned()), None)
        } else {
            let rank_text = {
                let longest_name = ranks
                    .iter()
                    .map(|(rank, _members)| rank.name.len())
                    .max()
                    .expect("Impossible empty list");
                let mut desc_lines: Vec<String> = ranks
                    .iter()
                    .map(|(rank, members)| {
                        format!(
                            "{:w$}{:3}",
                            format!("{}:", rank.name),
                            members.len(),
                            w = longest_name + 1
                        )
                    })
                    .collect();
                desc_lines.sort();
                Some(format!("```ldif\n{}```", desc_lines.join("\n")))
            };
            let reply = guild.members.get(&message.author.id).and_then(|user| {
                let mut rank_names: Vec<String> = ranks
                    .into_iter()
                    .filter_map(|(rank, _members)| {
                        if user.roles.contains(&rank.id) {
                            Some(format!("**{}**", rank.name))
                        } else {
                            None
                        }
                    })
                    .collect();
                if rank_names.is_empty() {
                    None
                } else {
                    rank_names.sort();
                    Some(format!("Your current ranks are {}", rank_names.join(", ")))
                }
            });
            (reply, rank_text)
        }
    } else {
        (
            Some("Rank listing is only available on a server!".to_owned()),
            None,
        )
    };
    if let Some(rank_text) = rank_text {
        message.channel_id.send_message(|msg| {
            msg.embed(|e| {
                e.colour(Colour::BLUE)
                    .title("Available ranks")
                    .description(rank_text)
                    .footer(|f| f.text("Use the !rank command to join/leave a rank."))
            })
        })?;
    }
    if let Some(reply) = reply {
        message.reply(&reply)?;
    }
    Ok(())
}

pub fn joinleave(_: &mut Context, message: &Message, args: Args) -> Result<(), CommandError> {
    let rankname = args.full().trim();
    let response = if let Some(guild) = util::guild_from_message(&message) {
        let mut guild = guild.write();
        let leave_emoji = util::use_emoji(Some(&guild), "aj05");
        let join_emoji = util::use_emoji(Some(&guild), "twiyay");
        if let Some(rank_id) = get_ranks(&guild)?
            .into_iter()
            .find(|(rank, _members)| rank.name.to_lowercase() == rankname.to_lowercase())
            .map(|(rank, _members)| rank.id)
        {
            if let Some(mut user) = guild.members.get_mut(&message.author.id) {
                let is_current = user.roles.contains(&rank_id);
                if is_current {
                    user.remove_role(rank_id)?;
                    format!("You have left **{}**! {}", rankname, leave_emoji)
                } else {
                    user.add_role(rank_id)?;
                    format!("You have joined **{}**! {}", rankname, join_emoji)
                }
            } else {
                "You are not on the server? WTF?".to_owned()
            }
        } else {
            format!(
                "There is no such rank. {}",
                util::use_emoji(Some(&guild), "lyou")
            )
        }
    } else {
        "Rank joining/leaving is only available on a server!".to_owned()
    };
    message.reply(&response)?;
    Ok(())
}
