#![expect(clippy::doc_markdown)]

//! # SQLite3 database client
//!
//! A [`DatabaseClient`] which uses a SQLite3 database as the backend. Either memory-backed or
//! file-backed databases can be used.

use std::{env::VarError, pin::Pin, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteSynchronous},
};
use tokio::task::{AbortHandle, JoinHandle};
use tracing::error;
use uuid::Uuid;

use crate::{
    db::interface::{DatabaseClient, DatabaseError},
    models::{
        EncodableHash, NewPasskeyCredential, PasskeyAuthenticationState, PasskeyCredential,
        PasskeyCredentialUpdate, PasskeyRegistrationState, Session, SessionUpdate, Tag, TagUpdate,
        User, UserCreate, UserUpdate,
    },
};

/// Represents errors that can occur when creating a new SQLite3 client, e.g. with
/// [`SqliteClient::open()`] or [`SqliteClient::new_memory()`].
#[derive(Debug, thiserror::Error)]
pub enum CreateSqliteClientError {
    /// An environment variable (whose name is given by the field) was required but not set.
    #[error("required environment variable not set: {0}")]
    MissingEnv(&'static str),

    /// An environment variable (whose name is given by the field) was set but is not valid UTF-8.
    #[error("environment variable {0} is not valid UTF-8")]
    EnvNotUtf8(&'static str),

    /// Applying a database migration failed. The [upstream error][sqlx::migrate::MigrateError] is
    /// contained in the tuple field.
    #[error("failed to migrate database to current version: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Some other database error occurred. The [upstream error][sqlx::Error] is contained in the
    /// tuple field.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// # SQLite3 database backend
///
/// See [the module-level documentation][crate::db::clients::sqlite] for details.
#[derive(Debug, Clone)]
pub struct SqliteClient {
    pool: SqlitePool,
    cleanup_task_abort_handle: AbortHandle,
}

impl SqliteClient {
    /// Opens or creates the database at the path given by the `DB_PATH` environment variable.
    pub async fn open() -> Result<Self, CreateSqliteClientError> {
        let pool = match std::env::var("DB_PATH") {
            Ok(path) => {
                Self::do_open(
                    SqliteConnectOptions::new()
                        .create_if_missing(true)
                        .filename(&path),
                )
                .await?
            }
            Err(VarError::NotPresent) => {
                return Err(CreateSqliteClientError::MissingEnv("DB_PATH"));
            }
            Err(VarError::NotUnicode(_)) => {
                return Err(CreateSqliteClientError::EnvNotUtf8("DB_PATH"));
            }
        };
        let cleanup_task = Self::spawn_cleanup_task(pool.clone());
        Ok(Self {
            pool,
            cleanup_task_abort_handle: cleanup_task.abort_handle(),
        })
    }

    /// Creates a client that uses a new in-memory database.
    pub async fn new_memory() -> Result<Self, CreateSqliteClientError> {
        // sqlx has some special handling for the in-memory database which only
        // happens when parsing from a URL string
        let pool = Self::do_open("sqlite://:memory:".parse().unwrap()).await?;
        let cleanup_task = Self::spawn_cleanup_task(pool.clone());
        Ok(Self {
            pool,
            cleanup_task_abort_handle: cleanup_task.abort_handle(),
        })
    }

    /// Creates a task that runs in the background and cleans up expired passkey registrations and authentications every 5 minutes.
    /// Returns the [`JoinHandle`] for the task.
    fn spawn_cleanup_task(pool: SqlitePool) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5 * 60)).await;
                do_cleanup(&pool).await;
            }
        })
    }

    async fn do_open(
        base_options: SqliteConnectOptions,
    ) -> Result<SqlitePool, CreateSqliteClientError> {
        let options = base_options
            .synchronous(SqliteSynchronous::Normal)
            .optimize_on_close(true, None)
            .pragma("foreign_keys", "ON");
        let pool = SqlitePool::connect_with(options).await?;

        sqlx::migrate!("src/db/clients/sqlite/migrations")
            .run(&pool)
            .await?;

        Ok(pool)
    }
}

impl Drop for SqliteClient {
    fn drop(&mut self) {
        self.cleanup_task_abort_handle.abort();
        _ = self.pool.close();
    }
}

