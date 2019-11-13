use diesel::prelude::*;
use serenity::prelude::*;
use std::sync::Arc;

pub mod models;
pub mod schema;

pub mod preludes {
    pub mod users {
        pub use super::super::models::{NewUser, User};
        pub use super::super::schema::users::dsl::*;
    }

    pub mod aliases {
        pub use super::super::models::{Alias, NewAlias};
        pub use super::super::schema::aliases::dsl::*;
    }

    #[allow(clippy::doc_markdown)]
    pub mod functions {
        use diesel::sql_types::Text;
        diesel::sql_function!(fn datetime(spec: Text) -> Text);
    }
}

error_chain::error_chain! {
    foreign_links {
        Connection(diesel::ConnectionError);
        Database(diesel::result::Error);
        Migrations(diesel_migrations::RunMigrationsError);
    }
}

embed_migrations!("migrations");

pub fn connect(path: impl AsRef<str>) -> Result<SqliteConnection> {
    let conn = SqliteConnection::establish(path.as_ref())?;
    conn.execute("PRAGMA foreign_keys = ON;")?;
    embedded_migrations::run_with_output(&conn, &mut std::io::stderr())?;
    Ok(conn)
}

pub struct DatabaseKey;

impl TypeMapKey for DatabaseKey {
    type Value = Arc<Mutex<SqliteConnection>>;
}

pub fn with_db<T, F>(context: &Context, f: F) -> Option<T>
where
    F: FnOnce(&SqliteConnection) -> T,
{
    let mut data = context.data.write();
    data.get_mut::<DatabaseKey>().map(|lock| f(&lock.lock()))
}
