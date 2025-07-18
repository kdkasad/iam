use crate::{
    db::interface::{DatabaseClient, DatabaseError},
    models::{ErrNotPopulated, PasskeyCredential, Tag},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
#[serde(rename_all = "camelCase")]
pub struct User {
    id: Uuid,
    email: String,
    display_name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,

    /// List of tags applied to this user. Depending on the database, this can be more expensive to
    /// retrieve than just the base user information, so it is not fetched by default, and will
    /// have a value of [`None`]. If needed, use [`User::fetch_tags()`] to populate.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "sqlx", sqlx(skip))]
    tags: Option<Vec<Tag>>,

    /// List of passkeys belonging to this user. Depending on the database, this can be more
    /// expensive to retrieve than just the base user information, so it is not fetched by default,
    /// and will have a value of [`None`]. If needed, use [`User::fetch_passkeys()`] to populate.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "sqlx", sqlx(skip))]
    passkeys: Option<Vec<PasskeyCredential>>,
}

impl User {
    #[must_use]
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }

    #[must_use]
    pub fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at
    }

    pub fn tags(&mut self) -> Result<&[Tag], ErrNotPopulated> {
        self.tags.as_deref().ok_or(ErrNotPopulated)
    }

    pub async fn fetch_tags(
        &mut self,
        client: &dyn DatabaseClient,
    ) -> Result<&[Tag], DatabaseError> {
        if let Some(ref tags) = self.tags {
            Ok(tags)
        } else {
            let tags = client.get_tags_by_user_id(&self.id).await?;
            self.tags = Some(tags);
            Ok(self.tags.as_deref().unwrap())
        }
    }

    pub async fn fetch_passkeys(
        &mut self,
        client: &dyn DatabaseClient,
    ) -> Result<&[PasskeyCredential], DatabaseError> {
        if let Some(ref passkeys) = self.passkeys {
            Ok(passkeys)
        } else {
            let passkeys = client.get_passkeys_by_user_id(&self.id).await?;
            self.passkeys = Some(passkeys);
            Ok(self.passkeys.as_deref().unwrap())
        }
    }
}

/// Data used to update a user
///
/// Fields with a value will replace the corresponding field's value in the [`User`]
/// to which the update is applied (via [`DatabaseClient::update_user()`][1]).
///
/// [1]: crate::db::interface::DatabaseClient::update_user
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserUpdate {
    pub email: Option<String>,
    pub display_name: Option<String>,
}

impl UserUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self {
            email: None,
            display_name: None,
        }
    }

    #[must_use]
    pub fn with_email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    #[must_use]
    pub fn with_display_name(mut self, display_name: String) -> Self {
        self.display_name = Some(display_name);
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.email.is_none() && self.display_name.is_none()
    }
}

/// Data used to create a user with [`DatabaseClient::create_user()`]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
#[serde(rename_all = "camelCase")]
pub struct UserCreate {
    pub email: String,
    pub display_name: String,
}
