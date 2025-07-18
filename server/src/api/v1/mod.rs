//! v1 API implementation

use std::sync::Arc;

use aide::{
    OperationOutput,
    axum::{
        ApiRouter,
        routing::{get, post},
    },
    generate::GenContext,
    openapi::{
        ApiKeyLocation, MediaType, OpenApi, Operation, Response as OapiResponse, SecurityScheme,
    },
};
use axum::{
    Extension, Router,
    http::{HeaderValue, Method, StatusCode, header::VARY},
    response::{IntoResponse, Response},
};
use chrono::Duration;
use tower_http::{
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
};
use webauthn_rs::Webauthn;

use crate::{
    api::{middleware::CacheControlLayer, utils::PreSerializedJson},
    db::interface::{DatabaseClient, DatabaseError},
    models::AppConfig,
};

use super::middleware::Publicity;

mod auth;
mod config;
mod extractors;
mod user;

struct V1StateInner {
    db: Arc<dyn DatabaseClient>,
    webauthn: Webauthn,
    config: PreSerializedJson<AppConfig>,
}

type V1State = Arc<V1StateInner>;

/// Returns a sub-router for `/api/v1` and its [`OpenApi`] specification.
///
/// # Panics
///
/// Panics if serializing the given `config` into JSON fails.
pub fn router_and_spec(
    db: Arc<dyn DatabaseClient>,
    webauthn: Webauthn,
    config: &AppConfig,
) -> (Router<()>, OpenApi) {
    // Public (cross-origin allowed) router
    let router_public: ApiRouter<V1State> = ApiRouter::new()
        .api_route("/health", get(async || ()))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Method::GET)
                .allow_credentials(false),
        );

    // Router for endpoints whose responses depend on authentication state.
    let router_auth: ApiRouter<V1State> = ApiRouter::new()
        .api_route("/users/{id}", get(user::get_user))
        .api_route("/users", post(user::post_user))
        .api_route("/users/me", get(user::get_current_user))
        .api_route("/logout", post(auth::logout))
        .api_route("/register/start", post(auth::start_registration))
        .api_route("/register/finish", post(auth::finish_registration))
        .api_route("/auth/start", post(auth::start_authentication))
        .api_route("/auth/finish", post(auth::finish_authentication))
        .api_route(
            "/auth/discoverable/start",
            post(auth::start_conditional_ui_authentication),
        )
        .api_route(
            "/auth/discoverable/finish",
            post(auth::finish_conditional_ui_authentication),
        )
        .api_route("/auth/upgrade", post(auth::upgrade_session))
        .api_route("/auth/downgrade", post(auth::downgrade_session))
        .api_route("/auth/session", get(auth::get_session))
        .layer(SetResponseHeaderLayer::appending(
            VARY,
            HeaderValue::from_static("Cookie"),
        ))
        .layer(CacheControlLayer::new().no_store(true).finish());

    // Router for endpoints whose responses do not depend on authentication state.
    let mut router_unauthenticated: ApiRouter<V1State> = ApiRouter::new()
        .api_route("/config", get(config::get_config))
        .api_route("/docs/openapi.json", get(get_openapi_json));

    // If the `scalar` feature is enabled, add the Scalar UI to the unauthenticated router
    #[cfg(feature = "scalar")]
    {
        use aide::scalar::Scalar;
        router_unauthenticated = router_unauthenticated.route(
            "/docs",
            Scalar::new("/api/v1/docs/openapi.json").axum_route(),
        );
    }

    // Allow clients/proxies to cache for up to 24 hours
    router_unauthenticated = router_unauthenticated.layer(
        CacheControlLayer::new()
            .publicity(Publicity::Public)
            .max_age(Duration::hours(24))
            .finish(),
    );

    let state = V1StateInner {
        db,
        webauthn,
        config: PreSerializedJson::new(config).expect("serializing app config failed"),
    };
    let mut openapi = OpenApi::default();
    let mut router = router_public
        .merge(router_auth)
        .merge(router_unauthenticated)
        .with_state(Arc::new(state))
        .finish_api_with(&mut openapi, |api| {
            api.security_scheme(
                "userSession",
                SecurityScheme::ApiKey {
                    location: ApiKeyLocation::Cookie,
                    name: "session_id".to_string(),
                    description: Some("A cookie containing the user's session ID. This is automatically set by the server when the user logs in.".to_string()),
                    #[allow(clippy::default_trait_access, reason = "using the type would require a direct dependency on indexmap")]
                    extensions: Default::default(),
                },
            )
        });

    // Add OpenAPI spec JSON to the router
    router = router.route_layer(Extension(
        PreSerializedJson::new(&openapi).expect("serializing OpenAPI spec failed"),
    ));

    (router, openapi)
}

/// # Error type for the v1 API
///
/// Implements [`IntoResponse`], thus returning a response with a sensible status code when used as
/// the return type of a handler. Currently, the response body is a plain text error message, but
/// that will change to JSON in the future.
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

    #[error("Invalid session ID")]
    InvalidSessionId,

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("Not an administrator")]
    NotAdmin,

    #[error("Authentication failed: {0}")]
    AuthFailed(#[source] webauthn_rs::prelude::WebauthnError),

    #[error("Session downgrade impossible")]
    DowngradeImpossible,
}

impl From<DatabaseError> for ApiV1Error {
    fn from(error: DatabaseError) -> Self {
        match error {
            DatabaseError::NotFound => ApiV1Error::NotFound,
            _ => ApiV1Error::InternalServerError(error.into()),
        }
    }
}

impl ApiV1Error {
    fn possible_status_codes() -> Vec<StatusCode> {
        vec![
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_REQUEST,
            StatusCode::NOT_FOUND,
            StatusCode::UNAUTHORIZED,
        ]
    }
}

impl IntoResponse for ApiV1Error {
    fn into_response(self) -> Response {
        #[allow(clippy::enum_glob_use)]
        use ApiV1Error::*;
        let status = match self {
            WebAuthn(_) | InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            InvalidAuthenticationId
            | InvalidRegistrationId
            | InvalidSessionId
            | DowngradeImpossible => StatusCode::BAD_REQUEST,
            UserNotFound | NotFound => StatusCode::NOT_FOUND,
            NotLoggedIn | SessionExpired | NotAdmin | AuthFailed(_) => StatusCode::UNAUTHORIZED,
        };
        (status, self.to_string()).into_response()
    }
}

impl OperationOutput for ApiV1Error {
    type Inner = Self;

    fn operation_response(
        _ctx: &mut GenContext,
        _operation: &mut Operation,
    ) -> Option<OapiResponse> {
        Some(OapiResponse {
            description: "Error response".to_string(),
            content: [(
                "text/plain".to_string(),
                MediaType {
                    example: Some("Not logged in".into()),
                    ..Default::default()
                },
            )]
            .into(),
            ..Default::default()
        })
    }

    fn inferred_responses(
        ctx: &mut GenContext,
        operation: &mut Operation,
    ) -> Vec<(Option<u16>, OapiResponse)> {
        Self::possible_status_codes()
            .into_iter()
            .map(|status| {
                (
                    Some(status.as_u16()),
                    Self::operation_response(ctx, operation).unwrap(),
                )
            })
            .collect()
    }
}

async fn get_openapi_json(
    Extension(api): Extension<PreSerializedJson<OpenApi>>,
) -> PreSerializedJson<OpenApi> {
    api
}
