//! Constrained WASI 0.2 context construction.

use crate::config::DIAGNOSTIC_BYTES;
use std::path::Path;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{FsPerms, ResourceTable, WasiCtx, WasiCtxBuilder};

/// Captured bounded guest diagnostics.
#[derive(Clone, Debug)]
pub struct GuestDiagnostics {
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
}

impl GuestDiagnostics {
    /// Returns the bytes written to guest stdout.
    pub fn stdout(&self) -> Vec<u8> {
        self.stdout.contents().to_vec()
    }

    /// Returns the bytes written to guest stderr.
    pub fn stderr(&self) -> Vec<u8> {
        self.stderr.contents().to_vec()
    }
}

/// Builds a WASI context with a read-only `/plugin` preopen and no inherited state.
pub fn build_wasi(
    package_root: &Path,
) -> wasmtime::Result<(WasiCtx, ResourceTable, GuestDiagnostics)> {
    let stdout = MemoryOutputPipe::new(DIAGNOSTIC_BYTES);
    let stderr = MemoryOutputPipe::new(DIAGNOSTIC_BYTES);
    let diagnostics = GuestDiagnostics {
        stdout: stdout.clone(),
        stderr: stderr.clone(),
    };
    let mut builder = WasiCtxBuilder::new();
    builder
        .stdout(stdout)
        .stderr(stderr)
        .allow_blocking_current_thread(true)
        .preopened_dir(package_root, "/plugin", FsPerms::ReadOnly)?;
    Ok((builder.build(), ResourceTable::new(), diagnostics))
}
