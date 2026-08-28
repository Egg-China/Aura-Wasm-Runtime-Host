//! Strict payload descriptor parsing.

use crate::error::{HostError, HostResult};
use crate::path_policy::resolve_resource;
use aura_wasm_engine::{create_engine, load_component};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_DESCRIPTOR_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDescriptor {
    #[serde(rename = "schemaVersion")]
    schema_version: i64,
    component: String,
}

/// Validated Aura Wasm payload descriptor.
#[derive(Debug)]
pub struct PayloadDescriptor {
    component: PathBuf,
}

impl PayloadDescriptor {
    /// Reads a descriptor and validates its referenced Component Model binary.
    pub fn read(package_root: &Path, entrypoint: &str) -> HostResult<Self> {
        let descriptor_path = resolve_resource(package_root, entrypoint, "json")?;
        let metadata = fs::metadata(&descriptor_path)
            .map_err(|error| HostError::new("invalid-descriptor", error.to_string()))?;
        if metadata.len() > MAX_DESCRIPTOR_BYTES {
            return Err(HostError::new(
                "invalid-descriptor",
                "descriptor exceeds one MiB",
            ));
        }
        let bytes = fs::read(&descriptor_path)
            .map_err(|error| HostError::new("invalid-descriptor", error.to_string()))?;
        let raw: RawDescriptor = serde_json::from_slice(&bytes)
            .map_err(|error| HostError::new("invalid-descriptor", error.to_string()))?;
        if raw.schema_version != 1 {
            return Err(HostError::new(
                "invalid-descriptor",
                "schemaVersion must be 1",
            ));
        }
        let component = resolve_resource(package_root, &raw.component, "wasm")?;
        let engine = create_engine()
            .map_err(|error| HostError::new("runtime-failure", error.to_string()))?;
        load_component(&engine, &component)
            .map_err(|error| HostError::new("invalid-component", error.to_string()))?;
        Ok(Self { component })
    }

    /// Returns the canonical component path.
    pub fn component(&self) -> &Path {
        &self.component
    }
}
