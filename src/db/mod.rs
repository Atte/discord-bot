use crate::CONFIG;
use rusqlite::{Connection, NO_PARAMS};
use serenity::prelude::*;
use std::sync::Arc;

mod migrations;
mod operations;

pub use migrations::apply_migrations;
pub use operations::*;

error_chain::error_chain! {
    foreign_links {
        Io(std::io::Error);
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

pub fn connect() -> Result<Connection> {
    let conn = Connection::open(CONFIG.db.to_string())?;
    rusqlite::vtab::array::load_module(&conn)?;
    conn.execute("PRAGMA foreign_keys = ON", NO_PARAMS)?;
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
