CREATE TABLE users (
    id BLOB PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX users_email_index ON users (email);

CREATE TABLE tags (
    id BLOB PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX tags_name_index ON tags (name);

CREATE TABLE users_tags (
    user_id BLOB NOT NULL,
    tag_id BLOB NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags (id) ON DELETE CASCADE
) STRICT;

CREATE INDEX users_tags_user_id_index ON users_tags (user_id);
CREATE INDEX users_tags_tag_id_index ON users_tags (tag_id);

CREATE TABLE passkeys (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    passkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_used_at INTEGER,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) STRICT;

CREATE INDEX passkeys_user_id_index ON passkeys (user_id);

CREATE TABLE passkey_registrations (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    email TEXT NOT NULL,
    registration TEXT NOT NULL,
    created_at INTEGER NOT NULL
    -- no foreign key because the user may not exist yet
) STRICT;

CREATE UNIQUE INDEX passkey_registrations_id_index ON passkey_registrations (id);
