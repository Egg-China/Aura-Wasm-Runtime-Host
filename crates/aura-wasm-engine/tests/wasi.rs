use aura_wasm_engine::wasi::build_wasi;
use std::fs;

#[test]
fn guest_diagnostics_are_not_inherited() {
    let root = tempfile::tempdir().expect("temporary package root");
    fs::write(root.path().join("payload.txt"), "readable").expect("write fixture");
    let (_, _, diagnostics) = build_wasi(root.path()).expect("constrained WASI context");
    assert!(diagnostics.stdout().is_empty());
    assert!(diagnostics.stderr().is_empty());
}
