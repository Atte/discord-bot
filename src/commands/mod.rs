use serenity::framework::standard::StandardFramework;

mod derp;
mod misc;
mod ranks;

pub fn register(framework: StandardFramework) -> StandardFramework {
    framework
        .command("ping", |cmd| {
            cmd.desc("Replies with a pong.").num_args(0).cmd(misc::ping)
        })
        .command("ranks", |cmd| {
            cmd.desc("Lists all available ranks, as well as the current user's active ones.")
                .num_args(0)
                .guild_only(true)
                .cmd(ranks::list)
        })
        .command("rank", |cmd| {
            cmd.desc("Joins/leaves a rank.")
                .usage("rankname")
                .num_args(1)
                .guild_only(true)
                .cmd(ranks::joinleave)
        })
        .command("roll", |cmd| {
            cmd.desc("Rolls dice.").usage("1d6 + 2d20").cmd(misc::roll)
        })
        .command("info", |cmd| {
            cmd.desc("Shows information about the bot.")
                .num_args(0)
                .cmd(misc::info)
        })
        .command("gib", |cmd| {
            cmd.desc("Gibs pics from derpibooru.").cmd(derp::gib)
        })
}
