use rusqlite::{Connection, NO_PARAMS};
use log::info;
use super::Result;

const MIGRATION_STEPS: u32 = 1;

fn migrate_step(conn: &Connection, step: u32) -> Result<()> {
    info!("Running migration step {}", step);
    match step {
        0 => {
            conn.execute_batch("
                BEGIN;

                CREATE TABLE users (
                    id TEXT PRIMARY KEY NOT NULL,
                    first_seen TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    last_seen TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                ) WITHOUT ROWID;

                CREATE TABLE aliases (
                    id INTEGER PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
                    username TEXT NOT NULL CHECK (length(username) BETWEEN 2 and 32),
                    discriminator TEXT NOT NULL CHECK (length(discriminator) = 4),
                    UNIQUE (username, discriminator) ON CONFLICT REPLACE
                );

                COMMIT;
            ")?;
        }
        _ => unreachable!()
    }
    Ok(())
}

pub fn apply_migrations(conn: &Connection) -> Result<(u32, u32)> {
    let initial: u32 = conn.query_row("SELECT user_version FROM pragma_user_version", NO_PARAMS, |row| row.get(0))?;
    for step in initial..MIGRATION_STEPS {
        migrate_step(&conn, step)?;
        conn.execute_batch(&format!("PRAGMA user_version = {}", step + 1))?;
    }
    Ok((initial, MIGRATION_STEPS - 1))
}
