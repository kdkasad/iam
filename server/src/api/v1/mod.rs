use std::sync::Arc;

use axum::{
    http::StatusCode, response::{IntoResponse, Response}, routing::{get, post}, Router
};

use crate::db::interface::{DatabaseClient, DatabaseError};

mod user;

/// Returns a sub-router for `/api/v1`
pub fn router() -> Router<Arc<dyn DatabaseClient>> {
    let router_public = Router::new().route("/health", get(async || ()));

    let router_authenticated = Router::new()
        .route("/users/{id}", get(user::get_user))
        .route("/users", post(user::post_user));
    // ApiRouter::new().api_route("/users/{id}", get(user::get_user));

    router_public.merge(router_authenticated)
}

#[derive(Debug, thiserror::Error)]
enum ApiV1Error {
    #[error("Not found")]
    NotFound,

    #[error("Internal server error: {0}")]
    InternalServerError(Box<dyn std::error::Error>),
}

impl From<DatabaseError> for ApiV1Error {
    fn from(error: DatabaseError) -> Self {
        match error {
            DatabaseError::NotFound => ApiV1Error::NotFound,
            _ => ApiV1Error::InternalServerError(error.into()),
        }
    }
}

impl IntoResponse for ApiV1Error {
    fn into_response(self) -> Response {
        let status = match self {
            ApiV1Error::NotFound => StatusCode::NOT_FOUND,
            ApiV1Error::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}
