use super::{DbKey, get_data};
use color_eyre::eyre::{Result, eyre};
use itertools::Itertools;
use log::info;
use mongodb::bson::{Document, doc};
use serenity::{
    all::EditMember,
    client::Context,
    model::{guild::Member, id::RoleId},
};
use std::collections::HashSet;

const COLLECTION_NAME: &str = "sticky-roles";

pub async fn save_stickies(ctx: &Context, member: &Member) -> Result<()> {
    let collection = get_data::<DbKey>(ctx)
        .await?
        .collection::<Document>(COLLECTION_NAME);
    collection
        .update_one(
            doc! {
                "user_id": member.user.id.to_string(),
                "guild_id": member.guild_id.to_string(),
            },
            doc! {
                "$set": {
                    "role_ids": member.roles.iter().map(ToString::to_string).collect::<Vec<_>>(),
                },
            },
        )
        .upsert(true)
        .await?;
    Ok(())
}

pub async fn apply_stickies(ctx: &Context, member: &mut Member) -> Result<bool> {
    let collection = get_data::<DbKey>(ctx)
        .await?
        .collection::<Document>(COLLECTION_NAME);
    if let Some(entry) = collection
        .find_one(doc! {
            "user_id": member.user.id.to_string(),
            "guild_id": member.guild_id.to_string(),
        })
        .projection(doc! { "role_ids": 1 })
        .await?
    {
        let current_user_id = ctx.cache.current_user().id;

        let guild = member
            .guild_id
            .to_guild_cached(ctx)
            .ok_or_else(|| eyre!("Guild not found!"))?
            .clone();

        let bot_roles: HashSet<RoleId> = guild
            .member(&ctx, current_user_id)
            .await?
            .roles(ctx)
            .ok_or_else(|| eyre!("Roles for bot not found!"))?
            .into_iter()
            .map(|role| role.id)
            .collect();
        let guild_role_ids: HashSet<RoleId> = guild
            .roles
            .values()
            .sorted_by_key(|role| role.position)
            .rev()
            .take_while(|role| !bot_roles.contains(&role.id))
            .map(|role| role.id)
            .collect();

        #[allow(clippy::cast_sign_loss)]
        let role_ids: Vec<RoleId> = entry
            .get_array("role_ids")?
            .iter()
            .filter_map(|i| i.as_str().and_then(|s| s.parse().ok()).map(RoleId::new))
            .filter(|id| guild_role_ids.contains(id))
            .collect();

        if !role_ids.is_empty() {
            info!("Restoring roles: {role_ids:?}");

            let mut user_role_ids: Vec<RoleId> = member.roles.clone();
            user_role_ids.extend(role_ids);
            member
                .edit(&ctx, EditMember::new().roles(user_role_ids))
                .await?;

            return Ok(true);
        }
    }
    Ok(false)
}
