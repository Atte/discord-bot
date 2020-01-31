use super::Result;
use crate::CONFIG;
use rusqlite::{named_params, Connection, OptionalExtension};
use serenity::model::prelude::*;

pub fn cache_message(conn: &Connection, message: &Message) -> Result<()> {
    conn.prepare_cached(
        "
        INSERT OR REPLACE INTO messages (id, user_id, json)
        VALUES (:id, :user_id, :json)
        ",
    )?
    .execute_named(named_params! {
        ":id": message.id.to_string(),
        ":user_id": message.author.id.to_string(),
        ":json": serde_json::to_string(&message)?,
    })?;

    conn.prepare_cached(
        "
        DELETE FROM messages
        WHERE id NOT IN (
            SELECT id FROM messages
            ORDER BY time DESC
            LIMIT :history
        )
        ",
    )?
    .execute_named(named_params! {
        ":history": CONFIG.discord.deleted_msg_cache,
    })?;

    Ok(())
}

pub fn get_message(conn: &Connection, id: MessageId) -> Result<Option<Message>> {
    if let Some(json) = conn
        .prepare_cached(
            "
            SELECT json FROM messages
            WHERE id = :id
            ",
        )?
        .query_row_named(
            named_params! {
                ":id": id.to_string(),
            },
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        Ok(serde_json::from_str(&json)?)
    } else {
        Ok(None)
    }
}
