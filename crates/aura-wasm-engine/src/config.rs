//! Fixed first-beta execution limits.

use std::time::Duration;

/// Maximum linear memory available to a payload.
pub const MEMORY_BYTES: usize = 256 * 1024 * 1024;
/// Fresh fuel budget assigned to every lifecycle call.
pub const FUEL_PER_CALL: u64 = 50_000_000;
/// Maximum native duration of one lifecycle call.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(10);
/// Maximum number of table elements in one payload store.
pub const TABLE_ELEMENTS: usize = 100_000;
/// Maximum number of core instances in one payload store.
pub const INSTANCES: usize = 100;
/// Maximum number of core tables in one payload store.
pub const TABLES: usize = 100;
/// Maximum number of linear memories in one payload store.
pub const MEMORIES: usize = 1;
/// Maximum bytes retained independently from guest stdout and stderr.
pub const DIAGNOSTIC_BYTES: usize = 64 * 1024;
