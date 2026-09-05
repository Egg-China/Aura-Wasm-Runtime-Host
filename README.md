# Aura Wasm Runtime Host

Aura Wasm Runtime Host is an optional schema-v5 Runtime Provider for Aura Launcher. It runs
WebAssembly Component Model plugins in isolated Wasmtime processes and connects them to Aura
through Bridge ABI 1.

The first supported release line targets Aura `>=27.1-0-next` on Windows, Linux, and macOS for x64
and arm64. The repository is licensed under GPL-3.0-or-later.

## Source support and published downloads

Current source implements the canonical Java Hook SPI and `aura.patch.v1` callbacks. The Host
advertises `bridge/hooks/patches/native`; the [launch-hook example](examples/launch-hook/README.md)
demonstrates both a structured launch-plan transformation and an observational `after` Patch.
The existing `0.1.0-beta.1` Release assets and Store entry are unchanged: downloading that older
beta does not provide these new source changes. This work does not publish a new release.

Provider ABI 1, Bridge ABI 1, process protocol v1, and schema-v5 manifests remain compatible.
Hooks use the dispatcher's exact timeout and callback ID `0`; Patch invocations also use callback
ID `0`. Java capability tokens remain in the JVM. Invocation-local Patch handles must not be
retained or reused after a callback returns. Aura checks the exact payload lifecycle and current
`launcher-patch` grant for each Patch callback; a declaration does not grant permission by itself.

## Build and test current source

Use the pinned Rust toolchain, `wasm32-wasip1`, `cargo-component` 0.21.1, JDK 17, Gradle 9.6.1,
Node.js/npm, and a native linker for your platform (MSVC on Windows). The commands below use
PowerShell 7 from the repository root. GitHub CLI needs read access to the Aura CI artifact.

The exact build dependency is Aura commit `636b06aad03c5d21946369c836280c891c13054d`, successful
Java CI run `33931508945`, file `Aura-Launcher-27.1.dev-636b06a-next.jar` (16,265,195 bytes).
Its SHA-256 is `674f717f5f97a5b7e8f7f20e4d60aa2e25451d71a96ab475f4595d0482f99d4b`.

```powershell
gh run download 33931508945 --repo Egg-China/Aura-Launcher `
  --name Aura-Launcher-636b06aad03c5d21946369c836280c891c13054d --dir .ci/aura
$env:AURA_JAR = (Resolve-Path '.ci/aura/Aura-Launcher-27.1.dev-636b06a-next.jar').Path
if ((Get-FileHash $env:AURA_JAR -Algorithm SHA256).Hash.ToLowerInvariant() -cne `
  '674f717f5f97a5b7e8f7f20e4d60aa2e25451d71a96ab475f4595d0482f99d4b') {
  throw 'Unexpected Aura build dependency'
}
npm --prefix sdk ci --ignore-scripts
cargo component build --manifest-path examples/launch-hook/Cargo.toml --release --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --manifest-path sdk/rust/aura-wasm-guest/Cargo.toml --locked
cargo build --release --locked -p aura-wasm-host
$hostName = if ($IsWindows) { 'aura-wasm-host.exe' } else { 'aura-wasm-host' }
$env:AURA_WASM_PROCESS_HOST = (Resolve-Path (Join-Path 'target/release' $hostName)).Path
$env:AURA_WASM_COMPONENT = (Resolve-Path 'target/wasm32-wasip1/release/launch_hook.wasm').Path
gradle -p host-plugin test jar --rerun-tasks --no-daemon
& ./tools/test-ci-workflows.ps1
```

Java integration tests require both the native Host and the compiled example component: missing
inputs fail instead of skipping tests. They exercise actual Java-to-process Hook/Patch lifecycles,
including the original structured Hook's `replace` response. Native stdio tests additionally check
literal wire bytes, request IDs, clean shutdown, extra stdout, and timeout cleanup. Every native CI
platform runs the integration and validates its generated NPL with deliberate invalid-package
mutations. See [the guest SDK](sdk/README.md) for authoring and [the example](examples/launch-hook/README.md)
for component packaging.
