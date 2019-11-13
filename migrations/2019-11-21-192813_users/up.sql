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
