use crate::{
    db::interface::DatabaseClient,
    models::{ErrNotPopulated, PasskeyCredential, Tag},
};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    id: Uuid,
    email: String,
    display_name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(skip)]
    tags: Option<Vec<Tag>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(skip)]
    passkeys: Option<Vec<PasskeyCredential>>,
}

impl User {
    pub fn new(email: String, display_name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: super::new_uuid(),
            email,
            display_name,
            created_at: now,
            updated_at: now,
            tags: None,
            passkeys: None,
        }
    }

    pub fn new_full(
        id: Uuid,
        email: String,
        display_name: String,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
        tags: Option<Vec<Tag>>,
        passkeys: Option<Vec<PasskeyCredential>>,
    ) -> Self {
        Self {
            id,
            email,
            display_name,
            created_at,
            updated_at,
            tags,
            passkeys,
        }
    }

    fn update(&mut self) {
        self.updated_at = chrono::Utc::now();
    }

    #[must_use]
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn set_email(&mut self, email: String) {
        self.email = email;
        self.update();
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn set_display_name(&mut self, display_name: String) {
        self.display_name = display_name;
        self.update();
    }

    #[must_use]
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }

    #[must_use]
    pub fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at
    }

    pub fn add_tag(&mut self, tag: Tag) -> Result<(), ErrNotPopulated> {
        self.tags.as_mut().ok_or(ErrNotPopulated)?.push(tag);
        self.update();
        Ok(())
    }

    pub fn remove_tag(&mut self, tag_id: &Uuid) -> Result<(), ErrNotPopulated> {
        self.tags
            .as_mut()
            .ok_or(ErrNotPopulated)?
            .retain(|tag| tag.id() != tag_id);
        self.update();
        Ok(())
    }

    #[must_use]
    pub fn tags(&mut self) -> Result<&[Tag], ErrNotPopulated> {
        self.tags.as_deref().ok_or(ErrNotPopulated)
    }

    pub async fn fetch_tags<C>(&mut self, client: C) -> Result<&[Tag], C::Error>
    where
        C: DatabaseClient,
    {
        match self.tags {
            Some(ref tags) => Ok(tags),
            None => {
                let tags = client.get_tags_by_user_id(&self.id).await?;
                self.tags = Some(tags);
                Ok(self.tags.as_deref().unwrap())
            }
        }
    }

    pub async fn fetch_passkeys<C>(&mut self, client: C) -> Result<&[PasskeyCredential], C::Error>
    where
        C: DatabaseClient,
    {
        match self.passkeys {
            Some(ref passkeys) => Ok(passkeys),
            None => {
                let passkeys = client.get_passkeys_by_user_id(&self.id).await?;
                self.passkeys = Some(passkeys);
                Ok(self.passkeys.as_deref().unwrap())
            }
        }
    }
}