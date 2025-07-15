#![expect(clippy::doc_markdown)]

//! # OpenAPI specification generator
//!
//! This binary generates an OpenAPI specification from [`iam_server`]'s API handlers.
//! The generated spec is written as JSON to the standard output stream.

use std::sync::Arc;

use iam_server::{api::new_api_router, db::clients::sqlite::SqliteClient, models::AppConfig};
use webauthn_rs::WebauthnBuilder;

#[tokio::main]
async fn main() {
    let db = Arc::new(SqliteClient::new_memory().await.unwrap());
    let webauthn = WebauthnBuilder::new("localhost", &"http://localhost:3000".parse().unwrap())
        .unwrap()
        .rp_name("IAM")
        .build()
        .unwrap();
    let config = AppConfig {
        instance_name: "IAM".to_string(),
    };
    aide::generate::on_error(|err| {
        eprintln!("Error: {err}");
        std::process::exit(1);
    });
    let (_router, specs) = new_api_router(db, webauthn, &config);
    for spec in specs.to_vec() {
        println!("{}", serde_json::to_string(&spec).unwrap());
    }
}
