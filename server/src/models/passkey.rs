use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasskeyCredential {
    id: Uuid,
    user_id: Uuid,
    display_name: String,
    credential_id: Vec<u8>,
    public_key: Vec<u8>,
    sign_count: u32,
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
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }

    #[must_use]
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    #[must_use]
    pub fn sign_count(&self) -> u32 {
        self.sign_count
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