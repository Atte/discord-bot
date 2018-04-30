use super::super::util::{guild_from_message, use_emoji};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;
use serenity::CACHE;

fn get_ranks(guild: &Guild) -> Result<Vec<(&Role, Vec<&Member>)>, SerenityError> {
    let bot = guild
        .members
        .values()
        .find(|member| member.user.read().id == CACHE.read().user.id)
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

command!(list(_context, message) {
    let (reply, rank_text) = if let Some(guild) = guild_from_message(&message) {
        let guild = guild.read();
        let ranks = get_ranks(&guild)?;
        if ranks.is_empty() {
            (Some("There are no ranks on the server!".to_owned()), None)
        } else {
            let rank_text = {
                let longest_name = ranks.iter().map(|(rank, _members)| rank.name.len()).max().expect("Impossible empty list");
                let mut desc_lines: Vec<String> = ranks.iter().map(|(rank, members)| {
                    format!("{:w$}{:3} member{}", format!("{}:", rank.name), members.len(), if members.len() == 1 { "" } else { "s" }, w = longest_name + 2)
                }).collect();
                desc_lines.sort();
                Some(format!("```ldif\n{}```", desc_lines.join("\n")))
            };
            let reply = guild.members.get(&message.author.id).and_then(|user| {
                let mut rank_names: Vec<String> = ranks.into_iter().filter_map(|(rank, _members)| if user.roles.contains(&rank.id) { Some(format!("**{}**", rank.name)) } else { None }).collect();
                if !rank_names.is_empty() {
                    rank_names.sort();
                    Some(format!("Your current ranks are {}", rank_names.join(", ")))
                } else {
                    None
                }
            });
            (reply, rank_text)
        }
    } else {
        (Some("Rank listing is only available on a server!".to_owned()), None)
    };
    if let Some(rank_text) = rank_text {
        message.channel_id.send_message(|msg| {
            msg.embed(|e|
                e.colour(Colour::blue())
                .title("Available ranks")
                .description(rank_text)
                .footer(|f| f.text("Use the !rank command to join/leave a rank."))
            )
        })?;
    }
    if let Some(reply) = reply {
        message.reply(&reply)?;
    }
});

command!(joinleave(_context, message, args) {
    let rankname: String = args.single::<String>()?;
    let response = if let Some(guild) = guild_from_message(&message) {
        let mut guild = guild.write();
        let leave_emoji = use_emoji(Some(&guild), "aj05");
        let join_emoji = use_emoji(Some(&guild), "twiyay");
        if let Some(rank_id) = get_ranks(&guild)?.into_iter().find(|(rank, _members)| rank.name.to_lowercase() == rankname.to_lowercase()).map(|(rank, _members)| rank.id) {
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
            format!("There is no such rank. {}", use_emoji(Some(&guild), "lyou"))
        }
    } else {
        "Rank joining/leaving is only available on a server!".to_owned()
    };
    message.reply(&response)?;
});
