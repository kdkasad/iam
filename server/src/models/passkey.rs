use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "sqlx")]
use sqlx::prelude::FromRow;
use uuid::Uuid;
use webauthn_rs::prelude::{
    DiscoverableAuthentication, Passkey, PasskeyAuthentication, PasskeyRegistration,
};

use crate::models::ViaJson;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
#[serde(rename_all = "camelCase")]
pub struct PasskeyCredential {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: Option<String>,
    #[schemars(skip)]
    pub passkey: ViaJson<Passkey>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<PasskeyCredential> for Passkey {
    fn from(value: PasskeyCredential) -> Self {
        value.passkey.0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyCredentialUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passkey: Option<ViaJson<Passkey>>,
}

impl PasskeyCredentialUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_display_name(mut self, display_name: Option<impl ToString>) -> Self {
        self.display_name = Some(display_name.map(|v| v.to_string()));
        self
    }

    #[must_use]
    pub fn with_passkey<P>(mut self, passkey: P) -> Self
    where
        P: Into<ViaJson<Passkey>>,
    {
        self.passkey = Some(passkey.into());
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.display_name.is_none() && self.passkey.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPasskeyCredential {
    pub display_name: Option<String>,
    pub passkey: Passkey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
#[serde(rename_all = "camelCase")]
pub struct PasskeyRegistrationState {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub registration: ViaJson<PasskeyRegistration>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
#[serde(rename_all = "camelCase")]
pub struct PasskeyAuthenticationState {
    pub id: Uuid,
    pub email: Option<String>,
    pub state: ViaJson<PasskeyAuthenticationStateType>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PasskeyAuthenticationStateType {
    Discoverable(DiscoverableAuthentication),
    Regular(PasskeyAuthentication),
}
