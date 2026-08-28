use aura_bridge_value::Value;
use aura_runtime_protocol::{BridgeError, BridgeTransport};
use aura_wasm_engine::plugin::WasmPlugin;
use aura_wasm_engine::{create_engine, load_component};
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn launch_hook_example_replaces_the_structured_launch_plan() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let package_root = repository.join("examples/launch-hook");
    let component_path = repository.join("target/wasm32-wasip1/release/launch_hook.wasm");
    let engine = create_engine().expect("create engine");
    let component = load_component(&engine, &component_path).expect("load launch-hook component");
    let mut plugin = WasmPlugin::instantiate(
        engine,
        &package_root,
        &component,
        Arc::new(NoBridge),
        59,
        61,
    )
    .expect("instantiate launch-hook component");

    let original_data = Value::Map(vec![(
        "plan".to_owned(),
        Value::Map(vec![(
            "command".to_owned(),
            Value::Map(vec![
                (
                    "mode".to_owned(),
                    Value::String("structured-java".to_owned()),
                ),
                (
                    "jvmArguments".to_owned(),
                    Value::Array(vec![Value::String("-Xmx2G".to_owned())]),
                ),
                ("futureField".to_owned(), Value::Bool(true)),
            ]),
        )]),
    )]);
    let input = Value::Map(vec![
        ("contractVersion".to_owned(), Value::Integer(1)),
        (
            "dispatchId".to_owned(),
            Value::String("dispatch-1".to_owned()),
        ),
        (
            "point".to_owned(),
            Value::String("before-game-launch".to_owned()),
        ),
        (
            "occurredAt".to_owned(),
            Value::String("2026-08-29T00:00:00Z".to_owned()),
        ),
        ("data".to_owned(), original_data),
    ]);

    plugin.load().expect("call load").expect("guest load");
    plugin.enable().expect("call enable").expect("guest enable");
    let output = plugin
        .invoke(
            "hook.before-game-launch",
            &input.to_wire().expect("encode event"),
            0,
        )
        .expect("call launch Hook")
        .expect("guest launch Hook");
    let decoded = Value::from_wire(&output).expect("decode Hook result");
    assert_eq!(
        decoded,
        Value::Map(vec![
            ("contractVersion".to_owned(), Value::Integer(1)),
            ("action".to_owned(), Value::String("replace".to_owned())),
            (
                "data".to_owned(),
                Value::Map(vec![(
                    "plan".to_owned(),
                    Value::Map(vec![(
                        "command".to_owned(),
                        Value::Map(vec![
                            (
                                "mode".to_owned(),
                                Value::String("structured-java".to_owned())
                            ),
                            (
                                "jvmArguments".to_owned(),
                                Value::Array(vec![
                                    Value::String("-Xmx2G".to_owned()),
                                    Value::String("-Daura.example.wasm-hook=true".to_owned()),
                                ]),
                            ),
                            ("futureField".to_owned(), Value::Bool(true)),
                        ]),
                    )]),
                )]),
            ),
            ("protectedSecrets".to_owned(), Value::Map(Vec::new())),
        ])
    );
    plugin
        .disable()
        .expect("call disable")
        .expect("guest disable");
    plugin.unload().expect("call unload").expect("guest unload");
}

struct NoBridge;

impl BridgeTransport for NoBridge {
    fn invoke(
        &self,
        _plugin_id: u64,
        _session: u64,
        _operation: &str,
        _input: &[u8],
    ) -> Result<Vec<u8>, BridgeError> {
        panic!("example must not invoke Bridge")
    }

    fn retain_handle(
        &self,
        _session: u64,
        _object_id: u64,
        _generation: u64,
    ) -> Result<(), BridgeError> {
        panic!("example must not retain handles")
    }

    fn release_handle(
        &self,
        _session: u64,
        _object_id: u64,
        _generation: u64,
    ) -> Result<(), BridgeError> {
        panic!("example must not release handles")
    }
}
