use std::{env::VarError, path::Path};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteSynchronous},
};
use uuid::Uuid;

use crate::{db::interface::DatabaseClient, models::{User, Tag, PasskeyCredential}};

#[derive(Debug, thiserror::Error)]
pub enum SqliteError {
    #[error("database error: {0}")]
    DatabaseError(#[source] sqlx::Error),

    #[error("environment variable not set: {0}")]
    MissingEnv(&'static str),

    #[error("environment variable {0} is not valid UTF-8")]
    EnvNotUtf8(&'static str),

    #[error("the requested resource already exists")]
    AlreadyExists,
}

impl From<sqlx::Error> for SqliteError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::Database(e) if e.is_unique_violation() => Self::AlreadyExists,
            other => Self::DatabaseError(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SqliteClient {
    pool: SqlitePool,
}

impl SqliteClient {
    /// Opens or creates the database at the path given by the `DB_PATH` environment variable.
    pub async fn open() -> Result<Self, SqliteError> {
        match std::env::var("DB_PATH") {
            Ok(path) => Ok(Self {
                pool: Self::do_open(&path).await?,
            }),
            Err(VarError::NotPresent) => Err(SqliteError::MissingEnv("DB_PATH")),
            Err(VarError::NotUnicode(_)) => Err(SqliteError::EnvNotUtf8("DB_PATH")),
        }
    }

    /// Creates a client that uses a new in-memory database.
    pub async fn new_memory() -> Result<Self, SqliteError> {
        Ok(Self {
            pool: Self::do_open(":memory:").await?,
        })
    }

    async fn do_open(path: &str) -> Result<SqlitePool, sqlx::Error> {
        SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .synchronous(SqliteSynchronous::Normal)
                .create_if_missing(true)
                .optimize_on_close(true, None)
                .pragma("foreign_keys", "ON")
                .filename(path),
        )
        .await
    }
}

impl DatabaseClient for SqliteClient {
    type Error = SqliteError;

    async fn create_user(&self, user: &User) -> Result<(), Self::Error> {
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

    async fn get_user_by_id(&self, id: &Uuid) -> Result<User, Self::Error> {
        let user: User = sqlx::query_as(
            "SELECT id, email, display_name, created_at, updated_at FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User, Self::Error> {
        let user: User = sqlx::query_as(
            "SELECT id, email, display_name, created_at, updated_at FROM users WHERE email = ?",
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    async fn update_user(&self, user: &User) -> Result<User, Self::Error> {
        sqlx::query("UPDATE users SET email = ?, display_name = ?, updated_at = ? WHERE id = ?")
            .bind(user.email())
            .bind(user.display_name())
            .bind(user.updated_at())
            .bind(user.id())
            .execute(&self.pool)
            .await?;
        Ok(user.clone())
    }

    async fn delete_user_by_id(&self, id: &Uuid) -> Result<(), Self::Error> {
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
    ) -> Result<(), Self::Error> {
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
    ) -> Result<(), Self::Error> {
        sqlx::query("DELETE FROM users_tags WHERE user_id = ? AND tag_id = ?")
            .bind(user_id)
            .bind(tag.id())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_users_by_tag_id(&self, tag_id: &Uuid) -> Result<Vec<User>, Self::Error> {
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

    async fn create_tag(&self, tag: &crate::models::Tag) -> Result<(), Self::Error> {
        sqlx::query("INSERT INTO tags (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)")
            .bind(tag.id())
            .bind(tag.name())
            .bind(tag.created_at())
            .bind(tag.updated_at())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_tag_by_id(&self, id: &Uuid) -> Result<crate::models::Tag, Self::Error> {
        let tag: Tag = sqlx::query_as(
            "SELECT id, name, created_at, updated_at FROM tags WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(tag)
    }

    async fn get_tag_by_name(&self, name: &str) -> Result<crate::models::Tag, Self::Error> {
        let tag: Tag = sqlx::query_as(
            "SELECT id, name, created_at, updated_at FROM tags WHERE name = ?",
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await?;
        Ok(tag)
    }

    async fn delete_tag_by_id(&self, id: &Uuid) -> Result<(), Self::Error> {
        sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_tags_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<crate::models::Tag>, Self::Error> {
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
    ) -> Result<(), Self::Error> {
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
    ) -> Result<crate::models::PasskeyCredential, Self::Error> {
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
    ) -> Result<crate::models::PasskeyCredential, Self::Error> {
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
    ) -> Result<Vec<crate::models::PasskeyCredential>, Self::Error> {
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
    ) -> Result<crate::models::PasskeyCredential, Self::Error> {
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

    async fn delete_passkey_by_id(&self, id: &Uuid) -> Result<(), Self::Error> {
        sqlx::query("DELETE FROM passkeys WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
