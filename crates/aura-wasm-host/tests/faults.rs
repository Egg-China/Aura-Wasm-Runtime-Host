use aura_runtime_protocol::{BridgeTransport, Message, MessageBody, read_frame, write_frame};
use aura_wasm_host::{GuestEngine, HostError, HostResult, PayloadDescriptor, ProcessServer};
use std::io::{Cursor, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[test]
fn fatal_engine_failures_emit_no_recoverable_response() {
    for code in [
        "deadline-exceeded",
        "fuel-exhausted",
        "resource-limit",
        "runtime-failure",
    ] {
        let package = package();
        let output = SharedOutput::default();
        let error = ProcessServer::new(
            Cursor::new(framed(&[
                message(1, MessageBody::Hello),
                load(3, package.path()),
                message(5, MessageBody::Enable),
            ])),
            output.clone(),
            FailingEngine { code },
        )
        .serve()
        .expect_err("fatal engine error must terminate the protocol");
        assert!(error.to_string().contains(code));
        assert_eq!(output.messages().len(), 2);
    }
}

#[test]
fn eof_disables_and_unloads_an_enabled_payload() {
    let package = package();
    let calls = Arc::new(Mutex::new(Vec::new()));
    ProcessServer::new(
        Cursor::new(framed(&[
            message(1, MessageBody::Hello),
            load(3, package.path()),
            message(5, MessageBody::Enable),
        ])),
        SharedOutput::default(),
        RecordingEngine {
            calls: Arc::clone(&calls),
        },
    )
    .serve()
    .expect("clean EOF");
    assert_eq!(
        *calls.lock().expect("lock calls"),
        ["load", "enable", "disable", "unload"]
    );
}

struct FailingEngine {
    code: &'static str,
}

impl GuestEngine for FailingEngine {
    fn load(
        &mut self,
        _: &Path,
        _: &PayloadDescriptor,
        _: u64,
        _: u64,
        _: Arc<dyn BridgeTransport>,
    ) -> HostResult<()> {
        Ok(())
    }
    fn enable(&mut self) -> HostResult<()> {
        Err(HostError::new(self.code, self.code))
    }
    fn invoke(&mut self, _: &str, _: &[u8], _: u64) -> HostResult<Vec<u8>> {
        unreachable!()
    }
    fn disable(&mut self) -> HostResult<()> {
        Ok(())
    }
    fn unload(&mut self) -> HostResult<()> {
        Ok(())
    }
}

struct RecordingEngine {
    calls: Arc<Mutex<Vec<&'static str>>>,
}

impl GuestEngine for RecordingEngine {
    fn load(
        &mut self,
        _: &Path,
        _: &PayloadDescriptor,
        _: u64,
        _: u64,
        _: Arc<dyn BridgeTransport>,
    ) -> HostResult<()> {
        self.calls.lock().expect("lock calls").push("load");
        Ok(())
    }
    fn enable(&mut self) -> HostResult<()> {
        self.calls.lock().expect("lock calls").push("enable");
        Ok(())
    }
    fn invoke(&mut self, _: &str, input: &[u8], _: u64) -> HostResult<Vec<u8>> {
        Ok(input.to_vec())
    }
    fn disable(&mut self) -> HostResult<()> {
        self.calls.lock().expect("lock calls").push("disable");
        Ok(())
    }
    fn unload(&mut self) -> HostResult<()> {
        self.calls.lock().expect("lock calls").push("unload");
        Ok(())
    }
}

#[derive(Clone, Default)]
struct SharedOutput(Arc<Mutex<Vec<u8>>>);

impl SharedOutput {
    fn messages(&self) -> Vec<Message> {
        let bytes = self.0.lock().expect("lock output").clone();
        let mut input = bytes.as_slice();
        let mut messages = Vec::new();
        while let Some(message) = read_frame(&mut input).expect("decode response") {
            messages.push(message);
        }
        messages
    }
}

impl Write for SharedOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("lock output").extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn package() -> tempfile::TempDir {
    let package = tempfile::tempdir().expect("temporary package");
    std::fs::write(
        package.path().join("aura-wasm.json"),
        r#"{"schemaVersion":1,"component":"plugin.wasm"}"#,
    )
    .expect("write descriptor");
    std::fs::write(
        package.path().join("plugin.wasm"),
        wat::parse_str("(component)").expect("component WAT"),
    )
    .expect("write component");
    package
}

fn load(request_id: u64, package_root: &Path) -> Message {
    message(
        request_id,
        MessageBody::Load {
            package_root: package_root.to_string_lossy().into_owned(),
            entrypoint: "aura-wasm.json".to_owned(),
            plugin_id: 11,
            session: 13,
        },
    )
}

fn message(request_id: u64, body: MessageBody) -> Message {
    Message::new(request_id, body).expect("valid message")
}

fn framed(messages: &[Message]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for message in messages {
        write_frame(&mut bytes, message).expect("encode frame");
    }
    bytes
}
