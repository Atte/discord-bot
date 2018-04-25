use serenity::framework::standard::StandardFramework;

mod meta;

pub fn register(framework: StandardFramework) -> StandardFramework {
    framework.command("ping", |cmd| {
        cmd.desc("Replies with a pong").num_args(0).cmd(meta::ping)
    })
}
