use uuid::Uuid;

mod passkey;
mod tag;
mod user;

pub use passkey::{
    NewPasskeyCredential, PasskeyCredential, PasskeyCredentialUpdate, PasskeyRegistrationState,
};
pub use tag::{Tag, TagUpdate};
pub use user::{User, UserCreate, UserUpdate};

/// Helper function to generate a new UUID.
/// This allows us to easily switch out the UUID version if needed.
pub fn new_uuid() -> Uuid {
    Uuid::new_v4()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[error("field not populated")]
pub struct ErrNotPopulated;
