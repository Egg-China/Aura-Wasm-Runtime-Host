//! Aura Bridge imports exposed to WebAssembly components.

use crate::bindings::aura::runtime::bridge::Host;
use crate::store::HostState;
use aura_bridge_value::Value;

impl Host for HostState {
    fn invoke(
        &mut self,
        operation: String,
        input: Vec<u8>,
    ) -> wasmtime::Result<Result<Vec<u8>, String>> {
        if !self.callback_active {
            return Ok(Err("bridge-denied".to_owned()));
        }
        Value::from_wire(&input).map_err(|_| wasmtime::Error::msg("invalid-value"))?;
        let output = match self
            .bridge
            .invoke(self.plugin_id, self.session, &operation, &input)
        {
            Ok(output) => output,
            Err(error) => return Ok(Err(error.stable_code().to_owned())),
        };
        Value::from_wire(&output).map_err(|_| wasmtime::Error::msg("invalid-value"))?;
        Ok(Ok(output))
    }

    fn retain_handle(
        &mut self,
        object_id: u64,
        generation: u64,
    ) -> wasmtime::Result<Result<(), String>> {
        if !self.callback_active {
            return Ok(Err("bridge-denied".to_owned()));
        }
        Ok(self
            .bridge
            .retain_handle(self.session, object_id, generation)
            .map_err(|error| error.stable_code().to_owned()))
    }

    fn release_handle(
        &mut self,
        object_id: u64,
        generation: u64,
    ) -> wasmtime::Result<Result<(), String>> {
        if !self.callback_active {
            return Ok(Err("bridge-denied".to_owned()));
        }
        Ok(self
            .bridge
            .release_handle(self.session, object_id, generation)
            .map_err(|error| error.stable_code().to_owned()))
    }
}
