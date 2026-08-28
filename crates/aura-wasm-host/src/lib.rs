#![deny(missing_docs)]

//! Strict descriptor and process supervision for Aura Wasm payloads.

/// Payload descriptor validation.
pub mod descriptor;
/// Stable Host errors.
pub mod error;
mod guest_engine;
/// Package-root path policy.
pub mod path_policy;
mod server;

pub use descriptor::PayloadDescriptor;
pub use error::{HostError, HostResult};
pub use guest_engine::WasmGuestEngine;
pub use server::{GuestEngine, ProcessServer};
