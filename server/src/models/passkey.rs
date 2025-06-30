use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasskeyCredential {
    id: Uuid,
    user_id: Uuid,
    credential_id: Vec<u8>,
    public_key: Vec<u8>,
    sign_count: u32,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}


impl PasskeyCredential {
    pub fn new(user_id: Uuid, credential_id: Vec<u8>, public_key: Vec<u8>) -> Self {
        Self {
            id: super::new_uuid(),
            user_id,
            credential_id,
            public_key,
            sign_count: 0,
            created_at: chrono::Utc::now(),
            last_used_at: None,
        }
    }

    pub fn new_full(
        id: Uuid,
        user_id: Uuid,
        credential_id: Vec<u8>,
        public_key: Vec<u8>,
        sign_count: u32,
        created_at: chrono::DateTime<chrono::Utc>,
        last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            credential_id,
            public_key,
            sign_count,
            created_at,
            last_used_at,
        }
    }

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

    pub fn update_sign_count(&mut self, new_count: u32) {
        self.sign_count = new_count;
        self.last_used_at = Some(chrono::Utc::now());
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