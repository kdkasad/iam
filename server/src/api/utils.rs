use std::marker::PhantomData;

use aide::{
    OperationOutput,
    generate::GenContext,
    openapi::{Operation, Response},
};
use axum::{body::Bytes, http::header::CONTENT_TYPE, response::IntoResponse};
use schemars::JsonSchema;
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

/// Implement the same schema as `T`.
impl<T> JsonSchema for PreSerializedJson<T>
where
    T: Serialize + JsonSchema,
{
    fn schema_name() -> std::borrow::Cow<'static, str> {
        T::schema_name()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        T::json_schema(generator)
    }

    fn inline_schema() -> bool {
        T::inline_schema()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        T::schema_id()
    }
}

/// Same effect on the API spec as [`axum::Json<T>`].
impl<T> OperationOutput for PreSerializedJson<T>
where
    T: Serialize + JsonSchema,
{
    type Inner = <axum::Json<T> as OperationOutput>::Inner;

    fn operation_response(ctx: &mut GenContext, operation: &mut Operation) -> Option<Response> {
        <axum::Json<T> as OperationOutput>::operation_response(ctx, operation)
    }

    fn inferred_responses(
        ctx: &mut GenContext,
        operation: &mut Operation,
    ) -> Vec<(Option<u16>, Response)> {
        <axum::Json<T> as OperationOutput>::inferred_responses(ctx, operation)
    }
}
