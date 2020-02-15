use crate::CONFIG;
use rusqlite::Connection;
use std::cell::RefCell;

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

thread_local!(static THREAD_CONNECTION: RefCell<Connection> = RefCell::new(connect().expect("db connection error")));

pub fn with_db<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&Connection) -> Result<T>,
{
    THREAD_CONNECTION.with(|conn| {
        match f(&conn.borrow()) {
            Err(err) => {
                log::error!("db error: {:?}", err);
                Err(err)
            }
            result @ Ok(_) => result,
        }
    })
}
