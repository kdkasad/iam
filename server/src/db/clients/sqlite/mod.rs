use std::env::VarError;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteSynchronous},
};
use uuid::Uuid;

use crate::{
    db::interface::{DatabaseClient, DatabaseError},
    models::{PasskeyCredential, Tag, TagUpdate, User, UserUpdate},
};

#[derive(Debug, thiserror::Error)]
pub enum CreateSqliteClientError {
    #[error("required environment variable not set: {0}")]
    MissingEnv(&'static str),

    #[error("environment variable {0} is not valid UTF-8")]
    EnvNotUtf8(&'static str),

    #[error("failed to migrate database to current version: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct SqliteClient {
    pool: SqlitePool,
}

impl SqliteClient {
    /// Opens or creates the database at the path given by the `DB_PATH` environment variable.
    pub async fn open() -> Result<Self, CreateSqliteClientError> {
        match std::env::var("DB_PATH") {
            Ok(path) => Ok(Self {
                pool: Self::do_open(&path).await?,
            }),
            Err(VarError::NotPresent) => Err(CreateSqliteClientError::MissingEnv("DB_PATH")),
            Err(VarError::NotUnicode(_)) => Err(CreateSqliteClientError::EnvNotUtf8("DB_PATH")),
        }
    }

    /// Creates a client that uses a new in-memory database.
    pub async fn new_memory() -> Result<Self, CreateSqliteClientError> {
        Ok(Self {
            pool: Self::do_open(":memory:").await?,
        })
    }

    async fn do_open(path: &str) -> Result<SqlitePool, CreateSqliteClientError> {
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .synchronous(SqliteSynchronous::Normal)
                .create_if_missing(true)
                .optimize_on_close(true, None)
                .pragma("foreign_keys", "ON")
                .filename(path),
        )
        .await?;

        sqlx::migrate!("src/db/clients/sqlite/migrations")
            .run(&pool)
            .await?;

        Ok(pool)
    }
}

impl DatabaseClient for SqliteClient {
    async fn create_user(&self, user: &User) -> Result<(), DatabaseError> {
        sqlx::query("INSERT INTO users (id, email, display_name, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
            .bind(user.id())
            .bind(user.email())
            .bind(user.display_name())
            .bind(user.created_at())
            .bind(user.updated_at())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_user_by_id(&self, id: &Uuid) -> Result<User, DatabaseError> {
        let user: User = sqlx::query_as(
            "SELECT id, email, display_name, created_at, updated_at FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User, DatabaseError> {
        let user: User = sqlx::query_as(
            "SELECT id, email, display_name, created_at, updated_at FROM users WHERE email = ?",
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    async fn update_user(&self, id: &Uuid, update: &UserUpdate) -> Result<User, DatabaseError> {
        if update.is_empty() {
            return Err(DatabaseError::EmptyUpdate);
        }

        let mut query_parts = Vec::new();
        let mut has_email = false;
        let mut has_display_name = false;

        if update.email.is_some() {
            query_parts.push("email = ?");
            has_email = true;
        }

        if update.display_name.is_some() {
            query_parts.push("display_name = ?");
            has_display_name = true;
        }

        // Always update the updated_at timestamp using SQLite's unixepoch function
        query_parts.push("updated_at = unixepoch()");

        let query = format!(
            "UPDATE users SET {} WHERE id = ? RETURNING id, email, display_name, created_at, updated_at",
            query_parts.join(", ")
        );

        let mut sql_query = sqlx::query_as::<_, User>(&query);

        // Bind parameters in order
        if has_email {
            sql_query = sql_query.bind(update.email.as_ref().unwrap());
        }
        if has_display_name {
            sql_query = sql_query.bind(update.display_name.as_ref().unwrap());
        }
        sql_query = sql_query.bind(id);

        let user = sql_query.fetch_one(&self.pool).await?;
        Ok(user)
    }

    async fn delete_user_by_id(&self, id: &Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_tag_to_user(
        &self,
        user_id: &Uuid,
        tag: &crate::models::Tag,
    ) -> Result<(), DatabaseError> {
        sqlx::query("INSERT INTO users_tags (user_id, tag_id) VALUES (?, ?)")
            .bind(user_id)
            .bind(tag.id())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_tag_from_user(
        &self,
        user_id: &Uuid,
        tag: &crate::models::Tag,
    ) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM users_tags WHERE user_id = ? AND tag_id = ?")
            .bind(user_id)
            .bind(tag.id())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_users_by_tag_id(&self, tag_id: &Uuid) -> Result<Vec<User>, DatabaseError> {
        let users: Vec<User> = sqlx::query_as(
            "SELECT u.id, u.email, u.display_name, u.created_at, u.updated_at 
             FROM users u 
             INNER JOIN users_tags ut
             ON u.id = ut.user_id 
             WHERE ut.tag_id = ?",
        )
        .bind(tag_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    async fn create_tag(&self, tag: &crate::models::Tag) -> Result<(), DatabaseError> {
        sqlx::query("INSERT INTO tags (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)")
            .bind(tag.id())
            .bind(tag.name())
            .bind(tag.created_at())
            .bind(tag.updated_at())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_tag_by_id(&self, id: &Uuid) -> Result<crate::models::Tag, DatabaseError> {
        let tag: Tag =
            sqlx::query_as("SELECT id, name, created_at, updated_at FROM tags WHERE id = ?")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(tag)
    }

    async fn get_tag_by_name(&self, name: &str) -> Result<crate::models::Tag, DatabaseError> {
        let tag: Tag =
            sqlx::query_as("SELECT id, name, created_at, updated_at FROM tags WHERE name = ?")
                .bind(name)
                .fetch_one(&self.pool)
                .await?;
        Ok(tag)
    }

    async fn delete_tag_by_id(&self, id: &Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_tags_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<crate::models::Tag>, DatabaseError> {
        let tags: Vec<Tag> = sqlx::query_as(
            "SELECT t.id, t.name, t.created_at, t.updated_at 
             FROM tags t 
             INNER JOIN users_tags ut
             ON t.id = ut.tag_id 
             WHERE ut.user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(tags)
    }

    async fn create_passkey(
        &self,
        passkey: &crate::models::PasskeyCredential,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            "INSERT INTO passkeys (id, user_id, credential_id, public_key, sign_count, created_at, last_used_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(passkey.id())
        .bind(passkey.user_id())
        .bind(passkey.credential_id())
        .bind(passkey.public_key())
        .bind(passkey.sign_count())
        .bind(passkey.created_at())
        .bind(passkey.last_used_at())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_passkey_by_id(
        &self,
        id: &Uuid,
    ) -> Result<crate::models::PasskeyCredential, DatabaseError> {
        let passkey: PasskeyCredential = sqlx::query_as(
            "SELECT id, user_id, credential_id, public_key, sign_count, created_at, last_used_at 
             FROM passkeys WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(passkey)
    }

    async fn get_passkey_by_credential_id(
        &self,
        credential_id: &[u8],
    ) -> Result<crate::models::PasskeyCredential, DatabaseError> {
        let passkey: PasskeyCredential = sqlx::query_as(
            "SELECT id, user_id, credential_id, public_key, sign_count, created_at, last_used_at 
             FROM passkeys WHERE credential_id = ?",
        )
        .bind(credential_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(passkey)
    }

    async fn get_passkeys_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<crate::models::PasskeyCredential>, DatabaseError> {
        let passkeys: Vec<PasskeyCredential> = sqlx::query_as(
            "SELECT id, user_id, credential_id, public_key, sign_count, created_at, last_used_at 
             FROM passkeys WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(passkeys)
    }

    async fn update_passkey(
        &self,
        passkey: &crate::models::PasskeyCredential,
    ) -> Result<crate::models::PasskeyCredential, DatabaseError> {
        sqlx::query(
            "UPDATE passkeys SET user_id = ?, credential_id = ?, public_key = ?, sign_count = ?, last_used_at = ? 
             WHERE id = ?",
        )
        .bind(passkey.user_id())
        .bind(passkey.credential_id())
        .bind(passkey.public_key())
        .bind(passkey.sign_count())
        .bind(passkey.last_used_at())
        .bind(passkey.id())
        .execute(&self.pool)
        .await?;
        Ok(passkey.clone())
    }

    async fn delete_passkey_by_id(&self, id: &Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM passkeys WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_tag(&self, id: &Uuid, update: &TagUpdate) -> Result<Tag, DatabaseError> {
        if update.is_empty() {
            return Err(DatabaseError::EmptyUpdate);
        }

        let mut query_parts = Vec::new();
        let mut has_name = false;

        if update.name.is_some() {
            query_parts.push("name = ?");
            has_name = true;
        }

        // Always update the updated_at timestamp using SQLite's unixepoch function
        query_parts.push("updated_at = unixepoch()");

        let query = format!(
            "UPDATE tags SET {} WHERE id = ? RETURNING id, name, created_at, updated_at",
            query_parts.join(", ")
        );

        let mut sql_query = sqlx::query_as::<_, Tag>(&query);

        // Bind parameters in order
        if has_name {
            sql_query = sql_query.bind(update.name.as_ref().unwrap());
        }
        sql_query = sql_query.bind(id);

        let tag = sql_query.fetch_one(&self.pool).await?;
        Ok(tag)
    }

    async fn increment_passkey_sign_count(&self, id: &Uuid) -> Result<(), DatabaseError> {
        sqlx::query("UPDATE passkeys SET sign_count = sign_count + 1, last_used_at = unixepoch() WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
