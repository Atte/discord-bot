use super::super::util::guild_from_message;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;
use serenity::CACHE;

fn get_ranks(guild: &Guild) -> Result<Vec<(&Role, Vec<&Member>)>, SerenityError> {
    let bot = guild.members
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
            let rank_members: Vec<_> = guild.members
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
    if let Some(guild) = guild_from_message(&message) {
        let guild = guild.read();
        let ranks = get_ranks(&guild)?;
        if ranks.is_empty() {
            message.reply("There are no ranks on the server!")?;
        } else {
            let longest_name = ranks.iter().map(|(rank, _members)| rank.name.len()).min().expect("Impossible empty list");
            let mut desc_lines: Vec<String> = ranks.into_iter().map(|(rank, members)| format!("{:w$}{}", format!("{}:", rank.name), members.len(), w = longest_name + 2)).collect();
            desc_lines.sort();
            message.channel_id.send_message(|msg| {
                msg.embed(|e|
                    e.colour(Colour::blue())
                    .title("Ranks")
                    .description(format!("```ldif\n{}```", desc_lines.join("\n")))
                    .footer(|f| f.text("Use the !rank command to join/leave a rank."))
                )
            })?;
        }
    } else {
        message.reply("Rank listing is only available on a server!")?;
    }
});

command!(joinleave(_context, message, args) {
    let rankname: String = args.single::<String>()?.to_lowercase();
    let response = if let Some(guild) = guild_from_message(&message) {
        let mut guild = guild.write();
        if let Some(role_id) = guild.role_by_name(&rankname).map(|role| role.id) {
            let is_rank = get_ranks(&guild)?.into_iter().any(|(rank, _members)| rank.id == role_id);
            if let Some(mut user) = guild.members.get_mut(&message.author.id) {
                let is_current = user.roles.contains(&role_id);

                if is_rank {
                    if is_current {
                        user.remove_role(role_id)?;
                        format!("You have left **{}**!", rankname)
                    } else {
                        user.add_role(role_id)?;
                        format!("You have joined **{}**!", rankname)
                    }
                } else {
                    "There is no such rank!".to_owned()
                }
            } else {
                "You are not on the server? WTF?".to_owned()
            }
        } else {
            "There is no such rank!".to_owned()
        }
    } else {
        "Rank joining/leaving is only available on a server!".to_owned()
    };
    message.reply(&response)?;
});
