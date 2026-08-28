use aura_wasm_engine::{create_engine, load_component};
use std::fs;

#[test]
fn frozen_wit_package_parses() {
    let mut resolve = wit_parser::Resolve::default();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sdk/wit");
    let (package, _) = resolve.push_dir(&path).expect("WIT package must parse");
    assert_eq!(resolve.packages[package].name.namespace, "aura");
    assert_eq!(resolve.packages[package].name.name, "runtime");
}

#[test]
fn accepts_components_and_rejects_core_modules() {
    let root = tempfile::tempdir().expect("temporary directory");
    let component = root.path().join("component.wasm");
    let module = root.path().join("module.wasm");
    fs::write(
        &component,
        wat::parse_str("(component)").expect("component WAT"),
    )
    .expect("write component");
    fs::write(&module, wat::parse_str("(module)").expect("module WAT")).expect("write module");

    let engine = create_engine().expect("engine");
    load_component(&engine, &component).expect("Component Model binary must load");
    assert!(load_component(&engine, &module).is_err());
}
