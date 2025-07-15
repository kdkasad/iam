use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "sqlx")]
use sqlx::prelude::FromRow;
use uuid::Uuid;
use webauthn_rs::prelude::{
    DiscoverableAuthentication, Passkey, PasskeyAuthentication, PasskeyRegistration,
};

use crate::models::ViaJson;

/// # Passkey credential
///
/// Stores the data needed to maintain and use a passkey for user authentication.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
#[serde(rename_all = "camelCase")]
pub struct PasskeyCredential {
    /// Unique ID
    pub id: Uuid,
    /// UUID of the user to which this passkey belongs
    pub user_id: Uuid,
    /// Display name of this passkey, if set
    pub display_name: Option<String>,
    /// Opaque [`Passkey`] data from [`webauthn_rs`]
    #[schemars(skip)]
    pub passkey: ViaJson<Passkey>,
    /// Time at which this passkey was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Time at which this passkey was last used to log in
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<PasskeyCredential> for Passkey {
    fn from(value: PasskeyCredential) -> Self {
        value.passkey.0
    }
}

/// Data used to update a [`PasskeyCredential`].
///
/// Fields with a value will replace the corresponding field's value in the [`PasskeyCredential`]
/// to which the update is applied (via [`DatabaseClient::update_passkey()`][1]).
///
/// [1]: crate::db::interface::DatabaseClient::update_passkey
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

/// Data used to create a new [`PasskeyCredential`] with [`DatabaseClient::create_passkey()`][1]
///
/// [1]: crate::db::interface::DatabaseClient::create_passkey
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPasskeyCredential {
    pub display_name: Option<String>,
    pub passkey: Passkey,
}

/// Object storing the server-side state for an in-progress passkey registration
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

/// Object storing the server-side state for an in-progress passkey login
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
#[serde(rename_all = "camelCase")]
pub struct PasskeyAuthenticationState {
    pub id: Uuid,
    pub email: Option<String>,
    pub state: ViaJson<PasskeyAuthenticationStateType>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Type of passkey login being performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PasskeyAuthenticationStateType {
    Discoverable(DiscoverableAuthentication),
    Regular(PasskeyAuthentication),
}
