use rusqlite::{Connection, NO_PARAMS};
use serenity::prelude::*;
use std::{path::Path, sync::Arc};

mod migrations;
mod operations;

pub use operations::*;

error_chain::error_chain! {
    foreign_links {
        Io(std::io::Error);
        Rusqlite(rusqlite::Error);
    }
}

pub struct DatabaseKey;

impl TypeMapKey for DatabaseKey {
    type Value = Arc<Mutex<Connection>>;
}

pub fn connect(path: impl AsRef<Path>) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute("PRAGMA foreign_keys = ON", NO_PARAMS)?;
    migrations::apply_migrations(&conn)?;
    Ok(conn)
}

pub fn with_db<F>(context: &Context, f: F)
where
    F: FnOnce(&Connection) -> Result<()>,
{
    let mut data = context.data.write();
    if let Some(Err(err)) = data.get_mut::<DatabaseKey>().map(|lock| f(&lock.lock())) {
        log::error!("db error: {}", err);
    }
}
