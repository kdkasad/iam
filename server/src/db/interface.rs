use std::{borrow::Cow, future::Future};

use uuid::Uuid;

use crate::models::{PasskeyCredential, Tag, TagUpdate, User, UserUpdate};

pub trait DatabaseClient: Send + Sync + 'static {
    // User repository
    fn create_user(&self, user: &User) -> impl Future<Output = Result<(), DatabaseError>> + Send;
    fn get_user_by_id(&self, id: &Uuid)
    -> impl Future<Output = Result<User, DatabaseError>> + Send;
    fn get_user_by_email(
        &self,
        email: &str,
    ) -> impl Future<Output = Result<User, DatabaseError>> + Send;
    fn update_user(
        &self,
        id: &Uuid,
        update: &UserUpdate,
    ) -> impl Future<Output = Result<User, DatabaseError>> + Send;
    fn delete_user_by_id(
        &self,
        id: &Uuid,
    ) -> impl Future<Output = Result<(), DatabaseError>> + Send;
    fn add_tag_to_user(
        &self,
        user_id: &Uuid,
        tag: &Tag,
    ) -> impl Future<Output = Result<(), DatabaseError>> + Send;
    fn remove_tag_from_user(
        &self,
        user_id: &Uuid,
        tag: &Tag,
    ) -> impl Future<Output = Result<(), DatabaseError>> + Send;
    fn get_users_by_tag_id(
        &self,
        tag_id: &Uuid,
    ) -> impl Future<Output = Result<Vec<User>, DatabaseError>> + Send;

    // Tag repository
    fn create_tag(&self, tag: &Tag) -> impl Future<Output = Result<(), DatabaseError>> + Send;
    fn get_tag_by_id(&self, id: &Uuid) -> impl Future<Output = Result<Tag, DatabaseError>> + Send;
    fn get_tag_by_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Tag, DatabaseError>> + Send;
    fn update_tag(
        &self,
        id: &Uuid,
        update: &TagUpdate,
    ) -> impl Future<Output = Result<Tag, DatabaseError>> + Send;
    fn delete_tag_by_id(&self, id: &Uuid)
    -> impl Future<Output = Result<(), DatabaseError>> + Send;
    fn get_tags_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> impl Future<Output = Result<Vec<Tag>, DatabaseError>> + Send;

    // Passkey repository
    fn create_passkey(
        &self,
        passkey: &PasskeyCredential,
    ) -> impl Future<Output = Result<(), DatabaseError>> + Send;
    fn get_passkey_by_id(
        &self,
        id: &Uuid,
    ) -> impl Future<Output = Result<PasskeyCredential, DatabaseError>> + Send;
    fn get_passkey_by_credential_id(
        &self,
        credential_id: &[u8],
    ) -> impl Future<Output = Result<PasskeyCredential, DatabaseError>> + Send;
    fn get_passkeys_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> impl Future<Output = Result<Vec<PasskeyCredential>, DatabaseError>> + Send;
    fn update_passkey(
        &self,
        passkey: &PasskeyCredential,
    ) -> impl Future<Output = Result<PasskeyCredential, DatabaseError>> + Send;
    fn delete_passkey_by_id(
        &self,
        id: &Uuid,
    ) -> impl Future<Output = Result<(), DatabaseError>> + Send;
    fn increment_passkey_sign_count(
        &self,
        id: &Uuid,
    ) -> impl Future<Output = Result<(), DatabaseError>> + Send;
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
