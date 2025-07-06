use axum::http::request::Parts;
use axum_extra::extract::CookieJar;

use crate::{
    api::v1::{ApiV1Error, V1State, auth::SESSION_ID_COOKIE},
    db::interface::DatabaseError,
    models::{EncodableHash, Session, Tag},
};

const ADMIN_TAG_NAME: &str = "iam::admin";

pub struct AuthenticatedSession(pub Session);

impl axum::extract::FromRequestParts<V1State> for AuthenticatedSession {
    type Rejection = ApiV1Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &V1State,
    ) -> Result<Self, Self::Rejection> {
        let cookies = CookieJar::from_request_parts(parts, state).await.unwrap();
        let Some(session_id_cookie) = cookies.get(SESSION_ID_COOKIE) else {
            return Err(ApiV1Error::NotLoggedIn);
        };
        let Ok(session_id_hash) =
            blake3::Hash::from_hex(session_id_cookie.value()).map(EncodableHash)
        else {
            return Err(ApiV1Error::InvalidSessionId);
        };
        match state.db.get_session_by_id_hash(&session_id_hash).await {
            Ok(session) => {
                if session.expires_at < chrono::Utc::now() {
                    return Err(ApiV1Error::SessionExpired);
                }
                Ok(AuthenticatedSession(session))
            }
            Err(DatabaseError::NotFound) => Err(ApiV1Error::NotLoggedIn),
            Err(e) => Err(e.into()),
        }
    }
}

#[allow(dead_code)]
pub struct AdminSession {
    pub session: Session,
    pub tags: Vec<Tag>,
}

impl axum::extract::FromRequestParts<V1State> for AdminSession {
    type Rejection = ApiV1Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &V1State,
    ) -> Result<Self, Self::Rejection> {
        let session = AuthenticatedSession::from_request_parts(parts, state).await?;
        let tags = state.db.get_tags_by_user_id(&session.0.user_id).await?;
        if tags.iter().any(|t| t.name() == ADMIN_TAG_NAME) {
            Ok(AdminSession {
                session: session.0,
                tags,
            })
        } else {
            Err(ApiV1Error::NotAdmin)
        }
    }
}
