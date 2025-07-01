use std::borrow::Cow;

use uuid::Uuid;

use crate::models::{PasskeyCredential, Tag, TagUpdate, User, UserUpdate};

pub trait DatabaseClient: Send + Sync + 'static {
    // User repository
    async fn create_user(&self, user: &User) -> Result<(), DatabaseError>;
    async fn get_user_by_id(&self, id: &Uuid) -> Result<User, DatabaseError>;
    async fn get_user_by_email(&self, email: &str) -> Result<User, DatabaseError>;
    async fn update_user(&self, id: &Uuid, update: &UserUpdate) -> Result<User, DatabaseError>;
    async fn delete_user_by_id(&self, id: &Uuid) -> Result<(), DatabaseError>;
    async fn add_tag_to_user(&self, user_id: &Uuid, tag: &Tag) -> Result<(), DatabaseError>;
    async fn remove_tag_from_user(&self, user_id: &Uuid, tag: &Tag) -> Result<(), DatabaseError>;
    async fn get_users_by_tag_id(&self, tag_id: &Uuid) -> Result<Vec<User>, DatabaseError>;

    // Tag repository
    async fn create_tag(&self, tag: &Tag) -> Result<(), DatabaseError>;
    async fn get_tag_by_id(&self, id: &Uuid) -> Result<Tag, DatabaseError>;
    async fn get_tag_by_name(&self, name: &str) -> Result<Tag, DatabaseError>;
    async fn update_tag(&self, id: &Uuid, update: &TagUpdate) -> Result<Tag, DatabaseError>;
    async fn delete_tag_by_id(&self, id: &Uuid) -> Result<(), DatabaseError>;
    async fn get_tags_by_user_id(&self, user_id: &Uuid) -> Result<Vec<Tag>, DatabaseError>;

    // Passkey repository
    async fn create_passkey(&self, passkey: &PasskeyCredential) -> Result<(), DatabaseError>;
    async fn get_passkey_by_id(&self, id: &Uuid) -> Result<PasskeyCredential, DatabaseError>;
    async fn get_passkey_by_credential_id(
        &self,
        credential_id: &[u8],
    ) -> Result<PasskeyCredential, DatabaseError>;
    async fn get_passkeys_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<PasskeyCredential>, DatabaseError>;
    async fn update_passkey(
        &self,
        passkey: &PasskeyCredential,
    ) -> Result<PasskeyCredential, DatabaseError>;
    async fn delete_passkey_by_id(&self, id: &Uuid) -> Result<(), DatabaseError>;
    async fn increment_passkey_sign_count(&self, id: &Uuid) -> Result<(), DatabaseError>;
}

/// Error type for database operations
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("row/resource not found")]
    NotFound,

    #[error(
        "uniqueness violation {}{}",
        if field.is_some() { "on field " } else { "(field unknown)" },
        field.as_deref().unwrap_or("")
    )]
    UniquenessViolation {
        /// The field that caused the uniqueness violation, if known
        field: Option<Cow<'static, str>>,
    },

    #[error("the update request contains no changes")]
    EmptyUpdate,

    #[error("database error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl From<sqlx::Error> for DatabaseError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(e) if e.is_unique_violation() => {
                Self::UniquenessViolation { field: None }
            }
            other => Self::Other(Box::new(other)),
        }
    }
}
