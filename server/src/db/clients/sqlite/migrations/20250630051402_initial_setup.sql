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
    credential_id BLOB NOT NULL,
    display_name TEXT,
    created_at INTEGER NOT NULL,
    last_used_at INTEGER,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) STRICT;

CREATE INDEX passkeys_user_id_index ON passkeys (user_id);
CREATE UNIQUE INDEX passkeys_credential_id_index ON passkeys (credential_id);

CREATE TABLE passkey_registrations (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    email TEXT NOT NULL,
    registration TEXT NOT NULL,
    created_at INTEGER NOT NULL
    -- no foreign key because the user may not exist yet
) STRICT;

CREATE TABLE passkey_authentications (
    id BLOB PRIMARY KEY,
    email TEXT,
    state TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (email) REFERENCES users (email) ON DELETE CASCADE
) STRICT;

CREATE TABLE sessions (
    id_hash BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    state INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) STRICT;

CREATE INDEX sessions_user_id_index ON sessions (user_id);
