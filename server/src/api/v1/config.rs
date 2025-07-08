use axum::extract::State;

use crate::{
    api::{utils::PreSerializedJson, v1::V1State},
    models::AppConfig,
};

pub async fn get_config(State(state): State<V1State>) -> PreSerializedJson<AppConfig> {
    state.config.clone()
}
