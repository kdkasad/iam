//! # Helper utilities

use std::marker::PhantomData;

use aide::{
    generate::GenContext,
    openapi::{Operation, Response},
    OperationOutput,
};
use axum::{body::Bytes, http::header::CONTENT_TYPE, response::IntoResponse};
use axum_extra::extract::CookieJar;
use schemars::JsonSchema;
use serde::Serialize;

/// # Pre-serialized JSON response
///
/// This is a helper type for responses which consist of JSON serialized from some static (as in
/// unchanging) shared object. This helper avoids cloning both the object being serialized and the
/// serialized JSON data.
///
/// [`PreSerializedJson::new()`] serializes the input object and stores the
/// resulting JSON buffer, re-using that every time it is converted into a
/// response via [`IntoResponse`].
///
/// [`PreSerializedJson`] is cheaply cloneable and so does not need to be
/// wrapped in an [`Arc`][std::sync::Arc].
///
/// # Examples
///
/// ```ignore
/// # // FIXME: see if we can doctest private items
/// # #[derive(Debug, Clone, Serialize)]
/// # struct SomeTypeThatImplementsSerialize {
/// #     foo: i32,
/// # }
/// let foo = SomeTypeThatImplementsSerialize {
///     bar: 1,
/// };
/// let json = PreSerializedJson::new(&foo);  // serializes `foo` and stores the result
/// let r1 = json.into_response();  // won't clone `foo` or the JSON
/// let json2 = json.clone();  // will only clone the pointer to the JSON, not the contents
/// let r2 = json.into_response();  // won't clone `foo` or the JSON
/// ```
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

/// # Helper type representing a response with cookies
///
/// This type exists because [`aide`] can't deduce the proper response schema for types like
/// a plain [`CookieJar`] or a `(CookieJar, T)` tuple. This type's [`IntoResponse`] implementation
/// behaves identically to `(CookieJar, T)`, but its [`OperationOutput`] implementation delegates
/// to the inner type `T`.
pub struct WithCookies<T> {
    pub response: T,
    pub cookies: CookieJar,
}

impl<T> From<(CookieJar, T)> for WithCookies<T> {
    fn from((cookies, response): (CookieJar, T)) -> Self {
        Self { response, cookies }
    }
}

impl<T> From<WithCookies<T>> for (CookieJar, T) {
    fn from(wc: WithCookies<T>) -> Self {
        (wc.cookies, wc.response)
    }
}

impl From<CookieJar> for WithCookies<()> {
    fn from(cookies: CookieJar) -> Self {
        Self {
            response: (),
            cookies,
        }
    }
}

impl<T> WithCookies<T> {
    pub fn new(cookies: CookieJar, response: T) -> Self {
        (cookies, response).into()
    }
}

impl<T> IntoResponse for WithCookies<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> axum::response::Response {
        (self.cookies, self.response).into_response()
    }
}

impl<T> OperationOutput for WithCookies<T>
where
    T: OperationOutput,
{
    type Inner = T::Inner;

    fn operation_response(ctx: &mut GenContext, operation: &mut Operation) -> Option<Response> {
        T::operation_response(ctx, operation)
    }

    fn inferred_responses(
        ctx: &mut GenContext,
        operation: &mut Operation,
    ) -> Vec<(Option<u16>, Response)> {
        T::inferred_responses(ctx, operation)
    }
}
