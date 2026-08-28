//! Stable Wasm Host errors.

use std::error::Error;
use std::fmt::{Display, Formatter};

/// One bounded failure reported by the Wasm process Host.
#[derive(Debug)]
pub struct HostError {
    code: &'static str,
    message: String,
}

impl HostError {
    /// Creates one stable Host failure.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Returns the stable lower-case error code.
    pub const fn code(&self) -> &'static str {
        self.code
    }
}

impl Display for HostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl Error for HostError {}

/// Result type used by Wasm Host validation and supervision.
pub type HostResult<T> = Result<T, HostError>;
