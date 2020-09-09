use super::{get_data, DbKey};
use crate::Result;
use log::info;
use mongodb::{
    bson::doc,
    options::{FindOneOptions, UpdateOptions},
};
use serenity::{
    client::Context,
    model::{guild::Member, id::RoleId},
};

const COLLECTION_NAME: &str = "sticky-roles";

pub async fn save_stickies(ctx: &Context, member: &Member) -> Result<()> {
    let collection = get_data::<DbKey>(&ctx).await?.collection(COLLECTION_NAME);
    collection
        .update_one(
            doc! {
                "user_id": member.user.id,
                "guild_id": member.guild_id,
            },
            doc! {
                "$set": {
                    "role_ids": member.roles,
                },
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
    Ok(())
}

pub async fn apply_stickies(ctx: &Context, member: &Member) -> Result<()> {
    let collection = get_data::<DbKey>(&ctx).await?.collection(COLLECTION_NAME);
    if let Some(entry) = collection
        .find_one(
            doc! {
                "user_id": member.user.id,
                "guild_id": member.guild_id,
            },
            FindOneOptions::builder()
                .projection(doc! { "role_ids": 1 })
                .build(),
        )
        .await?
    {
        let role_ids: Vec<RoleId> = entry
            .get_array("role_ids")?
            .into_iter()
            .filter_map(|i| i.as_i64().map(|i| RoleId(i as u64)))
            .collect();
        info!("Restoring sticky roles: {:?}", role_ids);

        let user_role_ids: Vec<RoleId> = member.roles.clone();
        user_role_ids.extend(role_ids);
        member.edit(&ctx, |edit| edit.roles(user_role_ids));
    }
    Ok(())
}
