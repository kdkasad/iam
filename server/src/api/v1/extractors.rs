//! # Custom extractors for the v1 API

use aide::{OperationInput, openapi::SecurityRequirement};
use axum::{RequestPartsExt, http::request::Parts};
use axum_extra::extract::{Cached, CookieJar};

use crate::{
    api::v1::{ApiV1Error, V1State, auth::SESSION_ID_COOKIE},
    db::interface::DatabaseError,
    models::{EncodableHash, Session, SessionState},
};

/// # Authenticated session extractor
///
/// [`AuthenticatedSession`] retrieves the client's session ID from the `session_id` cookie,
/// fetches the session from the database, and validates it to ensure it's active and has not
/// expired. If this succeeds, the validated [`Session`] is returned by the extractor.
///
/// If validation fails, one of the following errors is returned:
/// - [`ApiV1Error::NotLoggedIn`] if there is no session ID cookie
/// - [`ApiV1Error::InvalidSessionId`] if the session ID cookie contains an invalid/unparseable value
/// - [`ApiV1Error::SessionExpired`] if the session is expired or canceled
/// - [`ApiV1Error::InternalServerError`] if a [`DatabaseError`] occurs
#[derive(Debug, Clone)]
pub struct AuthenticatedSession(pub Session);

impl axum::extract::FromRequestParts<V1State> for AuthenticatedSession {
    type Rejection = ApiV1Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &V1State,
    ) -> Result<Self, Self::Rejection> {
        // Get session ID hash from cookie
        let Cached(cookies): Cached<CookieJar> = parts.extract_with_state(state).await.unwrap();
        let Some(session_id_cookie) = cookies.get(SESSION_ID_COOKIE) else {
            return Err(ApiV1Error::NotLoggedIn);
        };
        let Ok(session_id_hash) =
            blake3::Hash::from_hex(session_id_cookie.value()).map(EncodableHash)
        else {
            return Err(ApiV1Error::InvalidSessionId);
        };

        // Look up session in database
        match state.db.get_session_by_id_hash(&session_id_hash).await {
            Ok(session) => {
                // Ensure session is active and not expired
                if session.state != SessionState::Active || session.expires_at < chrono::Utc::now()
                {
                    Err(ApiV1Error::SessionExpired)
                } else {
                    Ok(AuthenticatedSession(session))
                }
            }
            Err(DatabaseError::NotFound) => Err(ApiV1Error::NotLoggedIn),
            Err(e) => Err(e.into()),
        }
    }
}

impl OperationInput for AuthenticatedSession {
    fn operation_input(
        _ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        let security = SecurityRequirement::from([("userSession".to_string(), vec![])]);
        if !operation.security.contains(&security) {
            operation.security.push(security);
        }
    }
}

/// # Administrator session extractor
///
/// [`AdminSession`] is a wrapper around [`AuthenticatedSession`]. It behaves identically, except
/// it also ensures that the client's session is an administrator session ([`Session::is_admin`]),
/// returning [`ApiV1Error::NotAdmin`] if not.
#[derive(Debug, Clone)]
#[expect(dead_code)]
pub struct AdminSession(pub Session);

impl axum::extract::FromRequestParts<V1State> for AdminSession {
    type Rejection = ApiV1Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &V1State,
    ) -> Result<Self, Self::Rejection> {
        // Get authenticated session
        let AuthenticatedSession(session) = parts.extract_with_state(state).await?;
        // Ensure session has admin privilege
        if session.is_admin {
            Ok(AdminSession(session))
        } else {
            Err(ApiV1Error::NotAdmin)
        }
    }
}

impl OperationInput for AdminSession {
    fn operation_input(
        _ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        let security = SecurityRequirement::from([("adminSession".to_string(), vec![])]);
        if !operation.security.contains(&security) {
            operation.security.push(security);
        }
    }
}
