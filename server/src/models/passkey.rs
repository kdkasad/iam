use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, types::Json};
use uuid::Uuid;
use webauthn_rs::prelude::{
    DiscoverableAuthentication, Passkey, PasskeyAuthentication, PasskeyRegistration,
};

pub type WrappedPasskey = Json<Passkey>;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasskeyCredential {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub passkey: WrappedPasskey,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<PasskeyCredential> for Passkey {
    fn from(value: PasskeyCredential) -> Self {
        value.passkey.0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PasskeyCredentialUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<Option<String>>,
}

impl PasskeyCredentialUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_display_name(mut self, display_name: Option<String>) -> Self {
        self.display_name = Some(display_name);
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.display_name.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPasskeyCredential {
    pub display_name: Option<String>,
    pub passkey: Passkey,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasskeyRegistrationState {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub registration: Json<PasskeyRegistration>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasskeyAuthenticationState {
    pub id: Uuid,
    pub email: Option<String>,
    pub state: Json<PasskeyAuthenticationStateType>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PasskeyAuthenticationStateType {
    Discoverable(DiscoverableAuthentication),
    Regular(PasskeyAuthentication),
}
