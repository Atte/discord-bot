use super::schema::*;

#[derive(Debug, Queryable)]
pub struct User {
    pub id: String,
    pub first_seen: String,
    pub last_seen: String,
}

#[derive(Debug, Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub id: &'a str,
    pub last_seen: &'a str,
}

#[derive(Debug, Queryable)]
pub struct Alias {
    id: u64,
    pub user_id: String,
    pub username: String,
    pub discriminator: String,
}

#[derive(Debug, Insertable)]
#[table_name = "aliases"]
pub struct NewAlias<'a> {
    pub user_id: &'a str,
    pub username: &'a str,
    pub discriminator: &'a str,
}
