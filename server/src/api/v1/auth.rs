use std::borrow::Cow;

use axum::{Json, extract::State};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use rand::RngCore;
use serde::Deserialize;
use tracing::{error, warn};
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, Passkey, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse,
};

use crate::{
    api::v1::{ApiV1Error, V1State},
    db::interface::DatabaseError,
    models::{
        NewPasskeyCredential, PasskeyAuthenticationState, PasskeyRegistrationState, Session,
        SessionState, User, UserCreate,
    },
};

const REGISTRATION_ID_COOKIE: &str = "registration_id";
const AUTHENTICATION_ID_COOKIE: &str = "authentication_id";
const SESSION_ID_COOKIE: &str = "session_id";
const SESSION_DURATION: chrono::Duration = chrono::Duration::days(1);

fn new_secure_cookie<'a, K, V>(name: K, value: V) -> Cookie<'a>
where
    K: Into<Cow<'a, str>>,
    V: Into<Cow<'a, str>>,
{
    Cookie::build((name, value))
        .same_site(SameSite::Strict)
        .http_only(true)
        .secure(true)
        .into()
}

pub async fn start_registration(
    cookies: CookieJar,
    State(state): State<V1State>,
    Json(request): Json<UserCreate>,
) -> Result<(CookieJar, Json<CreationChallengeResponse>), ApiV1Error> {
    let user_id = Uuid::new_v4();
    let (challenge, reg) = state.webauthn.start_passkey_registration(
        user_id,
        &request.email,
        &request.display_name,
        None,
    )?;
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
            Cookie::build((REGISTRATION_ID_COOKIE, reg_state.id.to_string()))
                .same_site(SameSite::Strict)
                .http_only(true)
                .secure(true),
        ),
        Json(challenge),
    ))
}

#[derive(Debug, Clone, Deserialize)]
pub struct FinishRegistrationRequest {
    pub user: UserCreate,
    pub passkey_name: String,
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
        display_name: request.passkey_name,
        passkey,
    };
    let user = state.db.create_user(&Uuid::new_v4(), &request.user).await?;
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
    // FIXME: authenticate new user
    Ok((cookies.remove(REGISTRATION_ID_COOKIE), Json(user)))
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
        .map(|pk| pk.into_passkey())
        .collect();
    let (challenge, auth_state) = state.webauthn.start_passkey_authentication(&passkeys)?;
    let auth_id = Uuid::new_v4();
    let auth_state = PasskeyAuthenticationState {
        id: auth_id,
        email: request.email,
        state: sqlx::types::Json(auth_state),
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
        cookies.add(new_secure_cookie(
            AUTHENTICATION_ID_COOKIE,
            auth_id.to_string(),
        )),
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
    let result = state
        .webauthn
        .finish_passkey_authentication(&request, &auth_state.state)?;
    if result.needs_update() {
        // FIXME: update passkey in database
    }
    let user = state.db.get_user_by_email(&auth_state.email).await?;
    let (session, session_cookie) = new_session(&user);
    state.db.create_session(&session).await?;
    Ok((
        cookies.remove(AUTHENTICATION_ID_COOKIE).add(session_cookie),
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
    let cookie = new_secure_cookie(SESSION_ID_COOKIE, id_hash.to_string());
    (session, cookie)
}
