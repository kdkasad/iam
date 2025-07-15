//! # Database backend clients
//!
//! This module contains database clients which implement [`DatabaseClient`] using various database
//! backends.
//!
//! [`DatabaseClient`]: crate::db::interface::DatabaseClient

#[cfg(feature = "sqlite3")]
pub mod sqlite;
