use serenity::{
    client::Context,
    framework::standard::{
        help_commands,
        macros::{command, group, help},
        Args, CommandGroup, CommandResult, HelpOptions,
    },
    model::{channel::Message, id::UserId},
};
use std::collections::HashSet;

mod ranks;
pub use ranks::{JOIN_COMMAND, LEAVE_COMMAND, RANKS_COMMAND, RANK_COMMAND};

mod roll;
pub use roll::ROLL_COMMAND;

mod gib;
pub use gib::GIB_COMMAND;
pub use gib::{derpibooru_embed, derpibooru_search};

mod emotes;
pub use emotes::EMOTES_GROUP;

#[cfg(feature = "battlegrounds")]
mod battlegrounds;
#[cfg(feature = "battlegrounds")]
pub use battlegrounds::BATTLEGROUNDS_GROUP;

//mod stats;
//pub use stats::STATS_COMMAND;

#[group]
#[only_in(guilds)]
#[commands(gib)]
pub struct Horse;

#[group]
#[only_in(guilds)]
#[commands(join, leave, rank, ranks)]
pub struct Ranks;

#[group]
#[commands(roll, ping)]
pub struct Misc;

#[command]
#[num_args(0)]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;
    Ok(())
}

#[help]
#[strikethrough_commands_tip_in_guild("")]
async fn help_command(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners).await?;
    Ok(())
}
