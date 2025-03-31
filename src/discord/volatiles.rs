use super::{ConfigKey, DbKey, get_data};
use bson::doc;
use chrono::Utc;
use color_eyre::eyre::Result;
use serenity::all::{Context, Message};
use tokio::join;

pub const COLLECTION_NAME: &str = "volatiles";

pub async fn enforce(ctx: &Context, msg: &Message) -> Result<()> {
    let config = get_data::<ConfigKey>(ctx).await?;
    let Some(config) = config
        .discord
        .volatiles
        .iter()
        .find(|volatile| volatile.channel == msg.channel_id)
    else {
        // not enabled for this channel
        return Ok(());
    };

    let member = msg.member(ctx).await?;
    let collection = get_data::<DbKey>(ctx).await?.collection(COLLECTION_NAME);

    if collection
        .find_one(doc! {
            "channel.id": msg.channel_id.to_string(),
            "user.id": msg.author.id.to_string()
        })
        .await?
        .is_some()
    {
        log::info!("Repeat volatile message from {}", member.display_name());
        let (delete_result, add_result) = join!(msg.delete(ctx), member.add_role(ctx, config.role));
        delete_result?;
        add_result?;
        return Ok(());
    }

    log::info!("First volatile message from {}", member.display_name());
    let (add_result, db_result) = join!(
        member.add_role(ctx, config.role),
        collection
            .insert_one(doc! {
                "time": Utc::now(),
                "channel": {
                    "id": msg.channel_id.to_string(),
                },
                "user": {
                    "id": msg.author.id.to_string(),
                    "name": &msg.author.name,
                    "nick": member.display_name()
                }
            })
            .into_future(),
    );
    add_result?;
    db_result?;

    Ok(())
}
