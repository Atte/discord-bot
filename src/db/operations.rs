use rusqlite::{Connection, named_params};
use serenity::model::prelude::*;
use super::Result;

pub fn user_seen(conn: &Connection, user: &User) -> Result<()> {
    conn.prepare_cached("
        INSERT INTO users
        (id)
        VALUES (:id)
        ON CONFLICT (id)
        DO UPDATE SET last_seen = datetime('now')
    ")?.execute_named(named_params!{
        ":id": user.id.to_string(),
    })?;

    conn.prepare_cached("
        INSERT INTO aliases
        (user_id, username, discriminator)
        VALUES (:id, :name, :discriminator)
    ")?.execute_named(named_params!{
        ":id": user.id.to_string(),
        ":name": user.name,
        ":discriminator": format!("{:04}", user.discriminator),
    })?;

    Ok(())
}
