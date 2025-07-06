use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, types::Json};
use uuid::Uuid;
use webauthn_rs::prelude::{Passkey, PasskeyAuthentication, PasskeyRegistration};

pub type WrappedPasskey = Json<Passkey>;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasskeyCredential {
    id: Uuid,
    user_id: Uuid,
    display_name: String,
    passkey: WrappedPasskey,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl PasskeyCredential {
    #[must_use]
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    #[must_use]
    pub fn user_id(&self) -> &Uuid {
        &self.user_id
    }

    #[must_use]
    pub fn passkey(&self) -> &Passkey {
        &self.passkey
    }

    pub fn into_passkey(self) -> Passkey {
        self.passkey.0
    }

    #[must_use]
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }

    #[must_use]
    pub fn last_used_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.last_used_at
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PasskeyCredentialUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

impl PasskeyCredentialUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_display_name(mut self, display_name: String) -> Self {
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
    pub display_name: String,
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
    pub email: String,
    pub state: Json<PasskeyAuthentication>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
