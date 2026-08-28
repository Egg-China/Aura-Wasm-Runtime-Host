use aura_wasm_engine::config::{CALL_TIMEOUT, FUEL_PER_CALL, MEMORIES, MEMORY_BYTES};

#[test]
fn first_beta_limits_are_fixed() {
    assert_eq!(MEMORY_BYTES, 256 * 1024 * 1024);
    assert_eq!(MEMORIES, 1);
    assert_eq!(FUEL_PER_CALL, 50_000_000);
    assert_eq!(CALL_TIMEOUT.as_secs(), 10);
}
