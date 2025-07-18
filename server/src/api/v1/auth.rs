//! # v1 authentication-related API endpoint handlers

use std::borrow::Cow;

use axum::{Json, extract::State};
use axum_extra::extract::{
    Cached, CookieJar,
    cookie::{Cookie, Expiration, SameSite},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use cookie::{CookieBuilder, time::Duration};
use rand::RngCore;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};
use uuid::Uuid;
use webauthn_rs::prelude::{
    AuthenticationResult, CreationChallengeResponse, DiscoverableKey, Passkey, PublicKeyCredential,
    RegisterPublicKeyCredential, RequestChallengeResponse, WebauthnError,
};
use webauthn_rs_proto::{AuthenticatorSelectionCriteria, ResidentKeyRequirement};

use crate::{
    api::{utils::WithCookies, v1::{extractors::AuthenticatedSession, ApiV1Error, V1State}},
    db::interface::{DatabaseClient, DatabaseError},
    models::{
        NewPasskeyCredential, PasskeyAuthenticationState, PasskeyAuthenticationStateType,
        PasskeyCredentialUpdate, PasskeyRegistrationState, Session, SessionState, SessionUpdate,
        User, UserCreate, ViaJson,
    },
};

const REGISTRATION_ID_COOKIE: &str = "registration_id";
const AUTHENTICATION_ID_COOKIE: &str = "authentication_id";
pub const SESSION_ID_COOKIE: &str = "session_id";
const IS_ADMIN_COOKIE: &str = "session_is_admin";
const SESSION_DURATION: chrono::Duration = chrono::Duration::days(1);

fn new_secure_cookie<'a, K, V>(name: K, value: V) -> CookieBuilder<'a>
where
    K: Into<Cow<'a, str>>,
    V: Into<Cow<'a, str>>,
{
    Cookie::build((name, value))
        .same_site(SameSite::Strict)
        .http_only(true)
        .secure(true)
        .path("/")
}

