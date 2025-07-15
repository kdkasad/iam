use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Session state
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum SessionState {
    /// Session is active and usable
    Active,
    /// Session was revoked by the user from another device
    Revoked,
    /// Session was canceled due to the user logging out
    LoggedOut,
    /// Session was upgraded or downgraded
    Superseded,
}

/// # Login session
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// [`blake3`] hash of the session ID
    #[serde(skip)]
    pub id_hash: EncodableHash,
    /// UUID of the [`User`][super::User] to which this session belongs
    #[serde(skip)]
    pub user_id: Uuid,
    /// State of the session
    pub state: SessionState,
    /// Time at which the session was created
    pub created_at: DateTime<Utc>,
    /// Time at which the session expires
    pub expires_at: DateTime<Utc>,
    /// Whether this session has admin privileges
    pub is_admin: bool,
    /// [`blake3`] hash of the session ID of this session's parent, if it has one
    #[serde(skip)]
    pub parent_id_hash: Option<EncodableHash>,
}

/// Data used to update a session
///
/// Fields with a value will replace the corresponding field's value in the [`Session`]
/// to which the update is applied (via [`DatabaseClient::update_session()`][1]).
///
/// [1]: crate::db::interface::DatabaseClient::update_session
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct SessionUpdate {
    pub state: Option<SessionState>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl SessionUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_state(mut self, state: SessionState) -> Self {
        self.state = Some(state);
        self
    }

    #[must_use]
    pub fn with_expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.state.is_none() && self.expires_at.is_none()
    }
}

mod encodable_hash {
    //! # Encodable hash helper

    #[cfg(feature = "sqlx")]
    use std::borrow::Cow;
    use std::ops::{Deref, DerefMut};

    use serde::{Deserialize, Serialize};
    #[cfg(feature = "sqlx")]
    use sqlx::{
        encode::IsNull,
        error::BoxDynError,
        sqlite::{SqliteArgumentValue, SqliteValueRef},
    };

    /// # Encodable hash helper wrapper
    ///
    /// [`EncodableHash`] is a wrapper around [`blake3::Hash`] which implements [`sqlx::Encode`],
    /// [`sqlx::Decode`], and [`sqlx::Type`] if the `sqlx` feature is enabled. The value is
    /// encoded/decoded as a binary blob.
    #[repr(transparent)]
    // don't derive PartialEq or Eq to ensure we use constant-time comparison
    #[derive(Debug, Copy, Clone, Serialize, Deserialize)]
    pub struct EncodableHash(pub blake3::Hash);

    #[cfg(feature = "sqlx")]
    impl sqlx::Type<sqlx::Sqlite> for EncodableHash {
        fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
            <&[u8] as sqlx::Type<sqlx::Sqlite>>::type_info()
        }
    }

    #[cfg(feature = "sqlx")]
    impl sqlx::Decode<'_, sqlx::Sqlite> for EncodableHash {
        fn decode(value: SqliteValueRef<'_>) -> Result<Self, BoxDynError> {
            let bytes = <&[u8] as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
            Ok(Self(blake3::Hash::from_slice(bytes)?))
        }
    }

    #[cfg(feature = "sqlx")]
    impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for EncodableHash {
        fn encode_by_ref(
            &self,
            buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
        ) -> Result<sqlx::encode::IsNull, BoxDynError> {
            buf.push(SqliteArgumentValue::Blob(Cow::Owned(
                self.0.as_bytes().to_vec(),
            )));
            Ok(IsNull::No)
        }
    }

    impl Deref for EncodableHash {
        type Target = blake3::Hash;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for EncodableHash {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl From<blake3::Hash> for EncodableHash {
        fn from(hash: blake3::Hash) -> Self {
            Self(hash)
        }
    }

    impl From<EncodableHash> for blake3::Hash {
        fn from(hash: EncodableHash) -> Self {
            hash.0
        }
    }
}
pub use encodable_hash::EncodableHash;
