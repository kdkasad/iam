use axum::{
    Router,
    http::{
        HeaderValue,
        header::{
            CONTENT_SECURITY_POLICY, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
        },
    },
};
#[cfg(feature = "sqlite3")]
use iam_server::db::clients::sqlite::SqliteClient;
use iam_server::{
    api::new_api_router, db::interface::DatabaseClient, models::AppConfig, ui::new_ui_server,
};
use std::{env::VarError, ffi::OsString, path::PathBuf, process::ExitCode, sync::Arc};
use tokio::net::TcpListener;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::{error, info, warn};
use webauthn_rs::{WebauthnBuilder, prelude::Url};

mod vars {
    pub const STATIC_DIR: &str = "STATIC_DIR";
    pub const ORIGIN: &str = "ORIGIN";
    pub const SERVER_NAME: &str = "SERVER_NAME";
    pub const RP_ID: &str = "RP_ID";
    pub const DB_BACKEND: &str = "DB_BACKEND";
}

mod defaults {
    pub const STATIC_DIR: &str = "./ui/build";
    pub const LISTEN_ADDR: &str = "0.0.0.0:3000";
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt().init();

    // Create server config
    let origin = getenv_or_exit(vars::ORIGIN);
    let parsed_origin = match Url::parse(&origin) {
        Ok(origin) => origin,
        Err(err) => {
            error!(%origin, %err, "failed to create URL from given origin");
            return ExitCode::FAILURE;
        }
    };
    let config = AppConfig {
        instance_name: match std::env::var(vars::SERVER_NAME) {
            Ok(name) => name,
            Err(VarError::NotPresent) => {
                let default = parsed_origin.authority();
                warn!(
                    var = %vars::SERVER_NAME,
                    %default,
                    "variable not set; using default",
                );
                origin.clone()
            }
            Err(VarError::NotUnicode(_)) => {
                error!(var = %vars::SERVER_NAME, "environment variable is not valid UTF-8");
                return ExitCode::FAILURE;
            }
        },
    };

    // Create database client
    let db = match get_db_client().await {
        Ok(db) => db,
        Err(choice_str) => {
            error!(choice = %choice_str, "invalid database backend choice");
            return ExitCode::FAILURE;
        }
    };

    // Create WebAuthn client
    let rp_id = std::env::var(vars::RP_ID).unwrap_or_else(|err| match err {
        VarError::NotPresent => parsed_origin.to_string(),
        VarError::NotUnicode(_) => {
            error!(var = %vars::RP_ID, "environment variable is not valid UTF-8");
            std::process::exit(1);
        }
    });
    info!(%rp_id, origin = %parsed_origin, "creating WebAuthn manager");
    let webauthn = WebauthnBuilder::new(&rp_id, &parsed_origin)
        .unwrap()
        .rp_name(&config.instance_name)
        .build()
        .unwrap_or_exit(|err| error!(%err, "failed to build WebAuthn manager"));

    let api = new_api_router(db, webauthn, config);

    let static_dir = PathBuf::from(std::env::var_os(vars::STATIC_DIR).unwrap_or_else(|| {
        warn!(
            var = %vars::STATIC_DIR,
            default = %defaults::STATIC_DIR,
            "variable not set; using default",
        );
        OsString::from(defaults::STATIC_DIR)
    }));
    let ui = new_ui_server(&static_dir);

    let router = Router::new()
        .nest("/api", api)
        .fallback_service(ui)
        .layer(SetResponseHeaderLayer::if_not_present(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("frame-ancestors 'none'"),
        ));

    let listener = TcpListener::bind(defaults::LISTEN_ADDR)
        .await
        .unwrap_or_exit(|err| {
            error!(%err, address = %defaults::LISTEN_ADDR, "failed to start listener");
        });
    axum::serve(listener, router).await.unwrap_or_exit(|err| {
        error!(%err, "failed to start server");
    });

    ExitCode::SUCCESS
}

/// Calls [`std::env::var(name)`][std::env::var] and if that fails, exits the program after printing an error message.
fn getenv_or_exit(name: &str) -> String {
    std::env::var(name).unwrap_or_exit(|_| {
        error!(var = %name, "environment variable is not set");
    })
}

// Allow lints that happen when all database backend features are disabled.
#[allow(clippy::unused_async, unused_variables, unreachable_code)]
async fn get_db_client() -> Result<Arc<dyn DatabaseClient>, String> {
    let db_choice = getenv_or_exit(vars::DB_BACKEND);
    let db: Arc<dyn DatabaseClient> = match db_choice.as_str() {
        #[cfg(feature = "sqlite3")]
        "sqlite3" | "sqlite" => Arc::new(SqliteClient::open().await.unwrap_or_exit(|err| {
            error!(%err, "failed to open database");
        })),
        _ => return Err(db_choice),
    };
    Ok(db)
}

trait UnwrapOrExit<T, E> {
    /// Unwraps the result, or calls the given function with the error and exits the program with an exit code of 1.
    fn unwrap_or_exit(self, f: impl FnOnce(E)) -> T;
}

impl<T, E> UnwrapOrExit<T, E> for Result<T, E> {
    fn unwrap_or_exit(self, f: impl FnOnce(E)) -> T {
        match self {
            Ok(value) => value,
            Err(err) => {
                f(err);
                std::process::exit(1);
            }
        }
    }
}
