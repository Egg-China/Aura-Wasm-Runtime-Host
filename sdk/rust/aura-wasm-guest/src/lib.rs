#![deny(missing_docs)]

//! Canonical Bridge Value v1 helpers for Aura Wasm guests.

mod error;
mod value;

pub use error::{Error, ErrorCode};
pub use value::{HandleValue, Value};

/// Guest-facing name for the canonical Aura Bridge value tree.
pub type AuraValue = Value;

/// Validates that one operation has the exact expected name.
pub fn require_operation(operation: &str, expected: &str) -> Result<(), Error> {
    if operation == expected {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::InvalidArgument))
    }
}
