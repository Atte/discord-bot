use log::trace;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::Colour,
};

fn get_ranks<'a>(
    context: &Context,
    guild: &'a Guild,
) -> Result<Vec<(&'a Role, Vec<&'a Member>)>, SerenityError> {
    let uid = context.cache.read().user.id;
    let bot = guild
        .members
        .values()
        .find(|member| member.user.read().id == uid)
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
        .filter(|role| !role.name.starts_with('@') && role.position < max_pos)
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
                .filter(|member| member.roles.contains(&rank.id))
                .collect();
            (rank, rank_members)
        })
        .collect())
}

#[command]
#[description("Lists all available ranks, as well as the current user's active ones.")]
#[num_args(0)]
#[only_in("guilds")]
pub fn ranks(context: &mut Context, message: &Message, _: Args) -> CommandResult {
    let (reply, rank_text) = if let Some(guild) = message.guild(&context) {
        let guild = guild.read();
        let ranks = get_ranks(&context, &guild)?;
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
        message.channel_id.send_message(&context, |msg| {
            msg.embed(|e| {
                e.colour(Colour::BLUE)
                    .title("Available ranks")
                    .description(rank_text)
                    .footer(|f| f.text("Use the !rank command to join/leave a rank."))
            })
        })?;
    }
    if let Some(reply) = reply {
        message.reply(&context, &reply)?;
    }
    Ok(())
}

#[command]
#[description("Joins/leaves a rank.")]
#[usage("rankname")]
#[num_args(1)]
#[only_in("guilds")]
pub fn rank(context: &mut Context, message: &Message, args: Args) -> CommandResult {
    let rankname = args.message().trim();
    let response = if let Some(guild) = message.guild(&context) {
        let mut guild = guild.write();
        if let Some(rank_id) =
            get_ranks(&context, &guild)?
                .into_iter()
                .find_map(|(rank, _members)| {
                    if rank.name.to_lowercase() == rankname.to_lowercase() {
                        Some(rank.id)
                    } else {
                        None
                    }
                })
        {
            if let Some(user) = guild.members.get_mut(&message.author.id) {
                let is_current = user.roles.contains(&rank_id);
                if is_current {
                    user.remove_role(&context, rank_id)?;
                    format!("You have left **{}**! <:aj05:310579190770434050>", rankname)
                } else {
                    user.add_role(&context, rank_id)?;
                    format!(
                        "You have joined **{}**! <:twiyay:310582814565466112>",
                        rankname
                    )
                }
            } else {
                "You are not on the server? WTF?".to_owned()
            }
        } else {
            "There is no such rank. <:lyou:350623520494977035>".to_owned()
        }
    } else {
        "Rank joining/leaving is only available on a server!".to_owned()
    };
    message.reply(&context, &response)?;
    Ok(())
}
