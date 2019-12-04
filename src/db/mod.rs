use rusqlite::{Connection, NO_PARAMS};
use serenity::prelude::*;
use std::{path::Path, sync::Arc};

mod migrations;
mod operations;

pub use operations::*;

error_chain::error_chain! {
    foreign_links {
        Rusqlite(rusqlite::Error);
    }
}

pub fn connect(path: impl AsRef<Path>) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.query_row("PRAGMA journal_mode = WAL", NO_PARAMS, |_row| Ok(()))?;
    conn.execute("PRAGMA foreign_keys = ON", NO_PARAMS)?;
    migrations::apply_migrations(&conn)?;
    Ok(conn)
}

pub struct DatabaseKey;

impl TypeMapKey for DatabaseKey {
    type Value = Arc<Mutex<Connection>>;
}

pub fn with_db<T, F>(context: &Context, f: F) -> Option<T>
where
    F: FnOnce(&Connection) -> T,
{
    let mut data = context.data.write();
    data.get_mut::<DatabaseKey>().map(|lock| f(&lock.lock()))
}
