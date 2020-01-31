use super::Result;
use rusqlite::{named_params, types::Value, Connection};
use serenity::model::prelude::*;
use std::rc::Rc;

pub fn set_sticky_roles(
    conn: &Connection,
    user: UserId,
    roles: impl IntoIterator<Item = RoleId>,
) -> Result<()> {
    let ids = Rc::new(
        roles
            .into_iter()
            .map(|id| Value::from(id.to_string()))
            .collect(),
    );

    conn.prepare_cached(
        "
        DELETE FROM sticky_roles
        WHERE user_id = :user_id AND role_id NOT IN (SELECT value FROM rarray(:role_ids))
        ",
    )?
    .execute_named(named_params! {
        ":user_id": user.to_string(),
        ":role_ids": &ids,
    })?;

    conn.prepare_cached(
        "
        INSERT OR IGNORE INTO sticky_roles (user_id, role_id)
        SELECT :user_id, value FROM rarray(:role_ids)
        ",
    )?
    .execute_named(named_params! {
        ":user_id": user.to_string(),
        ":role_ids": &ids,
    })?;

    Ok(())
}

pub fn get_sticky_roles(conn: &Connection, user: UserId) -> Result<Vec<RoleId>> {
    let ids: rusqlite::Result<Vec<String>> = conn
        .prepare_cached(
            "
            SELECT role_id FROM sticky_roles
            WHERE user_id = :user_id
            ",
        )?
        .query_map_named(
            named_params! {
                ":user_id": user.to_string(),
            },
            |row| row.get(0),
        )?
        .collect();

    Ok(ids?
        .into_iter()
        .filter_map(|id| id.parse().ok().map(RoleId))
        .collect())
}
