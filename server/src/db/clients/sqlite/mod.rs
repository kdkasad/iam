use std::{env::VarError, pin::Pin};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteSynchronous},
};
use uuid::Uuid;

use crate::{
    db::interface::{DatabaseClient, DatabaseError},
    models::{
        EncodableHash, NewPasskeyCredential, PasskeyAuthenticationState, PasskeyCredential,
        PasskeyCredentialUpdate, PasskeyRegistrationState, Tag, TagUpdate, User, UserCreate,
        UserUpdate,
    },
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
                pool: Self::do_open(
                    SqliteConnectOptions::new()
                        .create_if_missing(true)
                        .filename(&path),
                )
                .await?,
            }),
            Err(VarError::NotPresent) => Err(CreateSqliteClientError::MissingEnv("DB_PATH")),
            Err(VarError::NotUnicode(_)) => Err(CreateSqliteClientError::EnvNotUtf8("DB_PATH")),
        }
    }

    /// Creates a client that uses a new in-memory database.
    pub async fn new_memory() -> Result<Self, CreateSqliteClientError> {
        // sqlx has some special handling for the in-memory database which only
        // happens when parsing from a URL string
        Ok(Self {
            pool: Self::do_open("sqlite://:memory:".parse().unwrap()).await?,
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
                .bind(tag.id())
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
                .bind(tag.id())
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
                "INSERT INTO passkeys (id, user_id, passkey, display_name, created_at, last_used_at)
                 VALUES ($1, $2, $3, $4, unixepoch(), unixepoch())
                 RETURNING *",
            )
            .bind(id)
            .bind(user_id)
            .bind(sqlx::types::Json(&passkey.passkey))
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
                "SELECT *
                 FROM passkeys WHERE id = $1",
            )
            .bind(id)
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
                "SELECT *
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
                "SELECT * FROM passkeys
                INNER JOIN users ON passkeys.user_id = users.id
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

            // There's only one updatable field, so we can just unwrap it
            let passkey: PasskeyCredential =
                sqlx::query_as("UPDATE passkeys SET display_name = $1 WHERE id = $2 RETURNING *")
                    .bind(passkey.display_name.as_ref().unwrap())
                    .bind(id)
                    .fetch_one(&pool)
                    .await?;
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

    fn increment_passkey_sign_count<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("UPDATE passkeys SET sign_count = sign_count + 1, last_used_at = unixepoch() WHERE id = $1")
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
            let result = sqlx::query("INSERT INTO passkey_authentications (id, user_email, state, created_at) VALUES ($1, $2, $3, $4)")
                .bind(state.id)
                .bind(&state.email)
                .bind(&state.state)
                .bind(state.created_at.timestamp())
                .execute(&pool)
                .await;
            if let Err(e) = result {
                if e.as_database_error()
                    .is_some_and(|dbe| dbe.is_foreign_key_violation())
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
        session: &'a crate::models::Session,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'a>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO sessions (id_hash, user_id, created_at, expires_at, state)
                VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(session.id_hash)
            .bind(session.user_id)
            .bind(session.created_at.timestamp())
            .bind(session.expires_at.timestamp())
            .bind(session.state)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn get_session_by_id_hash<'id>(
        &self,
        id_hash: &'id EncodableHash,
    ) -> Pin<Box<dyn Future<Output = Result<crate::models::Session, DatabaseError>> + Send + 'id>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let session: crate::models::Session =
                sqlx::query_as("SELECT * FROM sessions WHERE id_hash = $1")
                    .bind(id_hash)
                    .fetch_one(&pool)
                    .await?;
            Ok(session)
        })
    }
}

#[cfg(test)]
mod tests {

    use uuid::Uuid;
    use webauthn_rs::{
        Webauthn, WebauthnBuilder,
        prelude::{Passkey, Url},
    };

    use super::SqliteClient;
    use crate::{
        db::interface::DatabaseClient,
        models::{
            NewPasskeyCredential, PasskeyRegistrationState, Session, SessionState, UserCreate,
        },
    };

    struct Tools {
        client: SqliteClient,
        webauthn: Webauthn,
    }

    const PASSKEY_JSON: &str = r#"
    {"cred":{"cred_id":"Gx07kWmVrKBrB31KmXxHSnAK2kI","cred":{"type_":"ES256","key":{"EC_EC2":{"curve":"SECP256R1","x":"k1zbsP39Y1go2_Pea23c5AT2ZuP6NBx67NTZZdjiPUM","y":"qznBgidGVTuHwMohwxJNDRN_gVh1Ipn5mENE2hYXot0"}}},"counter":0,"transports":null,"user_verified":true,"backup_eligible":true,"backup_state":true,"registration_policy":"required","extensions":{"cred_protect":"Ignored","hmac_create_secret":"NotRequested","appid":"NotRequested","cred_props":"Ignored"},"attestation":{"data":"None","metadata":"None"},"attestation_format":"none"}}
    "#;

