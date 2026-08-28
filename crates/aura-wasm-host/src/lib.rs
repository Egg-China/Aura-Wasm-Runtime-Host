#![deny(missing_docs)]

//! Strict descriptor and process supervision for Aura Wasm payloads.

/// Payload descriptor validation.
pub mod descriptor;
/// Stable Host errors.
pub mod error;
/// Package-root path policy.
pub mod path_policy;
