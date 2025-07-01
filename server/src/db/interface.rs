use std::{borrow::Cow, future::Future, pin::Pin};

use uuid::Uuid;

use crate::models::{PasskeyCredential, Tag, TagUpdate, User, UserUpdate};

pub trait DatabaseClient: Send + Sync + 'static {

    // User repository

    fn create_user<'user>(&self, user: &'user User) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'user>>;

    fn get_user_by_id<'id>(&self, id: &'id Uuid) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'id>>;

    fn get_user_by_email<'email>(
        &self,
        email: &'email str,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'email>>;

    fn update_user<'arg>(
        &self,
        id: &'arg Uuid,
        update: &'arg UserUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'arg>>;

    fn delete_user_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>>;

    fn add_tag_to_user<'arg>(
        &self,
        user_id: &'arg Uuid,
        tag: &'arg Tag,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'arg>>;

    fn remove_tag_from_user<'arg>(
        &self,
        user_id: &'arg Uuid,
        tag: &'arg Tag,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'arg>>;

    fn get_users_by_tag_id<'id>(
        &self,
        tag_id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<User>, DatabaseError>> + Send + 'id>>;

    // Tag repository

    fn create_tag<'tag>(&self, tag: &'tag Tag) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'tag>>;

    fn get_tag_by_id<'id>(&self, id: &'id Uuid) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'id>>;

    fn get_tag_by_name<'name>(
        &self,
        name: &'name str,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'name>>;

    fn update_tag<'arg>(
        &self,
        id: &'arg Uuid,
        update: &'arg TagUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'arg>>;

    fn delete_tag_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>>;

    fn get_tags_by_user_id<'id>(
        &self,
        user_id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Tag>, DatabaseError>> + Send + 'id>>;

    // Passkey repository

    fn create_passkey<'key>(
        &self,
        passkey: &'key PasskeyCredential,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'key>>;

    fn get_passkey_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'id>>;

    fn get_passkey_by_credential_id<'id>(
        &self,
        credential_id: &'id [u8],
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'id>>;

    fn get_passkeys_by_user_id<'id>(
        &self,
        user_id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PasskeyCredential>, DatabaseError>> + Send + 'id>>;

    fn update_passkey<'key>(
        &self,
        passkey: &'key PasskeyCredential,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'key>>;

    fn delete_passkey_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>>;

    fn increment_passkey_sign_count<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>>;

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
