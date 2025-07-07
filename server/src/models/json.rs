use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

/// Wrapper type to encode/decode the encapsulated value as JSON text.
#[derive(Debug, Clone, Copy)]
pub struct ViaJson<T>(pub T);

impl<T> From<T> for ViaJson<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for ViaJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ViaJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'de, T> Deserialize<'de> for ViaJson<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <T as Deserialize<'de>>::deserialize(deserializer).map(Self)
    }
}

impl<T> Serialize for ViaJson<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        <T as Serialize>::serialize(&self.0, serializer)
    }
}

/// Delegate decoding to [`sqlx::types::Json`].
#[cfg(feature = "sqlx")]
impl<'r, T, DB> sqlx::Decode<'r, DB> for ViaJson<T>
where
    sqlx::types::Json<T>: sqlx::Decode<'r, DB>,
    DB: sqlx::Database,
{
    fn decode(
        value: <DB as sqlx::Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let sqlx_json = <sqlx::types::Json<T> as sqlx::Decode<'r, DB>>::decode(value)?;
        Ok(ViaJson(sqlx_json.0))
    }
}

/// Delegate type to [`sqlx::types::Json`].
#[cfg(feature = "sqlx")]
impl<T, DB: sqlx::Database> sqlx::Type<DB> for ViaJson<T>
where
    sqlx::types::Json<T>: sqlx::Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <sqlx::types::Json<T> as sqlx::Type<DB>>::type_info()
    }
}

/// Delegate encoding to [`sqlx::types::Json`].
#[cfg(feature = "sqlx")]
impl<'q, T, DB> sqlx::Encode<'q, DB> for ViaJson<T>
where
    DB: sqlx::Database,
    for<'a> sqlx::types::Json<&'a T>: sqlx::Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let sqlx_json = sqlx::types::Json(&self.0);
        <sqlx::types::Json<&T> as sqlx::Encode<'q, DB>>::encode_by_ref(&sqlx_json, buf)
    }
}
