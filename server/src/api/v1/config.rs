use std::sync::Arc;

use axum::{Json, extract::State};

use crate::{api::v1::V1State, models::AppConfig};

pub async fn get_config(State(state): State<V1State>) -> Json<Arc<AppConfig>> {
    Json(Arc::clone(&state.config))
}
