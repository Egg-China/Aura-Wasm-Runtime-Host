# Aura Wasm Launch Hook Example

This schema-v5 payload runs as a WebAssembly Component through
`dev.hmclce.runtime.wasm-host`. It handles `hook.before-game-launch`, preserves unknown launch-plan
fields, and appends `-Daura.example.wasm-hook=true` to structured Java JVM arguments.

Current source also handles `aura.patch.v1`. Its manifest declares an `after` Patch on
`org.jackhuang.hmcl.util.io.FileUtils.getName(java.nio.file.Path)` and requires `launcher-patch`
in both permission lists. The handler observes without modifying arguments or results and returns
the ordered Bridge Value map `schemaVersion: 1`, `action: "unchanged"`. This is a source example;
the existing beta downloads have not been republished with these changes.

Build and package it from the repository root:

```powershell
cargo component build --manifest-path examples/launch-hook/Cargo.toml --release --locked
powershell -NoProfile -ExecutionPolicy Bypass -File tools/package-wasm-plugin.ps1 `
  -Source examples/launch-hook `
  -Component target/wasm32-wasip1/release/launch_hook.wasm `
  -Output artifacts/dev.hmclce.example.wasm.launch-hook-v1.0.0.npl
```

The JVM capability token remains in Aura. The component sees canonical Hook/Patch envelopes and
the WIT Bridge imports authorized by its original payload context. It does not retain or reuse
invocation-local Patch handles. Grant `launcher-patch` only to reviewed exact artifacts; Aura can
revoke a callback when the payload is disabled, unloaded, replaced, or loses permission. Callback
errors and deadlines use Aura's Patch failure policy rather than replacing the target with a bad result.

The Hook returns `replace` only for `structured-java` commands and preserves unknown fields;
other command modes return `unchanged`. To run actual native and Java process tests, follow the
[root build instructions](../../README.md#build-and-test-current-source), including both required
`AURA_WASM_PROCESS_HOST` and `AURA_WASM_COMPONENT` paths.
