use std::sync::Arc;

use axum::{Router, http::header};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    limit::RequestBodyLimitLayer, sensitive_headers::SetSensitiveHeadersLayer, trace::TraceLayer,
};

use crate::db::interface::DatabaseClient;

mod v1;

/// Maximum request payload size in bytes
const MAX_REQUEST_PAYLOAD_BYTES: usize = 8 * 1024; // 8 KiB

/// IAM API server
pub struct ApiServer {
    router: Router<()>,
}

impl ApiServer {
    pub fn new<D: DatabaseClient>(db: D) -> Self {
        let db: Arc<dyn DatabaseClient> = Arc::new(db);
        let router: Router<()> = Router::new()
            .nest_service("/api/v1", v1::router().with_state(Arc::clone(&db)))
            .layer(
                // order is top to bottom
                ServiceBuilder::new()
                    .layer(SetSensitiveHeadersLayer::new(vec![header::AUTHORIZATION]))
                    .layer(TraceLayer::new_for_http())
                    .layer(RequestBodyLimitLayer::new(MAX_REQUEST_PAYLOAD_BYTES)),
            );

        Self { router }
    }

    pub async fn serve(self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind("0.0.0.0:3000").await?;
        axum::serve(listener, self.router.into_make_service()).await
    }
}
