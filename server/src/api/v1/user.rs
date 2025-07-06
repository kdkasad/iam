use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    api::v1::{
        ApiV1Error, V1State,
        extractors::{AdminSession, AuthenticatedSession},
    },
    models::{User, UserCreate},
};

pub async fn get_user(
    AdminSession { .. }: AdminSession,
    Path(id): Path<Uuid>,
    State(state): State<V1State>,
) -> Result<Json<User>, ApiV1Error> {
    let mut user = state.db.get_user_by_id(&id).await?;
    user.fetch_passkeys(state.db.as_ref()).await?;
    user.fetch_tags(state.db.as_ref()).await?;
    Ok(Json(user))
}

pub async fn post_user(
    AdminSession { .. }: AdminSession,
    State(state): State<V1State>,
    Json(user): Json<UserCreate>,
) -> Result<Json<User>, ApiV1Error> {
    let id = Uuid::new_v4();
    Ok(Json(state.db.create_user(&id, &user).await?))
}

pub async fn get_current_user(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<V1State>,
) -> Result<Json<User>, ApiV1Error> {
    let mut user = state.db.get_user_by_id(&session.user_id).await?;
    user.fetch_passkeys(state.db.as_ref()).await?;
    user.fetch_tags(state.db.as_ref()).await?;
    Ok(Json(user))
}
