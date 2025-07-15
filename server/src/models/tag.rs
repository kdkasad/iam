use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::interface::{DatabaseClient, DatabaseError},
    models::User,
};

/// # Tag model
///
/// A tag is a marker which can be applied to [`User`]s.
/// Tags can be applied to multiple users, and users can each have multiple tags.
///
/// Tags are used to grant privileges/permissions to users. For example, the built-in `iam::admin`
/// tag allows users to act as an administrator and manage other users in the IAM portal.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    /// Unique identifier
    pub id: Uuid,
    /// Tag name (must also be unique)
    pub name: String,
    /// Time at which the tag was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Time at which the tag was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// List of users to which this tag is applied. Depending on the database, this can be more
    /// expensive to retrieve than just the tag information, so it is not fetched by default, and
    /// will have a value of [`None`]. If needed, use [`Tag::fetch_users()`] to populate.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "sqlx", sqlx(skip))]
    pub users: Option<Vec<User>>,
}

impl Tag {
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

/// Data used to update a tag
///
/// Fields with a value will replace the corresponding field's value in the [`Tag`]
/// to which the update is applied (via [`DatabaseClient::update_tag()`][1]).
///
/// [1]: crate::db::interface::DatabaseClient::update_tag
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
