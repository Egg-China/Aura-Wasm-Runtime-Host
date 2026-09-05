# Aura Wasm Guest SDK

The stable guest contract is `sdk/wit/aura-runtime.wit`, package `aura:runtime@0.1.0`, world
`aura-plugin-v1`. `sdk/rust/aura-wasm-guest` is an unpublished Rust helper crate for canonical
Bridge Value v1 encoding and validation.

Generate Component bindings from the WIT contract with `cargo component bindings`, implement the
generated `Guest` trait, and export the implementation with the generated `export!` macro. The
complete `examples/launch-hook` package demonstrates lifecycle validation and a real Aura Hook
transformation.

## Hook and Patch callbacks

[Current source](../README.md#source-support-and-published-downloads) adds the Java Hook SPI and
Patch dispatch without changing the WIT contract or process ABI. Use `AuraValue::Map(Vec<...>)`
to preserve field order, and encode version fields with `AuraValue::Integer(1)` (not a floating
point value). The [example](../examples/launch-hook/README.md) implements:

- `hook.before-game-launch`: returns `contractVersion`, `action`, and, for `replace`, the updated
  `data` and `protectedSecrets`. It preserves unknown launch-plan fields.
- `aura.patch.v1`: reads a schema-v1 request and returns the ordered `schemaVersion: 1`,
  `action: "unchanged"` response for its observational `after` Patch.

Patch targets and `launcher-patch` must be declared in the schema-v5 manifest; current permission
and payload generation are revalidated by Aura for every call. Reference values are invocation-local
handles, not process-global object IDs, and expire when the invocation ends. Do not retain them.
Tokens, ABI identifiers, and transport framing are not plugin-defined data. All binary output belongs
to the Host protocol; do not write logs to its stdout.

Run the [native and Java integration gates](../README.md#build-and-test-current-source) against the
pinned Aura JAR and a newly built component. The existing published beta assets are not upgraded by
editing or building this SDK; no new SDK or Host release is published by this source delivery.
