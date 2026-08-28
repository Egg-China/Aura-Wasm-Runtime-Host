use aura_wasm_host::descriptor::PayloadDescriptor;
use std::fs;

fn write_descriptor(root: &std::path::Path, json: &str, wasm: &str) {
    fs::write(root.join("aura-wasm.json"), json).expect("write descriptor");
    fs::write(
        root.join("plugin.wasm"),
        wat::parse_str(wasm).expect("parse WAT"),
    )
    .expect("write WebAssembly");
}

#[test]
fn accepts_only_exact_component_descriptor() {
    let root = tempfile::tempdir().expect("temporary directory");
    write_descriptor(
        root.path(),
        r#"{"schemaVersion":1,"component":"plugin.wasm"}"#,
        "(component)",
    );
    let descriptor =
        PayloadDescriptor::read(root.path(), "aura-wasm.json").expect("valid descriptor");
    assert_eq!(
        descriptor.component(),
        root.path()
            .join("plugin.wasm")
            .canonicalize()
            .expect("canonical component"),
    );

    fs::write(
        root.path().join("aura-wasm.json"),
        r#"{"schemaVersion":1,"component":"plugin.wasm","extra":true}"#,
    )
    .expect("write unknown-field descriptor");
    let error = PayloadDescriptor::read(root.path(), "aura-wasm.json").expect_err("unknown field");
    assert_eq!(error.code(), "invalid-descriptor");
}

#[test]
fn rejects_escape_and_core_module() {
    let root = tempfile::tempdir().expect("temporary directory");
    write_descriptor(
        root.path(),
        r#"{"schemaVersion":1,"component":"../outside.wasm"}"#,
        "(component)",
    );
    let error = PayloadDescriptor::read(root.path(), "aura-wasm.json").expect_err("path escape");
    assert_eq!(error.code(), "path-escape");

    write_descriptor(
        root.path(),
        r#"{"schemaVersion":1,"component":"plugin.wasm"}"#,
        "(module)",
    );
    let error = PayloadDescriptor::read(root.path(), "aura-wasm.json").expect_err("core module");
    assert_eq!(error.code(), "invalid-component");
}
