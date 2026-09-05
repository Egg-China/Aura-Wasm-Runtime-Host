package dev.hmclce.runtime.wasm;

import org.jackhuang.hmcl.plugin.PluginArtifactIdentity;
import org.jackhuang.hmcl.plugin.PluginDataObject;
import org.jackhuang.hmcl.plugin.PluginDataValue;
import org.jackhuang.hmcl.plugin.PluginHookEvent;
import org.jackhuang.hmcl.plugin.PluginHookPoint;
import org.jackhuang.hmcl.plugin.PluginHookResult;
import org.jackhuang.hmcl.plugin.PluginPatchDeclaration;
import org.jackhuang.hmcl.plugin.PluginPatchInvocation;
import org.jackhuang.hmcl.plugin.PluginPatchResult;
import org.jackhuang.hmcl.plugin.PluginSecretAccess;
import org.jackhuang.hmcl.plugin.bridge.PluginCapabilityToken;
import org.jackhuang.hmcl.plugin.bridge.PluginPermissionAuthority;
import org.jackhuang.hmcl.plugin.runtime.PluginExecutionMode;
import org.jackhuang.hmcl.plugin.runtime.RuntimeFeature;
import org.jackhuang.hmcl.plugin.runtime.RuntimePatchWireCodec;
import org.jackhuang.hmcl.plugin.runtime.RuntimePayloadContext;
import org.jackhuang.hmcl.plugin.runtime.RuntimePayloadHandle;
import org.jackhuang.hmcl.plugin.runtime.RuntimeProvider;
import org.jackhuang.hmcl.plugin.runtime.RuntimeProviderDeclaration;
import org.jetbrains.annotations.NotNullByDefault;
import org.jetbrains.annotations.Nullable;
import org.jetbrains.annotations.Unmodifiable;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.time.Instant;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.concurrent.TimeUnit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/// Verifies canonical Hook and Patch contracts against the real Wasm process Host and component.
@NotNullByDefault
final class WasmRuntimeProcessIntegrationTest {
    /// Isolated data directory owned by the test payload.
    @TempDir
    Path temporaryDirectory;

    /// Runs a replacing Hook and an after Patch against one real isolated Wasm payload and closes its child process.
    ///
    /// The 60-second test-level bound covers the full lifecycle; each process-session transition and Hook dispatch
    /// remains separately deadline-bounded.
    ///
    /// @throws Exception if process startup, lifecycle, Hook, or Patch validation fails
    @Test
    @Timeout(value = 60L, unit = TimeUnit.SECONDS)
    void invokesCanonicalHookAndPatchAcrossRealWasmProcess() throws Exception {
        WasmRuntimeProvider provider = productionProvider(processHost());
        @Nullable RuntimePayloadHandle handle = null;
        boolean enabled = false;
        try {
            assertTrue(provider.healthCheck());
            handle = provider.loadPayload(payloadContext());
            provider.enablePayload(handle);
            enabled = true;

            RuntimeProvider.HookInvoker hookInvoker = assertInstanceOf(RuntimeProvider.HookInvoker.class, provider);
            PluginHookResult hookResult = Objects.requireNonNull(
                    hookInvoker.invokeHook(
                            handle,
                            capabilityToken(),
                            hookEvent(),
                            Duration.ofSeconds(2L)
                    ),
                    "Wasm process Hook payload returned a malformed result"
            );
            assertEquals(PluginHookResult.Action.REPLACE, hookResult.action());
            PluginDataObject command = Objects.requireNonNull(hookResult.data(), "missing replacement data")
                    .requireObject("plan")
                    .requireObject("command");
            assertEquals(
                    List.of(
                            new PluginDataValue.StringValue("-Daura.example.existing=true"),
                            new PluginDataValue.StringValue("-Daura.example.wasm-hook=true")
                    ),
                    command.requireArray("jvmArguments")
            );

            assertUnchangedAfterFileNamePatch(provider, handle);

            provider.disablePayload(handle);
            enabled = false;
            provider.unloadPayload(handle);
            handle = null;
        } finally {
            closePayload(provider, handle, enabled);
        }
    }

    /// Invokes the canonical Patch operation and proves the invocation-local handle table accepts one response.
    ///
    /// @param provider real Wasm process Provider
    /// @param handle enabled real payload handle
    /// @throws IOException if the process or canonical Patch exchange fails
    private void assertUnchangedAfterFileNamePatch(
            WasmRuntimeProvider provider,
            RuntimePayloadHandle handle
    ) throws IOException {
        PluginPatchDeclaration declaration = new PluginPatchDeclaration(
                "org.jackhuang.hmcl.util.io.FileUtils",
                "getName",
                PluginPatchDeclaration.PatchType.AFTER,
                List.of("java.nio.file.Path")
        );
        PluginPatchInvocation invocation = PluginPatchInvocation.after(
                declaration,
                null,
                List.of(temporaryDirectory.resolve("profile.json")),
                "profile.json"
        );
        try (RuntimePatchWireCodec codec = new RuntimePatchWireCodec()) {
            byte @Unmodifiable [] response = provider.invokePayload(
                    handle,
                    "aura.patch.v1",
                    codec.encodeInvocation(invocation),
                    0L
            );

            assertEquals(
                    PluginPatchResult.Action.UNCHANGED,
                    codec.decodeResult(response, invocation).action()
            );
            assertThrows(IOException.class, () -> codec.decodeResult(response, invocation));
        }
    }