impl DatabaseClient for SqliteClient {
    fn create_user<'user>(
        &self,
        id: &'user Uuid,
        user: &'user UserCreate,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'user>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            Ok(sqlx::query_as::<_, User>(
                "INSERT INTO users (id, email, display_name, created_at, updated_at)
                VALUES ($1, $2, $3, unixepoch(), unixepoch())
                RETURNING *",
            )
            .bind(id)
            .bind(&user.email)
            .bind(&user.display_name)
            .fetch_one(&pool)
            .await?)
        })
    }

    fn get_user_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let user: User = sqlx::query_as(
                "SELECT id, email, display_name, created_at, updated_at FROM users WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&pool)
            .await?;
            Ok(user)
        })
    }

    fn get_user_by_email<'email>(
        &self,
        email: &'email str,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'email>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let user: User = sqlx::query_as(
                "SELECT id, email, display_name, created_at, updated_at FROM users WHERE email = $1",
            )
            .bind(email)
            .fetch_one(&pool)
            .await?;
            Ok(user)
        })
    }

    fn update_user<'arg>(
        &self,
        id: &'arg Uuid,
        update: &'arg UserUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'arg>> {
        let pool = self.pool.clone();
        Box::pin(async move {
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

            let user = sql_query.fetch_one(&pool).await?;
            Ok(user)
        })
    }

    fn delete_user_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("DELETE FROM users WHERE id = $1")
                .bind(id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    fn add_tag_to_user<'arg>(
        &self,
        user_id: &'arg Uuid,
        tag: &'arg Tag,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'arg>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("INSERT INTO users_tags (user_id, tag_id) VALUES ($1, $2)")
                .bind(user_id)
                .bind(tag.id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    fn remove_tag_from_user<'arg>(
        &self,
        user_id: &'arg Uuid,
        tag: &'arg Tag,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'arg>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("DELETE FROM users_tags WHERE user_id = $1 AND tag_id = $2")
                .bind(user_id)
                .bind(tag.id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    fn get_users_by_tag_id<'id>(
        &self,
        tag_id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<User>, DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let users: Vec<User> = sqlx::query_as(
                "SELECT u.id, u.email, u.display_name, u.created_at, u.updated_at
                 FROM users u
                 INNER JOIN users_tags ut
                 ON u.id = ut.user_id
                 WHERE ut.tag_id = $1",
            )
            .bind(tag_id)
            .fetch_all(&pool)
            .await?;
            Ok(users)
        })
    }

    fn create_tag<'tag>(
        &self,
        id: &'tag Uuid,
        tag: &'tag TagUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'tag>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            Ok(sqlx::query_as::<_, Tag>(
                "INSERT INTO tags (id, name, created_at, updated_at)
            VALUES ($1, $2, unixepoch(), unixepoch())
            RETURNING id, name, created_at, updated_at",
            )
            .bind(id)
            .bind(&tag.name)
            .fetch_one(&pool)
            .await?)
        })
    }

    fn get_tag_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let tag: Tag =
                sqlx::query_as("SELECT id, name, created_at, updated_at FROM tags WHERE id = $1")
                    .bind(id)
                    .fetch_one(&pool)
                    .await?;
            Ok(tag)
        })
    }

    fn get_tag_by_name<'name>(
        &self,
        name: &'name str,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'name>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let tag: Tag =
                sqlx::query_as("SELECT id, name, created_at, updated_at FROM tags WHERE name = $1")
                    .bind(name)
                    .fetch_one(&pool)
                    .await?;
            Ok(tag)
        })
    }

    fn update_tag<'arg>(
        &self,
        id: &'arg Uuid,
        update: &'arg TagUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'arg>> {
        let pool = self.pool.clone();
        Box::pin(async move {
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
                "UPDATE tags SET {} WHERE id = $1 RETURNING id, name, created_at, updated_at",
                query_parts.join(", ")
            );

            let mut sql_query = sqlx::query_as::<_, Tag>(&query);

            // Bind parameters in order
            if has_name {
                sql_query = sql_query.bind(update.name.as_ref().unwrap());
            }
            sql_query = sql_query.bind(id);

            let tag = sql_query.fetch_one(&pool).await?;
            Ok(tag)
        })
    }

    fn delete_tag_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("DELETE FROM tags WHERE id = $1")
                .bind(id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    fn get_tags_by_user_id<'id>(
        &self,
        user_id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Tag>, DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let tags: Vec<Tag> = sqlx::query_as(
                "SELECT t.id, t.name, t.created_at, t.updated_at
                 FROM tags t
                 INNER JOIN users_tags ut
                 ON t.id = ut.tag_id
                 WHERE ut.user_id = $1",
            )
            .bind(user_id)
            .fetch_all(&pool)
            .await?;
            Ok(tags)
        })
    }

    fn create_passkey<'a>(
        &self,
        id: &'a Uuid,
        user_id: &'a Uuid,
        passkey: &'a NewPasskeyCredential,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'a>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let passkey: PasskeyCredential = sqlx::query_as(
                "INSERT INTO passkeys (id, user_id, passkey, credential_id, display_name, created_at, last_used_at)
                 VALUES ($1, $2, $3, $4, $5, unixepoch(), unixepoch())
                 RETURNING *",
            )
            .bind(id)
            .bind(user_id)
            .bind(sqlx::types::Json(&passkey.passkey))
            .bind(passkey.passkey.cred_id().as_ref())
            .bind(&passkey.display_name)
            .fetch_one(&pool)
            .await?;
            Ok(passkey)
        })
    }

    fn get_passkey_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let passkey: PasskeyCredential = sqlx::query_as(
                "SELECT id, user_id, passkey, display_name, created_at, last_used_at
                 FROM passkeys WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&pool)
            .await?;
            Ok(passkey)
        })
    }

    fn get_passkey_by_credential_id<'id>(
        &self,
        credential_id: &'id [u8],
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let passkey: PasskeyCredential = sqlx::query_as(
                "SELECT id, user_id, passkey, display_name, created_at, last_used_at
                 FROM passkeys WHERE credential_id = $1",
            )
            .bind(credential_id)
            .fetch_one(&pool)
            .await?;
            Ok(passkey)
        })
    }

    fn get_passkeys_by_user_id<'id>(
        &self,
        user_id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PasskeyCredential>, DatabaseError>> + Send + 'id>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let passkeys: Vec<PasskeyCredential> = sqlx::query_as(
                "SELECT id, user_id, passkey, display_name, created_at, last_used_at
                 FROM passkeys WHERE user_id = $1",
            )
            .bind(user_id)
            .fetch_all(&pool)
            .await?;
            Ok(passkeys)
        })
    }

    fn get_passkeys_by_user_email<'email>(
        &self,
        email: &'email str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PasskeyCredential>, DatabaseError>> + Send + 'email>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let passkeys: Vec<PasskeyCredential> = sqlx::query_as(
                "SELECT p.id, p.user_id, p.passkey, p.display_name, p.created_at, p.last_used_at
                FROM passkeys p
                INNER JOIN users ON p.user_id = users.id
                WHERE users.email = $1",
            )
            .bind(email)
            .fetch_all(&pool)
            .await?;
            Ok(passkeys)
        })
    }

    fn update_passkey<'key>(
        &self,
        id: &'key Uuid,
        passkey: &'key PasskeyCredentialUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'key>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            if passkey.is_empty() {
                return Err(DatabaseError::EmptyUpdate);
            }

            let mut query_parts = Vec::new();
            let mut has_display_name = false;
            let mut has_passkey = false;
            if passkey.display_name.is_some() {
                query_parts.push("display_name = ?");
                has_display_name = true;
            }
            if passkey.passkey.is_some() {
                query_parts.push("passkey = ?");
                has_passkey = true;
            }

            let query_str = format!(
                "UPDATE passkeys SET {}
                WHERE id = ?
                RETURNING id, user_id, passkey, display_name, created_at, last_used_at",
                query_parts.join(", ")
            );
            let mut query = sqlx::query_as::<_, PasskeyCredential>(&query_str);
            if has_display_name {
                query = query.bind(passkey.display_name.as_ref().unwrap().as_deref());
            }
            if has_passkey {
                query = query.bind(passkey.passkey.as_ref().unwrap());
            }
            query = query.bind(id);

            let passkey: PasskeyCredential = query.fetch_one(&pool).await?;
            Ok(passkey)
        })
    }

    fn delete_passkey_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("DELETE FROM passkeys WHERE id = $1")
                .bind(id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    fn create_passkey_registration<'a>(
        &self,
        registration: &'a PasskeyRegistrationState,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'a>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO passkey_registrations (id, user_id, email, registration, created_at)
                VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(registration.id)
            .bind(registration.user_id)
            .bind(&registration.email)
            .bind(&registration.registration)
            .bind(registration.created_at.timestamp())
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn get_passkey_registration_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyRegistrationState, DatabaseError>> + Send + 'id>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let registration: PasskeyRegistrationState =
                sqlx::query_as("SELECT * FROM passkey_registrations WHERE id = $1")
                    .bind(id)
                    .fetch_one(&pool)
                    .await?;
            Ok(registration)
        })
    }

    fn create_passkey_authentication<'a>(
        &self,
        state: &'a PasskeyAuthenticationState,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'a>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let result = sqlx::query("INSERT INTO passkey_authentications (id, email, state, created_at) VALUES ($1, $2, $3, $4)")
                .bind(state.id)
                .bind(&state.email)
                .bind(&state.state)
                .bind(state.created_at.timestamp())
                .execute(&pool)
                .await;
            if let Err(e) = result {
                if e.as_database_error()
                    .is_some_and(sqlx::error::DatabaseError::is_foreign_key_violation)
                {
                    return Err(DatabaseError::UserNotFound);
                }
                return Err(e.into());
            }
            Ok(())
        })
    }

    fn get_passkey_authentication_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyAuthenticationState, DatabaseError>> + Send + 'id>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let state: PasskeyAuthenticationState =
                sqlx::query_as("SELECT * FROM passkey_authentications WHERE id = $1")
                    .bind(id)
                    .fetch_one(&pool)
                    .await?;
            Ok(state)
        })
    }

    fn create_session<'a>(
        &self,
        session: &'a Session,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'a>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO sessions (id_hash, user_id, created_at, expires_at, state, is_admin, parent_id_hash)
                VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(session.id_hash)
            .bind(session.user_id)
            .bind(session.created_at.timestamp())
            .bind(session.expires_at.timestamp())
            .bind(session.state)
            .bind(session.is_admin)
            .bind(session.parent_id_hash)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn get_session_by_id_hash<'id>(
        &self,
        id_hash: &'id EncodableHash,
    ) -> Pin<Box<dyn Future<Output = Result<Session, DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let session: Session = sqlx::query_as("SELECT * FROM sessions WHERE id_hash = $1")
                .bind(id_hash)
                .fetch_one(&pool)
                .await?;
            Ok(session)
        })
    }

    fn update_session<'a>(
        &self,
        id_hash: &'a EncodableHash,
        update: &'a SessionUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<Session, DatabaseError>> + Send + 'a>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            if update.is_empty() {
                return Err(DatabaseError::EmptyUpdate);
            }

            let mut query_parts = Vec::new();
            let mut has_state = false;
            let mut has_expires_at = false;

            if update.state.is_some() {
                query_parts.push("state = ?");
                has_state = true;
            }

            if update.expires_at.is_some() {
                query_parts.push("expires_at = ?");
                has_expires_at = true;
            }

            let query_str = format!(
                "UPDATE sessions SET {}
                WHERE id_hash = ?
                RETURNING *",
                query_parts.join(", ")
            );

            let mut query = sqlx::query_as::<_, Session>(&query_str);
            if has_state {
                query = query.bind(update.state.as_ref().unwrap());
            }
            if has_expires_at {
                query = query.bind(update.expires_at.as_ref().unwrap().timestamp());
            }
            query = query.bind(id_hash);

            let session: Session = query.fetch_one(&pool).await?;
            Ok(session)
        })
    }
}

/// Cleans up expired passkey registrations and authentications.
async fn do_cleanup(pool: &SqlitePool) {
    if let Err(err) =
        sqlx::query("DELETE FROM passkey_registrations WHERE created_at < unixepoch() - 300")
            .execute(pool)
            .await
    {
        error!(%err, "failed to cleanup passkey registrations");
    }
    if let Err(err) =
        sqlx::query("DELETE FROM passkey_authentications WHERE created_at < unixepoch() - 300")
            .execute(pool)
            .await
    {
        error!(%err, "failed to cleanup passkey authentications");
    }
}

#[cfg(test)]
mod tests;
