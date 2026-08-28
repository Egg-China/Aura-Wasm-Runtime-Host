use aura_wasm_host::{ProcessServer, WasmGuestEngine};
use std::io;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.as_slice() != ["--stdio"] {
        eprintln!("usage: aura-wasm-host --stdio");
        std::process::exit(2);
    }

    if let Err(error) =
        ProcessServer::new(io::stdin(), io::stdout(), WasmGuestEngine::default()).serve()
    {
        eprintln!("Wasm Host protocol failure: {error}");
        std::process::exit(1);
    }
}
