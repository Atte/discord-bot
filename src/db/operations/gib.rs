use super::Result;
use crate::CONFIG;
use rusqlite::{named_params, Connection};

pub fn gib_seen(conn: &Connection, id: u32) -> Result<()> {
    conn.prepare_cached(
        "
        INSERT OR REPLACE INTO gib_seen (id)
        VALUES (:id)
        ",
    )?
    .execute_named(named_params! {
        ":id": id,
    })?;

    conn.prepare_cached(
        "
        DELETE FROM gib_seen
        WHERE id NOT IN (
            SELECT id FROM gib_seen
            ORDER BY time DESC
            LIMIT :history
        )
        ",
    )?
    .execute_named(named_params! {
        ":history": CONFIG.gib.history,
    })?;

    Ok(())
}

pub fn gib_is_seen(conn: &Connection, id: u32) -> Result<bool> {
    Ok(conn
        .prepare_cached(
            "
            SELECT id FROM gib_seen
            WHERE id = :id
            LIMIT 1
            ",
        )?
        .query_named(named_params! {
            ":id": id,
        })?
        .next()?
        .is_some())
}
