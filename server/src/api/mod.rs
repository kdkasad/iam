use std::sync::Arc;

use aide::openapi::OpenApi;
use axum::{Router, http::header};
use tower::ServiceBuilder;
use tower_http::{
    limit::RequestBodyLimitLayer, sensitive_headers::SetSensitiveHeadersLayer, trace::TraceLayer,
};
use webauthn_rs::Webauthn;

use crate::{db::interface::DatabaseClient, models::AppConfig};

mod middleware;
mod utils;
mod v1;

/// Maximum request payload size in bytes
const MAX_REQUEST_PAYLOAD_BYTES: usize = 8 * 1024; // 8 KiB

/// A collection of API specifications.
#[derive(Debug, Clone)]
pub struct ApiSpecs {
    pub v1: OpenApi,
}

impl ApiSpecs {
    #[must_use]
    pub fn to_vec(self) -> Vec<OpenApi> {
        vec![self.v1]
    }
}

impl From<ApiSpecs> for Vec<OpenApi> {
    fn from(val: ApiSpecs) -> Self {
        val.to_vec()
    }
}

/// Creates a new API router with the given database client.
pub fn new_api_router(
    db: Arc<dyn DatabaseClient>,
    webauthn: Webauthn,
    config: &AppConfig,
) -> (Router<()>, ApiSpecs) {
    let (v1_router, v1_spec) = v1::router_and_spec(db, webauthn, config);
    let router = Router::new().nest_service("/v1", v1_router).layer(
        // order is top to bottom
        ServiceBuilder::new()
            .layer(SetSensitiveHeadersLayer::new(vec![header::AUTHORIZATION]))
            .layer(TraceLayer::new_for_http())
            .layer(RequestBodyLimitLayer::new(MAX_REQUEST_PAYLOAD_BYTES)),
    );
    (router, ApiSpecs { v1: v1_spec })
}
