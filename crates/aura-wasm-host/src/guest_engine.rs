//! Wasmtime lifecycle adapter for process protocol v1.

use crate::{GuestEngine, HostError, HostResult, PayloadDescriptor};
use aura_runtime_protocol::BridgeTransport;
use aura_wasm_engine::plugin::WasmPlugin;
use aura_wasm_engine::{create_engine, load_component};
use std::path::Path;
use std::sync::Arc;

/// Adapts one real Wasm component to the process server lifecycle.
#[derive(Default)]
pub struct WasmGuestEngine {
    plugin: Option<WasmPlugin>,
}

impl GuestEngine for WasmGuestEngine {
    fn load(
        &mut self,
        package_root: &Path,
        descriptor: &PayloadDescriptor,
        plugin_id: u64,
        session: u64,
        bridge: Arc<dyn BridgeTransport>,
    ) -> HostResult<()> {
        if self.plugin.is_some() {
            return Err(invalid_state());
        }
        let engine = create_engine().map_err(runtime_error)?;
        let component = load_component(&engine, descriptor.component()).map_err(runtime_error)?;
        let mut plugin =
            WasmPlugin::instantiate(engine, package_root, &component, bridge, plugin_id, session)
                .map_err(|error| HostError::new("wit-mismatch", error.to_string()))?;
        require_guest(plugin.load().map_err(runtime_error)?)?;
        self.plugin = Some(plugin);
        Ok(())
    }

    fn enable(&mut self) -> HostResult<()> {
        require_guest(self.plugin_mut()?.enable().map_err(runtime_error)?)
    }

    fn invoke(&mut self, operation: &str, input: &[u8], callback_id: u64) -> HostResult<Vec<u8>> {
        require_guest(
            self.plugin_mut()?
                .invoke(operation, input, callback_id)
                .map_err(runtime_error)?,
        )
    }

    fn disable(&mut self) -> HostResult<()> {
        require_guest(self.plugin_mut()?.disable().map_err(runtime_error)?)
    }

    fn unload(&mut self) -> HostResult<()> {
        let mut plugin = self.plugin.take().ok_or_else(invalid_state)?;
        require_guest(plugin.unload().map_err(runtime_error)?)
    }
}

impl WasmGuestEngine {
    fn plugin_mut(&mut self) -> HostResult<&mut WasmPlugin> {
        self.plugin.as_mut().ok_or_else(invalid_state)
    }
}

fn require_guest<T>(result: Result<T, String>) -> HostResult<T> {
    result.map_err(|message| HostError::new("guest-error", message))
}

fn runtime_error(error: wasmtime::Error) -> HostError {
    let message = error.to_string();
    let code = if message.contains("fuel") {
        "fuel-exhausted"
    } else if message.contains("epoch") || message.contains("deadline") {
        "deadline-exceeded"
    } else if message.contains("limit") || message.contains("memory") {
        "resource-limit"
    } else if message.contains("invalid-value") {
        "invalid-value"
    } else {
        "runtime-failure"
    };
    HostError::new(code, message)
}

fn invalid_state() -> HostError {
    HostError::new("invalid-state", "Wasm payload is not loaded")
}
