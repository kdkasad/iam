use std::borrow::Cow;

use axum::{
    Json,
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::{
    either::Either,
    extract::{
        CookieJar,
        cookie::{Cookie, Expiration, SameSite},
    },
};
use base64::{Engine, prelude::BASE64_STANDARD};
use cookie::{CookieBuilder, time::Duration};
use rand::RngCore;
use serde::Deserialize;
use tracing::{debug, error, warn};
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, DiscoverableKey, Passkey, PublicKeyCredential,
    RegisterPublicKeyCredential, RequestChallengeResponse, WebauthnError,
};
use webauthn_rs_proto::{AuthenticatorSelectionCriteria, ResidentKeyRequirement};

use crate::{
    api::v1::{ApiV1Error, V1State},
    db::interface::DatabaseError,
    models::{
        NewPasskeyCredential, PasskeyAuthenticationState, PasskeyAuthenticationStateType,
        PasskeyRegistrationState, Session, SessionState, User, UserCreate,
    },
};

const REGISTRATION_ID_COOKIE: &str = "registration_id";
const AUTHENTICATION_ID_COOKIE: &str = "authentication_id";
pub const SESSION_ID_COOKIE: &str = "session_id";
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
) -> Result<(CookieJar, Json<CreationChallengeResponse>), ApiV1Error> {
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
        registration: sqlx::types::Json(reg),
        created_at: chrono::Utc::now(),
    };
    state.db.create_passkey_registration(&reg_state).await?;
    Ok((
        cookies.add(
            new_secure_cookie(REGISTRATION_ID_COOKIE, reg_state.id.to_string())
                .expires(Expiration::Session),
        ),
        Json(challenge),
    ))
}

#[derive(Debug, Clone, Deserialize)]
pub struct FinishRegistrationRequest {
    pub user: UserCreate,
    pub passkey: RegisterPublicKeyCredential,
}

pub async fn finish_registration(
    cookies: CookieJar,
    State(state): State<V1State>,
    Json(request): Json<FinishRegistrationRequest>,
) -> Result<(CookieJar, Json<User>), ApiV1Error> {
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
    };
    let (session, session_cookie) = new_session(&user);
    state.db.create_session(&session).await?;
    Ok((
        cookies
            .remove(new_secure_cookie(REGISTRATION_ID_COOKIE, ""))
            .add(session_cookie),
        Json(user),
    ))
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthenticationStartRequest {
    pub email: String,
}

pub async fn start_authentication(
    cookies: CookieJar,
    State(state): State<V1State>,
    Json(request): Json<AuthenticationStartRequest>,
) -> Result<(CookieJar, Json<RequestChallengeResponse>), ApiV1Error> {
    let passkeys: Vec<Passkey> = state
        .db
        .get_passkeys_by_user_email(&request.email)
        .await?
        .into_iter()
        .map(|pk| pk.into())
        .collect();
    let (challenge, auth_state) = state.webauthn.start_passkey_authentication(&passkeys)?;
    let auth_id = Uuid::new_v4();
    let auth_state = PasskeyAuthenticationState {
        id: auth_id,
        email: Some(request.email),
        state: sqlx::types::Json(PasskeyAuthenticationStateType::Regular(auth_state)),
        created_at: chrono::Utc::now(),
    };
    match state.db.create_passkey_authentication(&auth_state).await {
        Ok(_) => (),
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
    ))
}

pub async fn finish_authentication(
    cookies: CookieJar,
    State(state): State<V1State>,
    Json(request): Json<PublicKeyCredential>,
) -> Result<(CookieJar, Json<User>), ApiV1Error> {
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
        // FIXME: update passkey in database
    }
    let Some(email) = auth_state.email else {
        return Err(ApiV1Error::InvalidAuthenticationId);
    };
    let user = state.db.get_user_by_email(&email).await?;
    let (session, session_cookie) = new_session(&user);
    state.db.create_session(&session).await?;
    Ok((
        cookies
            .remove(new_secure_cookie(AUTHENTICATION_ID_COOKIE, ""))
            .add(session_cookie),
        Json(user),
    ))
}

pub async fn start_conditional_ui_authentication(
    State(state): State<V1State>,
    cookies: CookieJar,
) -> Result<(CookieJar, Json<RequestChallengeResponse>), ApiV1Error> {
    let (challenge, disco_state) = state.webauthn.start_discoverable_authentication()?;
    let auth_state = PasskeyAuthenticationState {
        id: Uuid::new_v4(),
        email: None,
        state: sqlx::types::Json(PasskeyAuthenticationStateType::Discoverable(disco_state)),
        created_at: chrono::Utc::now(),
    };
    state.db.create_passkey_authentication(&auth_state).await?;
    Ok((
        cookies.add(
            new_secure_cookie(AUTHENTICATION_ID_COOKIE, auth_state.id.to_string())
                .expires(Expiration::Session),
        ),
        Json(challenge),
    ))
}

pub async fn finish_conditional_ui_authentication(
    State(state): State<V1State>,
    cookies: CookieJar,
    Json(request): Json<PublicKeyCredential>,
) -> Result<(CookieJar, Json<User>), ApiV1Error> {
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
        // FIXME: update passkey in database
    }

    // Create a new session for the user
    let user = state.db.get_user_by_id(&user_id).await?;
    let (session, session_cookie) = new_session(&user);
    state.db.create_session(&session).await?;
    Ok((
        cookies
            .remove(new_secure_cookie(AUTHENTICATION_ID_COOKIE, ""))
            .add(session_cookie),
        Json(user),
    ))
}

fn new_session(user: &User) -> (Session, Cookie<'static>) {
    let mut id = [0u8; 32]; // 256 bits
    rand::rng().fill_bytes(&mut id);
    let id_hash = blake3::hash(&id);
    let session = Session {
        id_hash: id_hash.into(),
        user_id: *user.id(),
        state: SessionState::Active,
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + SESSION_DURATION,
    };
    let cookie =
        new_secure_cookie(SESSION_ID_COOKIE, id_hash.to_string()).max_age(Duration::days(1));
    (session, cookie.into())
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogoutQueryParams {
    pub next: Option<String>,
}

pub async fn logout(
    cookies: CookieJar,
    Query(query): Query<LogoutQueryParams>,
) -> Either<(CookieJar, Redirect), CookieJar> {
    let new_cookies = cookies.remove(new_secure_cookie(SESSION_ID_COOKIE, ""));
    if let Some(next) = query.next {
        Either::E1((new_cookies, Redirect::to(&next)))
    } else {
        Either::E2(new_cookies)
    }
}
