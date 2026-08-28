package dev.hmclce.runtime.wasm;

import org.jackhuang.hmcl.plugin.runtime.PluginPlatformTarget;
import org.jetbrains.annotations.NotNullByDefault;
import org.junit.jupiter.api.Test;

import java.nio.file.Path;

import static org.junit.jupiter.api.Assertions.assertEquals;

/// Verifies deterministic platform-specific process Host selection.
@NotNullByDefault
final class WasmRuntimeHostPluginTest {
    /// Selects the exact Windows executable path.
    @Test
    void resolvesWindowsExecutable() {
        assertEquals(
                Path.of("root", "native", "windows-x64", "aura-wasm-host.exe"),
                WasmRuntimeHostPlugin.resolveExecutable(
                        Path.of("root"), PluginPlatformTarget.parse("windows-x64")));
    }

    /// Selects the exact Unix executable path.
    @Test
    void resolvesLinuxExecutable() {
        assertEquals(
                Path.of("root", "native", "linux-arm64", "aura-wasm-host"),
                WasmRuntimeHostPlugin.resolveExecutable(
                        Path.of("root"), PluginPlatformTarget.parse("linux-arm64")));
    }
}
