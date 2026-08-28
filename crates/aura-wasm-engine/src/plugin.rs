//! Typed Component Model lifecycle execution.

use crate::bindings::AuraPluginV1;
use crate::config::{CALL_TIMEOUT, FUEL_PER_CALL};
use crate::store::{HostState, create_store};
use aura_bridge_value::Value;
use aura_runtime_protocol::BridgeTransport;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Engine, Store};

/// One instantiated and isolated Aura Wasm component.
pub struct WasmPlugin {
    engine: Engine,
    store: Store<HostState>,
    bindings: AuraPluginV1,
    poisoned: bool,
}

impl WasmPlugin {
    /// Instantiates a component with constrained WASI and Aura Bridge imports.
    pub fn instantiate(
        engine: Engine,
        package_root: &Path,
        component: &Component,
        bridge: Arc<dyn BridgeTransport>,
        plugin_id: u64,
        session: u64,
    ) -> wasmtime::Result<Self> {
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        AuraPluginV1::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;
        let mut store = create_store(&engine, package_root, bridge, plugin_id, session)?;
        let bindings = AuraPluginV1::instantiate(&mut store, component, &linker)?;
        Ok(Self {
            engine,
            store,
            bindings,
            poisoned: false,
        })
    }

    fn call<T>(
        &mut self,
        operation: impl FnOnce(&AuraPluginV1, &mut Store<HostState>) -> wasmtime::Result<T>,
    ) -> wasmtime::Result<T> {
        if self.poisoned {
            return Err(wasmtime::Error::msg("runtime-failure: store is poisoned"));
        }
        self.store.set_fuel(FUEL_PER_CALL)?;
        self.store.set_epoch_deadline(1);
        self.store.epoch_deadline_trap();
        self.store.data_mut().callback_active = true;
        let (cancel, receiver) = mpsc::channel();
        let engine = self.engine.clone();
        let ticker = thread::spawn(move || {
            if receiver.recv_timeout(CALL_TIMEOUT).is_err() {
                engine.increment_epoch();
            }
        });
        let result = operation(&self.bindings, &mut self.store);
        self.store.data_mut().callback_active = false;
        let _ = cancel.send(());
        let _ = ticker.join();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    /// Calls the guest `load` lifecycle export.
    pub fn load(&mut self) -> wasmtime::Result<Result<(), String>> {
        self.call(|bindings, store| {
            bindings
                .aura_runtime_plugin()
                .call_load(store)
                .map(|result| result.map_err(format_guest_error))
        })
    }

    /// Calls the guest `enable` lifecycle export.
    pub fn enable(&mut self) -> wasmtime::Result<Result<(), String>> {
        self.call(|bindings, store| {
            bindings
                .aura_runtime_plugin()
                .call_enable(store)
                .map(|result| result.map_err(format_guest_error))
        })
    }

    /// Calls the guest operation export after validating Bridge Value v1 bytes.
    pub fn invoke(
        &mut self,
        operation: &str,
        input: &[u8],
        callback_id: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, String>> {
        Value::from_wire(input).map_err(|_| wasmtime::Error::msg("invalid-value"))?;
        let result = self.call(|bindings, store| {
            bindings
                .aura_runtime_plugin()
                .call_invoke(store, operation, input, callback_id)
                .map(|result| result.map_err(format_guest_error))
        })?;
        if let Ok(output) = &result {
            Value::from_wire(output).map_err(|_| wasmtime::Error::msg("invalid-value"))?;
        }
        Ok(result)
    }

    /// Calls the guest `disable` lifecycle export.
    pub fn disable(&mut self) -> wasmtime::Result<Result<(), String>> {
        self.call(|bindings, store| {
            bindings
                .aura_runtime_plugin()
                .call_disable(store)
                .map(|result| result.map_err(format_guest_error))
        })
    }

    /// Calls the guest `unload` lifecycle export.
    pub fn unload(&mut self) -> wasmtime::Result<Result<(), String>> {
        self.call(|bindings, store| {
            bindings
                .aura_runtime_plugin()
                .call_unload(store)
                .map(|result| result.map_err(format_guest_error))
        })
    }

    /// Returns bounded guest stdout and stderr captured so far.
    pub fn diagnostics(&self) -> (Vec<u8>, Vec<u8>) {
        let diagnostics = &self.store.data().diagnostics;
        (diagnostics.stdout(), diagnostics.stderr())
    }
}

fn format_guest_error(
    error: crate::bindings::exports::aura::runtime::plugin::PluginError,
) -> String {
    format!("{}: {}", error.code, error.message)
}
