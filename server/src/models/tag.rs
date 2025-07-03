use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::{
    db::interface::{DatabaseClient, DatabaseError},
    models::{ErrNotPopulated, User},
};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tag {
    id: Uuid,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(skip)]
    users: Option<Vec<User>>,
}

impl Tag {
    #[must_use]
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }

    #[must_use]
    pub fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at
    }

    pub fn users(&mut self) -> Result<&[User], ErrNotPopulated> {
        self.users.as_deref().ok_or(ErrNotPopulated)
    }

    pub async fn fetch_users<C>(&mut self, client: C) -> Result<&[User], DatabaseError>
    where
        C: DatabaseClient,
    {
        match self.users {
            Some(ref users) => Ok(users),
            None => {
                let users = client.get_users_by_tag_id(&self.id).await?;
                self.users = Some(users);
                Ok(self.users.as_deref().unwrap())
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TagUpdate {
    pub name: Option<String>,
}

impl TagUpdate {
    pub fn new() -> Self {
        Self { name: None }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none()
    }
}
