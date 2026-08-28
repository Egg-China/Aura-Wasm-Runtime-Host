# Aura Wasm Guest SDK

The stable guest contract is `sdk/wit/aura-runtime.wit`, package `aura:runtime@0.1.0`, world
`aura-plugin-v1`. `sdk/rust/aura-wasm-guest` is an unpublished Rust helper crate for canonical
Bridge Value v1 encoding and validation.

Generate Component bindings from the WIT contract with `cargo component bindings`, implement the
generated `Guest` trait, and export the implementation with the generated `export!` macro. The
complete `examples/launch-hook` package demonstrates lifecycle validation and a real Aura Hook
transformation.
