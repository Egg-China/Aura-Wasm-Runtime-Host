# Aura Wasm Launch Hook Example

This schema-v5 payload runs as a WebAssembly Component through
`dev.hmclce.runtime.wasm-host`. It handles `hook.before-game-launch`, preserves unknown launch-plan
fields, and appends `-Daura.example.wasm-hook=true` to structured Java JVM arguments.

Build and package it from the repository root:

```powershell
cargo component build --manifest-path examples/launch-hook/Cargo.toml --release
powershell -NoProfile -ExecutionPolicy Bypass -File tools/package-wasm-plugin.ps1 `
  -Source examples/launch-hook `
  -Component target/wasm32-wasip1/release/launch_hook.wasm `
  -Output artifacts/dev.hmclce.example.wasm.launch-hook-v1.0.0.npl
```

The JVM capability token remains in Aura. The component sees only the canonical Hook envelope and
the WIT Bridge imports authorized by its original payload context.
