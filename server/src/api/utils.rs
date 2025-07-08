use std::marker::PhantomData;

use axum::{body::Bytes, http::header::CONTENT_TYPE, response::IntoResponse};
use serde::Serialize;

/// Helper response type for pre-serialized JSON data.
///
/// [`PreSerializedJson::new()`] serializes the input object and stores the
/// resulting JSON buffer, re-using that every time it is converted into a
/// response via [`IntoResponse`].
///
/// [`PreSerializedJson`] is cheaply cloneable and so does not need to be
/// wrapped in an [`Arc`][std::sync::Arc].
#[derive(Debug, Clone)]
pub struct PreSerializedJson<T: ?Sized + Serialize> {
    json_bytes: Bytes,
    type_marker: PhantomData<T>,
}

impl<T: ?Sized + Serialize> PreSerializedJson<T> {
    pub fn new(value: &T) -> Result<Self, serde_json::Error> {
        Ok(Self {
            json_bytes: Bytes::from_owner(serde_json::to_vec(value)?),
            type_marker: PhantomData,
        })
    }
}

impl<T: ?Sized + Serialize> IntoResponse for PreSerializedJson<T> {
    fn into_response(self) -> axum::response::Response {
        ([(CONTENT_TYPE, "application/json")], self.json_bytes).into_response()
    }
}
