use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[repr(u8)]
pub enum SessionState {
    Active,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id_hash: EncodableHash,
    pub user_id: Uuid,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

mod encodable_hash {
    use std::{
        borrow::Cow,
        ops::{Deref, DerefMut},
    };

    use serde::{Deserialize, Serialize};
    use sqlx::{
        encode::IsNull,
        error::BoxDynError,
        sqlite::{SqliteArgumentValue, SqliteValueRef},
    };

    // don't derive PartialEq or Eq to ensure we use constant-time comparison
    #[derive(Debug, Copy, Clone, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct EncodableHash(pub blake3::Hash);

    impl sqlx::Type<sqlx::Sqlite> for EncodableHash {
        fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
            <&[u8] as sqlx::Type<sqlx::Sqlite>>::type_info()
        }
    }

    impl sqlx::Decode<'_, sqlx::Sqlite> for EncodableHash {
        fn decode(value: SqliteValueRef<'_>) -> Result<Self, BoxDynError> {
            let bytes = <&[u8] as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
            Ok(Self(blake3::Hash::from_slice(bytes)?))
        }
    }

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
