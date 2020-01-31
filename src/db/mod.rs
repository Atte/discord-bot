use crate::CONFIG;
use rusqlite::Connection;
use serenity::prelude::*;
use std::sync::Arc;

mod migrations;
mod operations;

pub use migrations::apply_migrations;
pub use operations::*;

error_chain::error_chain! {
    foreign_links {
        Io(std::io::Error);
        Json(serde_json::Error);
        Rusqlite(rusqlite::Error);
    }

    errors {
        NoDatabaseHandle {
            description("no database handle available")
        }
    }
}

pub struct DatabaseKey;

impl TypeMapKey for DatabaseKey {
    type Value = Arc<Mutex<Connection>>;
}

#[inline]
fn tracer(s: &str) {
    let words: Vec<&str> = s.trim().split_ascii_whitespace().collect();
    log::trace!("SQL({}): {}", thread_id::get(), words.join(" "));
}

pub fn connect() -> Result<Connection> {
    let mut conn = Connection::open(CONFIG.db.to_string())?;
    rusqlite::vtab::array::load_module(&conn)?;
    conn.trace(Some(tracer));
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
    ",
    )?;
    Ok(conn)
}

pub fn with_db<F, T>(context: &Context, f: F) -> Result<T>
where
    F: FnOnce(&Connection) -> Result<T>,
{
    let mut data = context.data.write();
    match data.get_mut::<DatabaseKey>().map(|lock| f(&lock.lock())) {
        None => {
            log::error!("lost db handle!");
            Err(ErrorKind::NoDatabaseHandle.into())
        }
        Some(Err(err)) => {
            log::error!("db error: {:?}", err);
            Err(err)
        }
        Some(result @ Ok(_)) => result,
    }
}
