use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::Tag;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub tags: Vec<Tag>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub passkeys: Vec<PasskeyCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyCredential {
    pub id: Uuid,
    pub user_id: Uuid,
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub sign_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl User {
    pub fn new(email: String, display_name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: super::new_uuid(),
            email,
            display_name,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            passkeys: Vec::new(),
        }
    }

    fn update(&mut self) {
        self.updated_at = chrono::Utc::now();
    }

    pub fn add_tag(&mut self, tag: Tag) {
        self.tags.push(tag);
        self.update();
    }

    pub fn remove_tag(&mut self, tag_id: Uuid) {
        self.tags.retain(|tag| tag.id != tag_id);
        self.update();
    }

    pub fn has_tag(&self, tag_name: &str) -> bool {
        self.tags.iter().any(|tag| tag.name == tag_name)
    }
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

    pub fn update_sign_count(&mut self, new_count: u32) {
        self.sign_count = new_count;
        self.last_used_at = Some(chrono::Utc::now());
    }
} 