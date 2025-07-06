use axum::Router;
use iam_server::{
    api::new_api_router, db::clients::sqlite::SqliteClient, models::AppConfig, ui::new_ui_server,
};
use std::{env::VarError, ffi::OsString, path::PathBuf, process::ExitCode};
use tokio::net::TcpListener;
use tracing::{error, info, warn};
use webauthn_rs::{WebauthnBuilder, prelude::Url};

mod vars {
    pub const STATIC_DIR: &str = "STATIC_DIR";
    pub const ORIGIN: &str = "ORIGIN";
    pub const SERVER_NAME: &str = "SERVER_NAME";
    pub const RP_ID: &str = "RP_ID";
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
    let config = AppConfig {
        instance_name: match std::env::var(vars::SERVER_NAME) {
            Ok(name) => name,
            Err(VarError::NotPresent) => {
                warn!(
                    "{} is not set; defaulting to {}",
                    vars::SERVER_NAME,
                    &origin
                );
                origin.clone()
            }
            Err(VarError::NotUnicode(_)) => {
                error!("{} is not valid UTF-8", vars::SERVER_NAME);
                return ExitCode::FAILURE;
            }
        },
    };

    // Create database client
    let db = SqliteClient::open().await.unwrap_or_exit(|err| {
        error!("failed to open database: {err}");
    });

    // Create WebAuthn client
    let parsed_origin = match Url::parse(&origin) {
        Ok(origin) => origin,
        Err(err) => {
            error!("failed to create URL from given origin: {err}");
            return ExitCode::FAILURE;
        }
    };
    let rp_id = std::env::var(vars::RP_ID).unwrap_or_else(|err| match err {
        VarError::NotPresent => parsed_origin.to_string(),
        VarError::NotUnicode(os_string) => {
            error!("{} is not valid UTF-8: {os_string:?}", vars::RP_ID);
            std::process::exit(1);
        }
    });
    info!("Creating WebAuthn manager with RP ID {rp_id} and origin {parsed_origin}");
    let webauthn = WebauthnBuilder::new(&rp_id, &parsed_origin)
        .unwrap()
        .rp_name(&config.instance_name)
        .build()
        .unwrap_or_exit(|err| error!("failed to build WebAuthn manager: {err}"));

    let api = new_api_router(db, webauthn, config);

    let static_dir = PathBuf::from(std::env::var_os(vars::STATIC_DIR).unwrap_or_else(|| {
        warn!(
            "{} is not set; using default of {}",
            vars::STATIC_DIR,
            defaults::STATIC_DIR
        );
        OsString::from(defaults::STATIC_DIR)
    }));
    let ui = new_ui_server(&static_dir);

    let router = Router::new().nest("/api", api).fallback_service(ui);

    let listener = TcpListener::bind(defaults::LISTEN_ADDR)
        .await
        .unwrap_or_exit(|err| {
            error!("failed to listen on {}: {err}", defaults::LISTEN_ADDR);
        });
    axum::serve(listener, router).await.unwrap_or_exit(|err| {
        error!("failed to start server: {err}");
    });

    ExitCode::SUCCESS
}

/// Calls [`std::env::var(name)`][std::env::var] and if that fails, exits the program after printing an error message.
fn getenv_or_exit(name: &str) -> String {
    std::env::var(name).unwrap_or_exit(|_| {
        error!("{name} is not set");
    })
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
