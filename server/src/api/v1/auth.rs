use axum::{Json, extract::State};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use serde::Deserialize;
use tracing::{error, warn};
use uuid::Uuid;
use webauthn_rs::prelude::{CreationChallengeResponse, RegisterPublicKeyCredential};

use crate::{
    api::v1::{ApiV1Error, V1State},
    models::{NewPasskeyCredential, PasskeyRegistrationState, User, UserCreate},
};

const REGISTRATION_ID_COOKIE: &str = "registration_id";

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

// pub struct AuthenticationStartRequest {
//     pub email: String,
// }

// pub async fn start_authentication(
//     cookies: CookieJar,
//     State(state): State<V1State>,
//     Json(request): Json<AuthenticationStartRequest>,
// ) -> Result<(), ApiV1Error> {
//     let passkeys: Vec<Passkey> = state
//         .db
//         .get_passkeys_by_user_email(&request.email)
//         .await?
//         .into_iter()
//         .map(|pk| pk.into_passkey())
//         .collect();
//     let (challenge, state) = state.webauthn.start_passkey_authentication(&passkeys);
//     todo!()
// }
