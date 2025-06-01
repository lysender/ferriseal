pub mod db;
pub mod entry;
pub mod error;
pub mod org;
mod schema;
pub mod user;
pub mod vault;

// Re-export error types for convenience
pub use error::{Error, Result};
