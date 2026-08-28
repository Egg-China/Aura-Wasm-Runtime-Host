//! Wasmtime store state and resource limits.

use crate::config::{INSTANCES, MEMORIES, MEMORY_BYTES, TABLE_ELEMENTS, TABLES};
use crate::wasi::{GuestDiagnostics, build_wasi};
use aura_runtime_protocol::BridgeTransport;
use std::path::Path;
use std::sync::Arc;
use wasmtime::{Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxView, WasiView};

/// State isolated to one payload process and one Wasmtime store.
pub struct HostState {
    /// WASI 0.2 capability context.
    pub wasi: WasiCtx,
    /// WASI Component Model resources.
    pub table: ResourceTable,
    /// Wasmtime allocation limits.
    pub limits: StoreLimits,
    /// Bounded guest output retained for diagnostics.
    pub diagnostics: GuestDiagnostics,
    /// Aura Bridge callback transport.
    pub bridge: Arc<dyn BridgeTransport>,
    /// Aura plugin identifier used by Bridge callbacks.
    pub plugin_id: u64,
    /// Aura capability session identifier.
    pub session: u64,
    /// Whether a lifecycle call currently authorizes callback reentry.
    pub callback_active: bool,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

/// Creates one bounded store for an isolated payload.
pub fn create_store(
    engine: &wasmtime::Engine,
    package_root: &Path,
    bridge: Arc<dyn BridgeTransport>,
    plugin_id: u64,
    session: u64,
) -> wasmtime::Result<Store<HostState>> {
    let (wasi, table, diagnostics) = build_wasi(package_root)?;
    let limits = StoreLimitsBuilder::new()
        .memory_size(MEMORY_BYTES)
        .table_elements(TABLE_ELEMENTS)
        .instances(INSTANCES)
        .tables(TABLES)
        .memories(MEMORIES)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(
        engine,
        HostState {
            wasi,
            table,
            limits,
            diagnostics,
            bridge,
            plugin_id,
            session,
            callback_active: false,
        },
    );
    store.limiter(|state| &mut state.limits);
    Ok(store)
}
