use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::interface::{DatabaseClient, DatabaseError},
    models::{ErrNotPopulated, User},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    id: Uuid,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "sqlx", sqlx(skip))]
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
        if let Some(ref users) = self.users {
            Ok(users)
        } else {
            let users = client.get_users_by_tag_id(&self.id).await?;
            self.users = Some(users);
            Ok(self.users.as_deref().unwrap())
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagUpdate {
    pub name: Option<String>,
}

impl TagUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self { name: None }
    }

    #[must_use]
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
    }
}
