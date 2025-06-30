use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Tag {
    pub fn new(name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: super::new_uuid(),
            name,
            created_at: now,
            updated_at: now,
        }
    }
} 