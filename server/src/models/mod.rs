use uuid::Uuid; 

mod user;
mod tag;

/// Helper function to generate a new UUID.
/// This allows us to easily switch out the UUID version if needed.
pub fn new_uuid() -> Uuid {
    Uuid::new_v4()
}

pub use user::{User, PasskeyCredential};
pub use tag::Tag;