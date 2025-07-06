use std::sync::Arc;

use axum::{
    Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use tower_http::cors::CorsLayer;
use webauthn_rs::Webauthn;

use crate::{
    db::interface::{DatabaseClient, DatabaseError},
    models::AppConfig,
};

mod auth;
mod config;
mod user;

struct V1StateInner {
    db: Arc<dyn DatabaseClient>,
    webauthn: Webauthn,
    config: Arc<AppConfig>,
}

type V1State = Arc<V1StateInner>;

/// Returns a sub-router for `/api/v1`
pub fn router(db: Arc<dyn DatabaseClient>, webauthn: Webauthn, config: AppConfig) -> Router<()> {
    let router_public: Router<V1State> = Router::new()
        .route("/health", get(async || ()))
        .route("/config", get(config::get_config))
        .layer(CorsLayer::permissive());

    let router_authenticated: Router<V1State> = Router::new()
        .route("/users/{id}", get(user::get_user))
        .route("/users", post(user::post_user))
        .route("/logout", post(auth::logout));

    let router_unauthenticated: Router<V1State> = Router::new()
        .route("/register/start", post(auth::start_registration))
        .route("/register/finish", post(auth::finish_registration))
        .route("/auth/start", post(auth::start_authentication))
        .route("/auth/finish", post(auth::finish_authentication));

    let state = V1StateInner {
        db,
        webauthn,
        config: Arc::new(config),
    };
    router_public
        .merge(router_authenticated)
        .merge(router_unauthenticated)
        .with_state(Arc::new(state))
}

#[derive(Debug, thiserror::Error)]
enum ApiV1Error {
    #[error("Not found")]
    NotFound,

    #[error("WebAuthn error: {0}")]
    WebAuthn(#[from] webauthn_rs::prelude::WebauthnError),

    #[error("Internal server error: {0}")]
    InternalServerError(Box<dyn std::error::Error>),

    #[error("Invalid or missing registration ID cookie")]
    InvalidRegistrationId,

    #[error("Session expired")]
    SessionExpired,

    #[error("Invalid or missing authentication ID cookie")]
    InvalidAuthenticationId,

    #[error("User not found")]
    UserNotFound,
    // #[error("Invalid session ID")]
    // InvalidSessionId,

    // #[error("Not logged in")]
    // NotLoggedIn,
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
        use ApiV1Error::*;
        let status = match self {
            NotFound => StatusCode::NOT_FOUND,
            WebAuthn(_) | InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            InvalidAuthenticationId | InvalidRegistrationId | SessionExpired => {
                StatusCode::BAD_REQUEST
            }
            UserNotFound => StatusCode::NOT_FOUND,
            // NotLoggedIn => StatusCode::UNAUTHORIZED,
        };
        (status, self.to_string()).into_response()
    }
}
