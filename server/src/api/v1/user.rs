use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    api::v1::ApiV1Error,
    db::interface::DatabaseClient,
    models::{User, UserUpdate},
};

pub async fn get_user(
    Path(id): Path<Uuid>,
    State(db): State<Arc<dyn DatabaseClient>>,
) -> Result<Json<User>, ApiV1Error> {
    let mut user = db.get_user_by_id(&id).await?;
    user.fetch_passkeys(db.as_ref()).await?;
    user.fetch_tags(db.as_ref()).await?;
    Ok(Json(user))
}

pub async fn post_user(
    State(db): State<Arc<dyn DatabaseClient>>,
    Json(user): Json<UserUpdate>,
) -> Result<Json<User>, ApiV1Error> {
    let id = Uuid::new_v4();
    Ok(Json(db.create_user(&id, &user).await?))
}
