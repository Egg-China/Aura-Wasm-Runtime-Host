#![deny(missing_docs)]

//! Wasmtime Component Model configuration for Aura payloads.

use std::path::Path;
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

/// Generated bindings for `aura:runtime@0.1.0`.
pub mod bindings;

/// Creates the common Wasmtime engine used by isolated payload processes.
pub fn create_engine() -> wasmtime::Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.consume_fuel(true);
    config.epoch_interruption(true);
    Engine::new(&config)
}

/// Loads and validates one Component Model binary.
pub fn load_component(engine: &Engine, path: &Path) -> wasmtime::Result<Component> {
    Component::from_file(engine, path)
}
