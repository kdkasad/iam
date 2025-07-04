use iam_server::{api::ApiServer, db::clients::sqlite::SqliteClient};
use std::process::ExitCode;

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

    let api = ApiServer::new(db);
    api.serve().await.unwrap();

    ExitCode::SUCCESS
}
