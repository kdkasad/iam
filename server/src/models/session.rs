use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[repr(u8)]
pub enum SessionState {
    Active,
    Revoked,
    LoggedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Session {
    pub id_hash: EncodableHash,
    pub user_id: Uuid,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

    // don't derive PartialEq or Eq to ensure we use constant-time comparison
    #[derive(Debug, Copy, Clone, Serialize, Deserialize)]
    #[repr(transparent)]
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
