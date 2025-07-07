use std::sync::Arc;

use axum::{Router, http::header};
use tower::ServiceBuilder;
use tower_http::{
    limit::RequestBodyLimitLayer, sensitive_headers::SetSensitiveHeadersLayer, trace::TraceLayer,
};
use webauthn_rs::Webauthn;

use crate::{db::interface::DatabaseClient, models::AppConfig};

mod middleware;
mod v1;

/// Maximum request payload size in bytes
const MAX_REQUEST_PAYLOAD_BYTES: usize = 8 * 1024; // 8 KiB

/// Creates a new API router with the given database client.
pub fn new_api_router<D: DatabaseClient>(
    db: D,
    webauthn: Webauthn,
    config: AppConfig,
) -> Router<()> {
    let db: Arc<dyn DatabaseClient> = Arc::new(db);
    Router::new()
        .nest_service("/v1", v1::router(db, webauthn, config))
        .layer(
            // order is top to bottom
            ServiceBuilder::new()
                .layer(SetSensitiveHeadersLayer::new(vec![header::AUTHORIZATION]))
                .layer(TraceLayer::new_for_http())
                .layer(RequestBodyLimitLayer::new(MAX_REQUEST_PAYLOAD_BYTES)),
        )
}