    /// Closes a partially completed lifecycle without allowing a failed assertion to retain a child process.
    ///
    /// @param provider real Provider to close
    /// @param handle loaded payload handle, or `null` after successful unload
    /// @param enabled whether the payload is still enabled
    /// @throws IOException if disablement or shutdown fails during cleanup
    private static void closePayload(
            WasmRuntimeProvider provider,
            @Nullable RuntimePayloadHandle handle,
            boolean enabled
    ) throws IOException {
        try {
            if (handle != null) {
                if (enabled) {
                    provider.disablePayload(handle);
                }
                provider.unloadPayload(handle);
            }
        } finally {
            provider.close();
        }
    }

    /// Creates the production Provider boundary whose sessions start the real Aura process supervisor.
    ///
    /// @param executable exact Wasm process Host executable
    /// @return production-backed Wasm Provider
    private static WasmRuntimeProvider productionProvider(Path executable) {
        return new WasmRuntimeProvider(
                "dev.hmclce.runtime.wasm-host",
                "0.1.0-beta.1",
                List.of(new RuntimeProviderDeclaration(
                        "wasm",
                        Set.of(1),
                        1,
                        Set.of(PluginExecutionMode.ISOLATED),
                        Set.of(RuntimeFeature.BRIDGE, RuntimeFeature.HOOKS, RuntimeFeature.PATCHES,
                                RuntimeFeature.NATIVE)
                )),
                executable,
                WasmRuntimeProvider.AuraSession::start
        );
    }

    /// Creates an isolated temporary package from the checked-in metadata and compiled Wasm component.
    ///
    /// @return context whose Java capability supplier must never cross the process boundary
    /// @throws IOException if the required source metadata or component is missing
    private RuntimePayloadContext payloadContext() throws IOException {
        Path sampleRoot = Path.of(requireSystemProperty("aura.wasm.launchHookSample")).toRealPath();
        Path component = component();
        Path packageRoot = Files.createDirectories(temporaryDirectory.resolve("package"));
        Files.copy(sampleRoot.resolve("plugin.json"), packageRoot.resolve("plugin.json"));
        Files.copy(sampleRoot.resolve("aura-wasm.json"), packageRoot.resolve("aura-wasm.json"));
        Files.copy(component, packageRoot.resolve("plugin.wasm"));
        return new RuntimePayloadContext(
                new PluginArtifactIdentity(
                        "dev.hmclce.example.wasm.launch-hook",
                        "1.0.0",
                        "a".repeat(64)
                ),
                packageRoot,
                "aura-wasm.json",
                PluginExecutionMode.ISOLATED,
                temporaryDirectory.resolve("data"),
                () -> {
                    throw new AssertionError("Wasm process test must not resolve a JVM capability token");
                }
        );
    }

    /// Creates a live Hook token which remains Java-only at the Provider boundary.
    ///
    /// @return launcher-issued opaque Hook capability token
    private static PluginCapabilityToken capabilityToken() {
        PluginArtifactIdentity identity = new PluginArtifactIdentity(
                "dev.hmclce.example.wasm.launch-hook",
                "1.0.0",
                "a".repeat(64)
        );
        return new PluginPermissionAuthority().issue(
                identity,
                PluginExecutionMode.ISOLATED,
                Set.of(),
                "runtime.payload",
                Duration.ofMinutes(1L)
        );
    }

    /// Creates a structured launch-plan Hook event that the sample must replace without dropping existing fields.
    ///
    /// @return event whose response must append the sample JVM argument
    private static PluginHookEvent hookEvent() {
        PluginDataObject command = PluginDataObject.of(Map.of(
                "mode", new PluginDataValue.StringValue("structured-java"),
                "jvmArguments", new PluginDataValue.ArrayValue(
                        List.of(new PluginDataValue.StringValue("-Daura.example.existing=true"))
                )
        ));
        PluginDataObject plan = PluginDataObject.of(Map.of(
                "command", new PluginDataValue.ObjectValue(command)
        ));
        return new PluginHookEvent(
                1,
                "wasm-process-hook-42",
                PluginHookPoint.BEFORE_GAME_LAUNCH,
                Instant.parse("2026-09-05T00:00:00Z"),
                PluginDataObject.of(Map.of("plan", new PluginDataValue.ObjectValue(plan))),
                PluginSecretAccess.denied("dev.hmclce.example.wasm.launch-hook")
        );
    }

    /// Resolves the explicit Gradle-forwarded process executable and rejects missing test configuration.
    ///
    /// @return canonical real Wasm process Host executable
    /// @throws IOException if the supplied path is absent or is not a regular file
    private static Path processHost() throws IOException {
        Path executable = Path.of(requireSystemProperty("aura.wasm.processHost")).toAbsolutePath().normalize();
        if (!Files.isRegularFile(executable)) {
            throw new IOException("AURA_WASM_PROCESS_HOST does not name a regular file: " + executable);
        }
        return executable.toRealPath();
    }

    /// Resolves the explicit Gradle-forwarded component and rejects missing test configuration.
    ///
    /// @return canonical compiled Wasm component
    /// @throws IOException if the supplied path is absent or is not a regular file
    private static Path component() throws IOException {
        Path component = Path.of(requireSystemProperty("aura.wasm.component")).toAbsolutePath().normalize();
        if (!Files.isRegularFile(component)) {
            throw new IOException("AURA_WASM_COMPONENT does not name a regular file: " + component);
        }
        return component.toRealPath();
    }

    /// Returns one required Gradle-forwarded integration setting.
    ///
    /// @param name exact Java system property name
    /// @return non-blank property value
    private static String requireSystemProperty(String name) {
        @Nullable String value = System.getProperty(name);
        if (value == null || value.isBlank()) {
            throw new IllegalStateException("Set required Wasm process integration configuration: " + name);
        }
        return value;
    }
}
