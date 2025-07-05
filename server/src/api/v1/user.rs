use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    api::v1::{ApiV1Error, V1State},
    models::{User, UserCreate},
};

pub async fn get_user(
    Path(id): Path<Uuid>,
    State(state): State<V1State>,
) -> Result<Json<User>, ApiV1Error> {
    let mut user = state.db.get_user_by_id(&id).await?;
    user.fetch_passkeys(state.db.as_ref()).await?;
    user.fetch_tags(state.db.as_ref()).await?;
    Ok(Json(user))
}

pub async fn post_user(
    State(state): State<V1State>,
    Json(user): Json<UserCreate>,
) -> Result<Json<User>, ApiV1Error> {
    let id = Uuid::new_v4();
    Ok(Json(state.db.create_user(&id, &user).await?))
}
