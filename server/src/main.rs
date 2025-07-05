use axum::Router;
use iam_server::{api::new_api_router, db::clients::sqlite::SqliteClient, ui::new_ui_server};
use std::{path::PathBuf, process::ExitCode};
use tokio::net::TcpListener;
use tracing::{error, warn};

mod vars {
    pub const STATIC_DIR: &str = "STATIC_DIR";
}

mod defaults {
    pub const STATIC_DIR: &str = "./ui/build";
    pub const LISTEN_ADDR: &str = "0.0.0.0:3000";
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt().init();

    let db = match SqliteClient::open().await {
        Ok(db) => db,
        Err(err) => {
            tracing::error!("failed to open database: {err}");
            return ExitCode::FAILURE;
        }
    };

    let api = new_api_router(db);

    let static_dir = match std::env::var_os(vars::STATIC_DIR) {
        Some(dir) => PathBuf::from(dir),
        None => {
            let path = PathBuf::from(defaults::STATIC_DIR);
            warn!("STATIC_DIR not set; using default of {}", path.display());
            path
        }
    };
    let ui = new_ui_server(&static_dir);

    let router = Router::new().nest("/api", api).nest_service("/ui", ui);

    let listener = match TcpListener::bind(defaults::LISTEN_ADDR).await {
        Ok(l) => l,
        Err(err) => {
            error!("failed to listen on {}: {err}", defaults::LISTEN_ADDR);
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = axum::serve(listener, router).await {
        error!("failed to start server: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
