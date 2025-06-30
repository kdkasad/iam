use uuid::Uuid;

use crate::models::{PasskeyCredential, Tag, TagUpdate, User, UserUpdate};

pub trait DatabaseClient {
    type Error: std::error::Error + Send + Sync + 'static;

    // User repository
    async fn create_user(&self, user: &User) -> Result<(), Self::Error>;
    async fn get_user_by_id(&self, id: &Uuid) -> Result<User, Self::Error>;
    async fn get_user_by_email(&self, email: &str) -> Result<User, Self::Error>;
    async fn update_user(&self, id: &Uuid, update: &UserUpdate) -> Result<User, Self::Error>;
    async fn delete_user_by_id(&self, id: &Uuid) -> Result<(), Self::Error>;
    async fn add_tag_to_user(&self, user_id: &Uuid, tag: &Tag) -> Result<(), Self::Error>;
    async fn remove_tag_from_user(&self, user_id: &Uuid, tag: &Tag) -> Result<(), Self::Error>;
    async fn get_users_by_tag_id(&self, tag_id: &Uuid) -> Result<Vec<User>, Self::Error>;

    // Tag repository
    async fn create_tag(&self, tag: &Tag) -> Result<(), Self::Error>;
    async fn get_tag_by_id(&self, id: &Uuid) -> Result<Tag, Self::Error>;
    async fn get_tag_by_name(&self, name: &str) -> Result<Tag, Self::Error>;
    async fn update_tag(&self, id: &Uuid, update: &TagUpdate) -> Result<Tag, Self::Error>;
    async fn delete_tag_by_id(&self, id: &Uuid) -> Result<(), Self::Error>;
    async fn get_tags_by_user_id(&self, user_id: &Uuid) -> Result<Vec<Tag>, Self::Error>;

    // Passkey repository
    async fn create_passkey(&self, passkey: &PasskeyCredential) -> Result<(), Self::Error>;
    async fn get_passkey_by_id(&self, id: &Uuid) -> Result<PasskeyCredential, Self::Error>;
    async fn get_passkey_by_credential_id(
        &self,
        credential_id: &[u8],
    ) -> Result<PasskeyCredential, Self::Error>;
    async fn get_passkeys_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<PasskeyCredential>, Self::Error>;
    async fn update_passkey(
        &self,
        passkey: &PasskeyCredential,
    ) -> Result<PasskeyCredential, Self::Error>;
    async fn delete_passkey_by_id(&self, id: &Uuid) -> Result<(), Self::Error>;
    async fn increment_passkey_sign_count(&self, id: &Uuid) -> Result<(), Self::Error>;
}
