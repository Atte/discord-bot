PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE new_users (
    id TEXT PRIMARY KEY NOT NULL,
    first_online TEXT DEFAULT CURRENT_TIMESTAMP,
    last_online TEXT DEFAULT CURRENT_TIMESTAMP,
    first_message TEXT DEFAULT NULL,
    last_message TEXT DEFAULT NULL
) WITHOUT ROWID;
INSERT INTO new_users SELECT * FROM users;

CREATE TABLE new_usernames (
    id TEXT NOT NULL REFERENCES new_users (id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 2 and 32),
    discriminator TEXT NOT NULL CHECK (length(discriminator) = 4),
    first_online TEXT DEFAULT CURRENT_TIMESTAMP,
    last_online TEXT DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id, name, discriminator)
) WITHOUT ROWID;
INSERT INTO new_usernames SELECT * FROM usernames;

CREATE TABLE new_nicks (
    id TEXT NOT NULL REFERENCES new_users (id) ON DELETE CASCADE,
    nick TEXT NOT NULL CHECK (length(nick) <= 32),
    first_online TEXT DEFAULT CURRENT_TIMESTAMP,
    last_online TEXT DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id, nick)
) WITHOUT ROWID;
INSERT INTO new_nicks SELECT * FROM nicks;

DROP TABLE nicks;
DROP TABLE usernames;
DROP TABLE users;

ALTER TABLE new_users RENAME TO users;
ALTER TABLE new_usernames RENAME TO usernames;
ALTER TABLE new_nicks RENAME TO nicks;

COMMIT;

PRAGMA foreign_keys = ON;