    /// Create a new set of tools/clients for a test.
    async fn tools() -> Tools {
        Tools {
            client: SqliteClient::new_memory()
                .await
                .expect("expected client creation to succeed"),
            webauthn: WebauthnBuilder::new(
                "example.org",
                &Url::parse("http://example.org").unwrap(),
            )
            .expect("expected webauthn builder creation to succeed")
            .build()
            .expect("expected webauthn creation to succeed"),
        }
    }

    #[tokio::test]
    async fn test_create_user() {
        let Tools { client, .. } = tools().await;
        let user = client
            .create_user(
                &Uuid::new_v4(),
                &UserCreate {
                    email: "test@example.com".to_string(),
                    display_name: "Test User".to_string(),
                },
            )
            .await
            .expect("expected user creation to succeed");
        assert_eq!(user.email(), "test@example.com");
        assert_eq!(user.display_name(), "Test User");
    }

    #[tokio::test]
    async fn test_create_passkey_registration() {
        let Tools { client, webauthn } = tools().await;
        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let display_name = "Test User";
        let (_, reg) = webauthn
            .start_passkey_registration(user_id, email, display_name, None)
            .unwrap();
        let registration = PasskeyRegistrationState {
            id: Uuid::new_v4(),
            user_id,
            email: email.to_string(),
            registration: sqlx::types::Json(reg),
            created_at: chrono::Utc::now(),
        };
        client
            .create_passkey_registration(&registration)
            .await
            .expect("expected create passkey registration to succeed");
    }

    #[tokio::test]
    async fn test_get_passkey_registration_by_id() {
        let Tools { client, webauthn } = tools().await;
        // Set up: create a passkey registration
        let email = "test@kasad.com";
        let display_name = "Test User";
        let id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (_, reg) = webauthn
            .start_passkey_registration(user_id, email, display_name, None)
            .unwrap();
        let registration = PasskeyRegistrationState {
            id,
            user_id,
            email: email.to_string(),
            registration: sqlx::types::Json(reg),
            created_at: chrono::Utc::now(),
        };
        client
            .create_passkey_registration(&registration)
            .await
            .unwrap();

        // Test: get the passkey registration by id
        let registration = client.get_passkey_registration_by_id(&id).await.unwrap();
        assert_eq!(registration.user_id, user_id);
        assert_eq!(registration.email, email);
    }

    #[tokio::test]
    async fn test_create_session() {
        let Tools { client, .. } = tools().await;

        // Set up: create a user
        let user = client
            .create_user(
                &Uuid::new_v4(),
                &UserCreate {
                    email: "test@kasad.com".to_string(),
                    display_name: "Test User".to_string(),
                },
            )
            .await
            .expect("expected user creation to succeed");

        // Test: create session
        let session_id: u64 = 123456789;
        let session = Session {
            user_id: *user.id(),
            id_hash: blake3::hash(&session_id.to_le_bytes()).into(),
            state: SessionState::Active,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(1),
        };
        client.create_session(&session).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_session_by_id_hash() {
        let Tools { client, .. } = tools().await;

        // Set up: create a user
        let user = client
            .create_user(
                &Uuid::new_v4(),
                &UserCreate {
                    email: "test@kasad.com".to_string(),
                    display_name: "Test User".to_string(),
                },
            )
            .await
            .expect("expected user creation to succeed");

        // Set up: create session
        let session_id: u64 = 123456789;
        let session = Session {
            user_id: *user.id(),
            id_hash: blake3::hash(&session_id.to_le_bytes()).into(),
            state: SessionState::Active,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(1),
        };
        client.create_session(&session).await.unwrap();

        // Test: get session by id hash
        let session = client
            .get_session_by_id_hash(&session.id_hash)
            .await
            .unwrap();
        assert_eq!(session.user_id, *user.id());
        assert_eq!(session.id_hash.0, session.id_hash.0);
        assert_eq!(session.state, SessionState::Active);
        assert_eq!(session.created_at, session.created_at);
        assert_eq!(session.expires_at, session.expires_at);
    }

    #[tokio::test]
    async fn test_create_passkey() {
        let Tools { client, .. } = tools().await;
        let user_id = Uuid::new_v4();
        client
            .create_user(
                &user_id,
                &UserCreate {
                    email: "test@kasad.com".to_string(),
                    display_name: "Test User".to_string(),
                },
            )
            .await
            .unwrap();
        let passkey: Passkey = serde_json::from_str(PASSKEY_JSON).unwrap();
        client
            .create_passkey(
                &Uuid::new_v4(),
                &user_id,
                &NewPasskeyCredential {
                    display_name: None,
                    passkey,
                },
            )
            .await
            .unwrap();
    }
}
