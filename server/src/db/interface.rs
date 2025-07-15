//! # Database abstraction layer
//!
//! See [`DatabaseClient`] for details.

use std::{borrow::Cow, future::Future, pin::Pin};

use uuid::Uuid;

use crate::models::{
    EncodableHash, NewPasskeyCredential, PasskeyAuthenticationState, PasskeyCredential,
    PasskeyCredentialUpdate, PasskeyRegistrationState, Session, SessionUpdate, Tag, TagUpdate,
    User, UserCreate, UserUpdate,
};

/// # Database abstraction layer interface
///
/// [`DatabaseClient`] is an abstraction layer that allows database operations to be performed
/// regardless of the underlying database backend. All operations which require reading/writing of
/// persistent storage must go through a method in this trait.
///
/// Database backends (e.g., [`SqliteClient`]) must implement this interface and should also
/// provide an `open()` function to open a connection to a database and return the new client.
/// Since different databases might require different information for creating a client, that
/// function is not part of this trait.
///
/// [`SqliteClient`]: crate::db::clients::sqlite::SqliteClient
pub trait DatabaseClient: Send + Sync + 'static {
    // User repository

    /// Creates a new [`User`] with the given ID and initial information and returns a result
    /// containing the created [`User`] or an error.
    fn create_user<'user>(
        &self,
        id: &'user Uuid,
        user: &'user UserCreate,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'user>>;

    /// Fetches the [`User`] with the given user ID.
    fn get_user_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'id>>;

    /// Fetches the [`User`] with the given email address.
    fn get_user_by_email<'email>(
        &self,
        email: &'email str,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'email>>;

    /// Alters the [`User`] with the given UUID, returning the updated [`User`] on success.
    fn update_user<'arg>(
        &self,
        id: &'arg Uuid,
        update: &'arg UserUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<User, DatabaseError>> + Send + 'arg>>;

    /// Deletes the [`User`] with the given UUID.
    fn delete_user_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>>;

    /// Adds the given [`Tag`] to the user with the given UUID.
    fn add_tag_to_user<'arg>(
        &self,
        user_id: &'arg Uuid,
        tag: &'arg Tag,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'arg>>;

    /// Removes the given [`Tag`] from the user with the given UUID.
    fn remove_tag_from_user<'arg>(
        &self,
        user_id: &'arg Uuid,
        tag: &'arg Tag,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'arg>>;

    /// Fetches a list of users who belong to the [`Tag`] with the given UUID.
    fn get_users_by_tag_id<'id>(
        &self,
        tag_id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<User>, DatabaseError>> + Send + 'id>>;

    // Tag repository

    /// Creates a new [`Tag`] with the given ID and initial information. Returns the newly
    /// created [`Tag`] on success.
    fn create_tag<'tag>(
        &self,
        id: &'tag Uuid,
        tag: &'tag TagUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'tag>>;

    /// Fetches the [`Tag`] with the given UUID.
    fn get_tag_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'id>>;

    /// Fetches the [`Tag`] with the given name.
    fn get_tag_by_name<'name>(
        &self,
        name: &'name str,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'name>>;

    /// Alters the [`Tag`] with the given UUID, returning the updated [`Tag`] on success.
    fn update_tag<'arg>(
        &self,
        id: &'arg Uuid,
        update: &'arg TagUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, DatabaseError>> + Send + 'arg>>;

    /// Deletes the [`Tag`] with the given UUID.
    fn delete_tag_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>>;

    /// Fetches a list of tags to which the [`User`] with the given UUID belongs.
    fn get_tags_by_user_id<'id>(
        &self,
        user_id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Tag>, DatabaseError>> + Send + 'id>>;

    // Passkey repository

    /// Creates a new [`PasskeyCredential`] with the given UUID and initial information for the
    /// user with the given user UUID. Returns the newly created [`PasskeyCredential`] on success.
    fn create_passkey<'a>(
        &self,
        id: &'a Uuid,
        user_id: &'a Uuid,
        passkey: &'a NewPasskeyCredential,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'a>>;

    /// Fetches a [`PasskeyCredential`] by its UUID.
    fn get_passkey_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'id>>;

    /// Fetches a [`PasskeyCredential`] by its credential ID.
    fn get_passkey_by_credential_id<'id>(
        &self,
        credential_id: &'id [u8],
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'id>>;

    /// Fetches a list of [`PasskeyCredential`]s belonging to the [`User`] with the given UUID.
    fn get_passkeys_by_user_id<'id>(
        &self,
        user_id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PasskeyCredential>, DatabaseError>> + Send + 'id>>;

    /// Fetches a list of [`PasskeyCredential`]s belonging to the [`User`] with the given email.
    fn get_passkeys_by_user_email<'email>(
        &self,
        email: &'email str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PasskeyCredential>, DatabaseError>> + Send + 'email>>;

    /// Alters the [`PasskeyCredential`] with the given UUID. Returns the updated
    /// [`PasskeyCredential`] on success.
    fn update_passkey<'key>(
        &self,
        id: &'key Uuid,
        passkey: &'key PasskeyCredentialUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyCredential, DatabaseError>> + Send + 'key>>;

    /// Deletes the [`PasskeyCredential`] with the given UUID.
    fn delete_passkey_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'id>>;

    // Authentication repository

    /// Stores a [passkey registration state object][PasskeyRegistrationState].
    fn create_passkey_registration<'a>(
        &self,
        registration: &'a PasskeyRegistrationState,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'a>>;

    /// Fetches the [`PasskeyRegistrationState`] with the given UUID.
    fn get_passkey_registration_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyRegistrationState, DatabaseError>> + Send + 'id>>;

    /// Stores a [passkey authentication state object][PasskeyAuthenticationState].
    fn create_passkey_authentication<'a>(
        &self,
        state: &'a PasskeyAuthenticationState,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'a>>;

    /// Fetches the [`PasskeyAuthenticationState`] with the given UUID.
    fn get_passkey_authentication_by_id<'id>(
        &self,
        id: &'id Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<PasskeyAuthenticationState, DatabaseError>> + Send + 'id>>;

    // Session repository

    /// Creatse a new authentication [`Session`].
    fn create_session<'a>(
        &self,
        session: &'a Session,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabaseError>> + Send + 'a>>;

    /// Fetches the [`Session`] with the given ID hash.
    fn get_session_by_id_hash<'id>(
        &self,
        id_hash: &'id EncodableHash,
    ) -> Pin<Box<dyn Future<Output = Result<Session, DatabaseError>> + Send + 'id>>;

    /// Alters the [`Session`] with the given ID hash. Returns the updated [`Session`] on success.
    fn update_session<'a>(
        &self,
        id_hash: &'a EncodableHash,
        update: &'a SessionUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<Session, DatabaseError>> + Send + 'a>>;
}

/// Error type for database operations
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// Returned when the given row/resource does not exist.
    #[error("row/resource not found")]
    NotFound,

    /// Returned when a uniqueness constraint is violated. If known, the name of the field/column
    /// for which the constraint was violated can be provided as `field`.
    #[error(
        "uniqueness violation {}{}",
        if field.is_some() { "on field " } else { "(field unknown)" },
        field.as_deref().unwrap_or("")
    )]
    UniquenessViolation {
        /// The field that caused the uniqueness violation, if known
        field: Option<Cow<'static, str>>,
    },

    /// An `update_*()` method was called, but the provided `*Update` struct was empty, i.e.
    /// contained no changes to be made.
    #[error("the update request contains no changes")]
    EmptyUpdate,

    /// An unknown database error occurred. The upstream error is container within the tuple field.
    #[error("database error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),

    /// The given user does not exist.
    #[error("user not found")]
    UserNotFound,
}

#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for DatabaseError {
    /// Converts a [`sqlx::Error`] into either a [`DatabaseError::NotFound`],
    /// a [`DatabaseError::UniquenessViolation`], or a [`DatabaseError::Other`] if neither of the
    /// previous apply.
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