pub async fn start_registration(
    cookies: CookieJar,
    State(state): State<V1State>,
    Json(request): Json<UserCreate>,
) -> Result<WithCookies<Json<CreationChallengeResponse>>, ApiV1Error> {
    let user_id = Uuid::new_v4();
    let (mut challenge, reg) = state.webauthn.start_passkey_registration(
        user_id,
        &request.email,
        &request.display_name,
        None,
    )?;

    // Prefer resident keys
    challenge.public_key.authenticator_selection = Some(AuthenticatorSelectionCriteria {
        resident_key: Some(ResidentKeyRequirement::Preferred),
        ..Default::default()
    });

    let reg_state = PasskeyRegistrationState {
        id: Uuid::new_v4(),
        user_id,
        email: request.email,
        registration: ViaJson(reg),
        created_at: chrono::Utc::now(),
    };
    state.db.create_passkey_registration(&reg_state).await?;
    Ok((
        cookies.add(
            new_secure_cookie(REGISTRATION_ID_COOKIE, reg_state.id.to_string())
                .expires(Expiration::Session),
        ),
        Json(challenge),
    ).into())
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FinishRegistrationRequest {
    pub user: UserCreate,
    pub passkey: RegisterPublicKeyCredential,
}

pub async fn finish_registration(
    cookies: CookieJar,
    State(state): State<V1State>,
    Json(request): Json<FinishRegistrationRequest>,
) -> Result<WithCookies<Json<User>>, ApiV1Error> {
    let Some(registration_id_cookie) = cookies.get("registration_id") else {
        return Err(ApiV1Error::InvalidRegistrationId);
    };
    let Ok(registration_id) = Uuid::parse_str(registration_id_cookie.value()) else {
        return Err(ApiV1Error::InvalidRegistrationId);
    };
    let reg_state = state
        .db
        .get_passkey_registration_by_id(&registration_id)
        .await?;
    let five_minutes_ago = chrono::Utc::now() - chrono::Duration::minutes(5);
    if reg_state.created_at < five_minutes_ago {
        return Err(ApiV1Error::SessionExpired);
    }
    let passkey = state
        .webauthn
        .finish_passkey_registration(&request.passkey, &reg_state.registration)?;
    let new_passkey = NewPasskeyCredential {
        display_name: None,
        passkey,
    };
    let user = state
        .db
        .create_user(&reg_state.user_id, &request.user)
        .await?;
    match state
        .db
        .create_passkey(&Uuid::new_v4(), user.id(), &new_passkey)
        .await
    {
        Ok(_passkey) => (),
        Err(err) => {
            warn!(
                "Passkey creation failed after user creation succeeded for {}: {err}",
                user.email()
            );
            // If the passkey creation fails, the whole registration is invalidated.
            if let Err(e2) = state.db.delete_user_by_id(user.id()).await {
                error!("Failed to delete user after passkey creation failure: {e2}");
            }
            return Err(err.into());
        }
    }
    let (_session, cookies) = new_session(cookies, &*state.db, user.id(), false, None).await?;
    Ok((
        cookies.remove(new_secure_cookie(REGISTRATION_ID_COOKIE, "")),
        Json(user),
    ).into())
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AuthenticationStartRequest {
    pub email: String,
}

pub async fn start_authentication(
    cookies: CookieJar,
    State(state): State<V1State>,
    Json(request): Json<AuthenticationStartRequest>,
) -> Result<WithCookies<Json<RequestChallengeResponse>>, ApiV1Error> {
    let passkeys: Vec<Passkey> = state
        .db
        .get_passkeys_by_user_email(&request.email)
        .await?
        .into_iter()
        .map(std::convert::Into::into)
        .collect();
    let (challenge, auth_state) = state.webauthn.start_passkey_authentication(&passkeys)?;
    let auth_id = Uuid::new_v4();
    let auth_state = PasskeyAuthenticationState {
        id: auth_id,
        email: Some(request.email),
        state: ViaJson(PasskeyAuthenticationStateType::Regular(auth_state)),
        created_at: chrono::Utc::now(),
    };
    match state.db.create_passkey_authentication(&auth_state).await {
        Ok(()) => (),
        Err(DatabaseError::UserNotFound) => {
            return Err(ApiV1Error::UserNotFound);
        }
        Err(e) => return Err(e.into()),
    }
    Ok((
        cookies.add(
            new_secure_cookie(AUTHENTICATION_ID_COOKIE, auth_id.to_string())
                .expires(Expiration::Session),
        ),
        Json(challenge),
    ).into())
}

pub async fn finish_authentication(
    cookies: CookieJar,
    State(state): State<V1State>,
    Json(request): Json<PublicKeyCredential>,
) -> Result<WithCookies<Json<User>>, ApiV1Error> {
    let Some(authentication_id_cookie) = cookies.get(AUTHENTICATION_ID_COOKIE) else {
        return Err(ApiV1Error::InvalidAuthenticationId);
    };
    let Ok(authentication_id) = Uuid::parse_str(authentication_id_cookie.value()) else {
        return Err(ApiV1Error::InvalidAuthenticationId);
    };
    let auth_state = state
        .db
        .get_passkey_authentication_by_id(&authentication_id)
        .await?;
    let five_minutes_ago = chrono::Utc::now() - chrono::Duration::minutes(5);
    if auth_state.created_at < five_minutes_ago {
        return Err(ApiV1Error::SessionExpired);
    }
    let PasskeyAuthenticationStateType::Regular(passkey_state) = auth_state.state.0 else {
        return Err(ApiV1Error::InvalidAuthenticationId);
    };
    let result = state
        .webauthn
        .finish_passkey_authentication(&request, &passkey_state)?;
    if result.needs_update() {
        do_passkey_update(&state, &result).await?;
    }
    let Some(email) = auth_state.email else {
        return Err(ApiV1Error::InvalidAuthenticationId);
    };
    let user = state.db.get_user_by_email(&email).await?;
    let (_session, cookies) = new_session(cookies, &*state.db, user.id(), false, None).await?;
    Ok((
        cookies.remove(new_secure_cookie(AUTHENTICATION_ID_COOKIE, "")),
        Json(user),
    ).into())
}

async fn do_passkey_update(
    state: &V1State,
    result: &AuthenticationResult,
) -> Result<(), DatabaseError> {
    debug!(
        "Updating passkey for credential ID {}",
        BASE64_STANDARD.encode(result.cred_id())
    );
    let mut passkey = state
        .db
        .get_passkey_by_credential_id(result.cred_id())
        .await?;
    if let Some(true) = passkey.passkey.update_credential(result) {
        state
            .db
            .update_passkey(
                &passkey.id,
                &PasskeyCredentialUpdate::new().with_passkey(passkey.passkey),
            )
            .await?;
    }
    Ok(())
}

pub async fn start_conditional_ui_authentication(
    State(state): State<V1State>,
    cookies: CookieJar,
) -> Result<WithCookies<Json<RequestChallengeResponse>>, ApiV1Error> {
    let (challenge, disco_state) = state.webauthn.start_discoverable_authentication()?;
    let auth_state = PasskeyAuthenticationState {
        id: Uuid::new_v4(),
        email: None,
        state: ViaJson(PasskeyAuthenticationStateType::Discoverable(disco_state)),
        created_at: chrono::Utc::now(),
    };
    state.db.create_passkey_authentication(&auth_state).await?;
    Ok((
        cookies.add(
            new_secure_cookie(AUTHENTICATION_ID_COOKIE, auth_state.id.to_string())
                .expires(Expiration::Session),
        ),
        Json(challenge),
    ).into())
}

pub async fn finish_conditional_ui_authentication(
    State(state): State<V1State>,
    cookies: CookieJar,
    Json(request): Json<PublicKeyCredential>,
) -> Result<WithCookies<Json<User>>, ApiV1Error> {
    // Get the authentication ID from the cookie
    let Some(auth_id_cookie) = cookies.get(AUTHENTICATION_ID_COOKIE) else {
        debug!("No auth ID cookie found");
        return Err(ApiV1Error::InvalidAuthenticationId);
    };
    let Ok(auth_id) = Uuid::parse_str(auth_id_cookie.value()) else {
        debug!("Invalid auth ID cookie value: {}", auth_id_cookie.value());
        return Err(ApiV1Error::InvalidAuthenticationId);
    };

    // Get the passkey from the credential ID in the request
    let (user_id, cred_id) = state
        .webauthn
        .identify_discoverable_authentication(&request)?;
    let auth_state = match state.db.get_passkey_authentication_by_id(&auth_id).await {
        Ok(auth_state) => auth_state,
        Err(DatabaseError::NotFound) => {
            debug!("Auth state not found for ID {auth_id}");
            return Err(ApiV1Error::InvalidAuthenticationId);
        }
        Err(e) => return Err(e.into()),
    };
    let passkey = match state.db.get_passkey_by_credential_id(cred_id).await {
        Ok(passkey) => passkey,
        Err(DatabaseError::NotFound) => {
            debug!(
                "Passkey not found for credential ID {}",
                BASE64_STANDARD.encode(cred_id)
            );
            return Err(ApiV1Error::InvalidAuthenticationId);
        }
        Err(e) => return Err(e.into()),
    };
    let PasskeyAuthenticationStateType::Discoverable(disco_state) = auth_state.state.0 else {
        debug!("Auth state is not a discoverable state");
        return Err(ApiV1Error::InvalidAuthenticationId);
    };

    // Finish the authentication
    let discoverable_key = DiscoverableKey::from(passkey.passkey.0);
    let result = state
        .webauthn
        .finish_discoverable_authentication(&request, disco_state, &[discoverable_key])
        .map_err(ApiV1Error::AuthFailed)?;

    // Ensure the user ID the user presented matches the one the passkey belongs to
    if passkey.user_id != user_id {
        debug!("Expected user ID {} but got {}", passkey.user_id, user_id);
        return Err(ApiV1Error::AuthFailed(WebauthnError::InvalidUserUniqueId));
    }

    if result.needs_update() {
        do_passkey_update(&state, &result).await?;
    }

    // Create a new session for the user
    let user = state.db.get_user_by_id(&user_id).await?;
    let (_session, cookies) = new_session(cookies, &*state.db, user.id(), false, None).await?;
    Ok((
        cookies.remove(new_secure_cookie(AUTHENTICATION_ID_COOKIE, "")),
        Json(user),
    ).into())
}

async fn new_session(
    mut cookies: CookieJar,
    db: &dyn DatabaseClient,
    user_id: &Uuid,
    is_admin: bool,
    parent: Option<&Session>,
) -> Result<(Session, CookieJar), DatabaseError> {
    // Create session
    let mut id = [0u8; 32]; // 256 bits
    rand::rng().fill_bytes(&mut id);
    let id_hash = blake3::hash(&id);
    let session = Session {
        id_hash: id_hash.into(),
        user_id: *user_id,
        state: SessionState::Active,
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + SESSION_DURATION,
        is_admin,
        parent_id_hash: parent.map(|p| p.id_hash),
    };

    // Store session in database
    db.create_session(&session).await?;

    // Set session cookie
    cookies = cookies
        .add(new_secure_cookie(SESSION_ID_COOKIE, id_hash.to_string()).max_age(Duration::days(1)));

    // Set admin marker cookie.
    // admin cookie is not HTTP-only so the UI can detect whether the session is admin or not.
    let is_admin_cookie = new_secure_cookie(IS_ADMIN_COOKIE, "y").http_only(false);
    cookies = if is_admin {
        cookies.add(is_admin_cookie)
    } else {
        cookies.remove(is_admin_cookie)
    };

    Ok((session, cookies))
}

pub async fn logout(
    State(state): State<V1State>,
    AuthenticatedSession(session): AuthenticatedSession,
    Cached(cookies): Cached<CookieJar>,
) -> Result<WithCookies<()>, ApiV1Error> {
    let session = state.db.get_session_by_id_hash(&session.id_hash).await?;
    if session.state == SessionState::Active {
        state
            .db
            .update_session(
                &session.id_hash,
                &SessionUpdate::new().with_state(SessionState::LoggedOut),
            )
            .await?;
    }
    let new_cookies = cookies.remove(new_secure_cookie(SESSION_ID_COOKIE, ""));
    Ok(new_cookies.into())
}

/// Describes what kind of session upgrade to perform.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(tag = "target")]
pub enum UpgradeTarget {
    Admin,
    // User { user_id: Uuid },
}

/// Upgrades a session, e.g. from regular user to admin privileges.
pub async fn upgrade_session(
    State(state): State<V1State>,
    Cached(cookies): Cached<CookieJar>,
    AuthenticatedSession(session): AuthenticatedSession,
    Json(target): Json<UpgradeTarget>,
) -> Result<WithCookies<()>, ApiV1Error> {
    // Check if user has admin tag
    let tags = state.db.get_tags_by_user_id(&session.user_id).await?;
    if !tags
        .iter()
        .map(|t| &*t.name)
        .any(|tag_name| tag_name == "iam::admin")
    {
        return Err(ApiV1Error::NotAdmin);
    }

    match target {
        UpgradeTarget::Admin => {
            // Create new admin session
            let (_session, cookies) =
                new_session(cookies, &*state.db, &session.user_id, true, Some(&session)).await?;
            // Invalidate current session
            supersede_session(&*state.db, &session).await?;
            Ok(cookies.into())
        }
    }
}

/// Downgrade a session that was previously upgraded.
pub async fn downgrade_session(
    State(state): State<V1State>,
    Cached(mut cookies): Cached<CookieJar>,
    AuthenticatedSession(session): AuthenticatedSession,
) -> Result<WithCookies<()>, ApiV1Error> {
    if let Some(parent_id_hash) = session.parent_id_hash {
        let parent_session = state.db.get_session_by_id_hash(&parent_id_hash).await?;
        // We can't actually return to the parent session since we don't know the non-hashed ID, so we
        // create a new one with the same privileges.
        (_, cookies) = new_session(
            cookies,
            &*state.db,
            &parent_session.user_id,
            parent_session.is_admin,
            Some(&session),
        )
        .await?;
        // Invalidate the current session
        supersede_session(&*state.db, &session).await?;
        Ok(cookies.into())
    } else {
        Err(ApiV1Error::DowngradeImpossible)
    }
}

/// Mark the given session as ugraded/downgraded.
async fn supersede_session(
    db: &dyn DatabaseClient,
    session: &Session,
) -> Result<(), DatabaseError> {
    db.update_session(
        &session.id_hash,
        &SessionUpdate {
            state: Some(SessionState::Superseded),
            expires_at: None,
        },
    )
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct UserAndSessionInfo {
    pub user: User,
    pub session: Session,
}

/// Return the currently logged in user and session.
pub async fn get_session(
    State(state): State<V1State>,
    AuthenticatedSession(session): AuthenticatedSession,
) -> Result<Json<UserAndSessionInfo>, ApiV1Error> {
    let mut user = state.db.get_user_by_id(&session.user_id).await?;
    user.fetch_tags(&*state.db).await?;
    Ok(Json(UserAndSessionInfo { user, session }))
}
