use crate::discord::{get_data, stats::Stats, DbKey, STATS_COLLECTION_NAME};
use mongodb::bson::doc;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
};

#[command]
#[description("Show stats of the calling user")]
#[num_args(0)]
async fn stats(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let collection = get_data::<DbKey>(ctx)
        .await?
        .collection::<Stats>(STATS_COLLECTION_NAME);

    if let Some(Stats::Member {
        message_count,
        emoji_count,
        nicks,
        ..
    }) = collection
        .find_one(doc! {"id": msg.author.id.to_string()}, None)
        .await?
    {
        msg.reply(ctx, format!(
            "You have sent {message_count} messages containing a total of {emoji_count} emotes. You have used {} different nickname{}.",
            nicks.len(),
            if nicks.len() == 1 { "" } else { "s" }),
        ).await?;
    } else {
        msg.reply(ctx, "You have no stats! How did you even do that?")
            .await?;
    }

    Ok(())
}
