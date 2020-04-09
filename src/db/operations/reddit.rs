use super::Result;
use rusqlite::{named_params, types::Value, Connection};
use std::rc::Rc;

pub fn reddit_seen(conn: &Connection, ids: impl IntoIterator<Item = String>) -> Result<()> {
    conn.prepare_cached(
        "
        INSERT OR IGNORE INTO reddit_seen (id)
        SELECT value FROM rarray(:ids)
        ",
    )?
    .execute_named(named_params! {
        ":ids": Rc::new(ids.into_iter().map(Value::from).collect::<Vec<_>>()),
    })?;

    Ok(())
}

pub fn reddit_contains_unseen(
    conn: &Connection,
    ids: impl IntoIterator<Item = String>,
) -> Result<bool> {
    Ok(conn
        .prepare_cached(
            "
            SELECT value FROM rarray(:ids)
            WHERE value NOT IN (SELECT id FROM reddit_seen)
            LIMIT 1
            ",
        )?
        .query_named(named_params! {
            ":ids": Rc::new(ids.into_iter().map(Value::from).collect::<Vec<_>>()),
        })?
        .next()?
        .is_some())
}
