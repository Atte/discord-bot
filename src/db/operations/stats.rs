use super::Result;
use rusqlite::{named_params, Connection};
use serenity::model::prelude::*;

pub fn user_online(conn: &Connection, user: &User) -> Result<()> {
    conn.prepare_cached(
        "
        INSERT INTO users (id)
        VALUES (:id)
        ON CONFLICT (id)
        DO UPDATE SET
            first_online = COALESCE(first_online, datetime('now')),
            last_online = datetime('now')
        ",
    )?
    .execute_named(named_params! {
        ":id": user.id.to_string(),
    })?;

    conn.prepare_cached(
        "
        INSERT INTO usernames (id, name, discriminator)
        VALUES (:id, :name, :discriminator)
        ON CONFLICT (id, name, discriminator)
        DO UPDATE SET
            first_online = COALESCE(first_online, datetime('now')),
            last_online = datetime('now')
        ",
    )?
    .execute_named(named_params! {
        ":id": user.id.to_string(),
        ":name": user.name,
        ":discriminator": format!("{:04}", user.discriminator),
    })?;

    Ok(())
}

pub fn member_online(conn: &Connection, member: &Member) -> Result<()> {
    let user = member.user.read();
    user_online(conn, &user)?;

    if let Some(ref nick) = member.nick {
        conn.prepare_cached(
            "
            INSERT INTO nicks (id, nick)
            VALUES (:id, :nick)
            ON CONFLICT (id, nick)
            DO UPDATE SET
                first_online = COALESCE(first_online, datetime('now')),
                last_online = datetime('now')
            ",
        )?
        .execute_named(named_params! {
            ":id": user.id.to_string(),
            ":nick": nick,
        })?;
    }

    Ok(())
}

pub fn user_offline(conn: &Connection, user: &User) -> Result<()> {
    conn.prepare_cached(
        "
        INSERT OR IGNORE INTO users (id, first_online, last_online)
        VALUES (:id, NULL, NULL)
        ",
    )?
    .execute_named(named_params! {
        ":id": user.id.to_string(),
    })?;

    conn.prepare_cached(
        "
        INSERT OR IGNORE INTO usernames (id, name, discriminator, first_online, last_online)
        VALUES (:id, :name, :discriminator, NULL, NULL)
        ",
    )?
    .execute_named(named_params! {
        ":id": user.id.to_string(),
        ":name": user.name,
        ":discriminator": format!("{:04}", user.discriminator),
    })?;

    Ok(())
}

pub fn member_offline(conn: &Connection, member: &Member) -> Result<()> {
    let user = member.user.read();
    user_offline(conn, &user)?;

    if let Some(ref nick) = member.nick {
        conn.prepare_cached(
            "
            INSERT OR IGNORE INTO nicks (id, nick, first_online, last_online)
            VALUES (:id, :nick, NULL, NULL)
            ",
        )?
        .execute_named(named_params! {
            ":id": user.id.to_string(),
            ":nick": nick,
        })?;
    }

    Ok(())
}

pub fn user_message(conn: &Connection, user: UserId) -> Result<()> {
    conn.prepare_cached(
        "
        UPDATE users SET
            first_message = COALESCE(first_message, datetime('now')),
            last_message = datetime('now')
        WHERE id = :id
        ",
    )?
    .execute_named(named_params! {
        ":id": user.to_string(),
    })?;

    Ok(())
}

pub fn channel_exists(conn: &Connection, channel: &GuildChannel) -> Result<()> {
    conn.prepare_cached(
        "
        INSERT INTO channels (id, guild_id, name, nsfw)
        VALUES (:id, :guild_id, :name, :nsfw)
        ON CONFLICT (id)
        DO UPDATE SET last_exists = datetime('now')
        ",
    )?
    .execute_named(named_params! {
        ":id": channel.id.to_string(),
        ":guild_id": channel.guild_id.to_string(),
        ":name": channel.name,
        ":nsfw": channel.is_nsfw(),
    })?;

    Ok(())
}
