//! # Data models

use uuid::Uuid;

mod config;
mod json;
mod passkey;
mod session;
mod tag;
mod user;

pub use config::*;
pub use json::*;
pub use passkey::*;
pub use session::*;
pub use tag::*;
pub use user::*;

/// Helper function to generate a new UUID.
/// This allows us to easily switch out the UUID version if needed.
#[must_use]
pub fn new_uuid() -> Uuid {
    Uuid::new_v4()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[error("field not populated")]
pub struct ErrNotPopulated;
