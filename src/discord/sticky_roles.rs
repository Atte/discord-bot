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
                "user_id": member.user.id.0,
                "guild_id": member.guild_id.0,
            },
            doc! {
                "$set": {
                    "role_ids": member.roles.iter().map(|role| role.0).collect::<Vec<u64>>(),
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
                "user_id": member.user.id.0,
                "guild_id": member.guild_id.0,
            },
            FindOneOptions::builder()
                .projection(doc! { "role_ids": 1 })
                .build(),
        )
        .await?
    {
        #[allow(clippy::cast_sign_loss)]
        let role_ids: Vec<RoleId> = entry
            .get_array("role_ids")?
            .iter()
            .filter_map(|i| i.as_i64().map(|i| RoleId(i as u64)))
            .collect();
        info!("Restoring sticky roles: {:?}", role_ids);

        let mut user_role_ids: Vec<RoleId> = member.roles.clone();
        user_role_ids.extend(role_ids);
        member.edit(&ctx, |edit| edit.roles(user_role_ids)).await?;
    }
    Ok(())
}
