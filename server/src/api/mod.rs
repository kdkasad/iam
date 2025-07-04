use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;

use crate::db::interface::DatabaseClient;

mod v1;

/// IAM API server
pub struct ApiServer {
    router: Router<()>,
}

impl ApiServer {
    pub fn new<D: DatabaseClient>(db: D) -> Self {
        let db: Arc<dyn DatabaseClient> = Arc::new(db);
        let router: Router<()> =
            Router::new().nest_service("/api/v1", v1::router().with_state(Arc::clone(&db)));

        Self { router }
    }

    pub async fn serve(self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind("0.0.0.0:3000").await?;
        axum::serve(listener, self.router.into_make_service()).await
    }
}
