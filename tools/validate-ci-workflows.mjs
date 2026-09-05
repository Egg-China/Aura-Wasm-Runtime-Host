import fs from "node:fs";
import { execFileSync } from "node:child_process";
import YAML from "../sdk/node_modules/yaml/dist/index.js";

const expectedPlatforms = [
  "windows-x64",
  "windows-arm64",
  "linux-x64",
  "linux-arm64",
  "macos-x64",
  "macos-arm64",
];
const expectedProvenance = {
  AURA_COMMIT: "636b06aad03c5d21946369c836280c891c13054d",
  AURA_RUN_ID: "33931508945",
  AURA_JAR_SHA256: "674f717f5f97a5b7e8f7f20e4d60aa2e25451d71a96ab475f4595d0482f99d4b",
};

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function readWorkflow(file) {
  if (!fs.existsSync(file)) throw new Error(`workflow is missing: ${file}`);
  return YAML.parse(fs.readFileSync(file, "utf8"));
}

function triggerOf(workflow) {
  return workflow.on ?? workflow[true];
}

function assertPinnedActions(workflow, label) {
  for (const [jobName, job] of Object.entries(workflow.jobs ?? {})) {
    for (const step of job.steps ?? []) {
      if (typeof step.uses !== "string") continue;
      assert(
        /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+(?:\/[A-Za-z0-9_.\/-]+)?@[0-9a-f]{40}$/.test(step.uses),
        `${label} job ${jobName} has an unpinned action: ${step.uses}`,
      );
    }
  }
}

function assertProvenance(workflow, label) {
  for (const [name, value] of Object.entries(expectedProvenance)) {
    assert(String(workflow.env?.[name]) === value, `${label} has wrong ${name}`);
  }
}

const ci = readWorkflow(process.argv[2] ?? ".github/workflows/ci.yml");
const release = readWorkflow(process.argv[3] ?? ".github/workflows/release.yml");
const ciTriggers = triggerOf(ci);
const releaseTriggers = triggerOf(release);

assert(ciTriggers?.pull_request === undefined, "CI must not expose private Aura access to pull requests");
assert(releaseTriggers?.pull_request === undefined, "release workflow must not run for pull requests");
assert(releaseTriggers?.push?.tags !== undefined, "release workflow must be tag-triggered");
assert(ci.permissions?.contents === "read", "CI contents permission must be read-only");
assert(ci.permissions?.actions === "read", "CI actions permission must be read-only");
assert(release.permissions?.contents === "write", "release contents permission must be write scope");
assert(release.concurrency?.group !== undefined, "release concurrency is missing");
assert(release.concurrency?.["cancel-in-progress"] === false, "release must not cancel active publication");
assertProvenance(ci, "CI");
assertProvenance(release, "release workflow");
assertPinnedActions(ci, "CI");
assertPinnedActions(release, "release workflow");

const matrix = ci.jobs?.build?.strategy?.matrix?.include;
assert(Array.isArray(matrix), "CI build matrix is missing");
assert(
  JSON.stringify(matrix.map((entry) => entry.platform)) === JSON.stringify(expectedPlatforms),
  "CI platform matrix differs",
);
assert(ci.jobs?.build?.strategy?.["fail-fast"] === false, "CI matrix must retain all evidence");
assert(ci.jobs?.manifest?.needs?.includes?.("build"), "manifest job must depend on all builds");
const audit = ci.jobs?.audit;
assert(audit !== undefined, "CI audit job is missing");
assert(audit["runs-on"] === "ubuntu-24.04", "CI audit job must use ubuntu-24.04");
assert(audit.if === undefined, "CI audit job must run without a condition");
assert(audit["continue-on-error"] !== true, "CI audit job must not tolerate failure");
const auditStep = audit.steps?.find((candidate) => candidate.name === "Check workflow policy and repository secrets");
assert(auditStep !== undefined, "CI audit gate step is missing");
assert(auditStep.if === undefined && auditStep["continue-on-error"] !== true,
  "CI audit gate must not be conditional or tolerate failure");
assert(auditStep.run?.includes("go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.12"),
  "CI audit gate must run pinned actionlint");
assert(auditStep.run?.includes("go run github.com/zricethezav/gitleaks/v8@v8.30.1 detect --source . --no-banner --redact --no-git"),
  "CI audit gate must run pinned gitleaks");
const buildNeeds = ci.jobs?.build?.needs;
assert(Array.isArray(buildNeeds) && buildNeeds.includes("resolve_aura") && buildNeeds.includes("audit"),
  "CI build job must depend on the audit job and resolve_aura");

function stepByName(name) {
  const step = ci.jobs.build.steps.find((candidate) => candidate.name === name);
  assert(step !== undefined, `CI step is missing: ${name}`);
  return step;
}
const integration = stepByName("Test Java Provider and build native Host");
assert(ci.jobs.build.if === undefined && integration.if === undefined,
  "real-process integration must run on every platform without a condition");
assert(ci.jobs.build["continue-on-error"] !== true && integration["continue-on-error"] !== true,
  "real-process integration must not tolerate failure");
assert(integration.shell === "pwsh", "real-process integration requires the cross-platform PowerShell setup");
assert(integration.run.includes("$env:AURA_WASM_PROCESS_HOST = (Resolve-Path") &&
  integration.run.includes("${{ matrix.target }}/release") && integration.run.includes("${{ matrix.platform }}"),
  "real-process integration must receive the current platform native Host");
assert(integration.run.includes("$env:AURA_WASM_COMPONENT = (Resolve-Path 'target/wasm32-wasip1/release/launch_hook.wasm').Path"),
  "real-process integration must receive the built sample component");
const buildIndex = integration.run.indexOf("cargo build --release");
const testIndex = integration.run.indexOf("gradle -p host-plugin test jar --rerun-tasks");
assert(buildIndex >= 0 && testIndex > buildIndex,
  "native Host must be built before mandatory Java real-process tests");
assert(!/--exclude-task|-x\s|--tests\s/.test(integration.run),
  "real-process integration must not filter out tests");
const toolCheck = stepByName("Check Rust, Component, and packaging tools");
for (const command of [
  "cargo fmt --all --check",
  "cargo component build --manifest-path examples/launch-hook/Cargo.toml --release",
  "cargo clippy --workspace --all-targets --target '${{ matrix.target }}' -- -D warnings",
  "cargo test --workspace --target '${{ matrix.target }}'",
  "cargo test --manifest-path sdk/rust/aura-wasm-guest/Cargo.toml",
]) {
  const commandIndex = toolCheck.run.indexOf(command);
  const guardIndex = toolCheck.run.indexOf("if ($LASTEXITCODE -ne 0)", commandIndex);
  assert(commandIndex >= 0 && guardIndex > commandIndex,
    `CI tool check must guard native command failures: ${command}`);
}
const authorTools = stepByName("Install locked author tools");
for (const command of [
  "npm --prefix sdk ci --ignore-scripts",
  "npm --prefix sdk audit --audit-level=moderate",
  "cargo install cargo-component@0.21.1 --locked",
]) {
  const commandIndex = authorTools.run.indexOf(command);
  const guardIndex = authorTools.run.indexOf("if ($LASTEXITCODE -ne 0)", commandIndex);
  assert(commandIndex >= 0 && guardIndex > commandIndex,
    `CI author tool setup must guard native command failures: ${command}`);
}
const packaging = stepByName("Package and validate platform NPL");
assert(packaging.if === undefined && packaging["continue-on-error"] !== true &&
  packaging.run.includes("test-built-host-npl.ps1"), "every built NPL must pass behavioral rejection tests");

const ciText = JSON.stringify(ci);
const releaseText = JSON.stringify(release);
assert(ciText.includes("cargo-component@0.21.1"), "CI must install the pinned Component author tool");
assert(ciText.includes("examples/launch-hook/Cargo.toml"), "CI must build the real launch-hook component");
assert(ciText.includes("test-verify-wasm-host-artifacts.ps1"), "CI must run artifact mutation tests");
assert(releaseText.includes("verify-wasm-host-artifacts.ps1"), "release must reverify downloaded NPLs");
assert(releaseText.includes("aura-runtime.wit"), "release SDK must include the frozen WIT contract");
assert(releaseText.includes("aura-wasm-guest"), "release SDK must include the Rust guest helpers");
assert(releaseText.includes("--draft"), "release must remain draft until public verification");
assert(releaseText.includes("--prerelease"), "release must be marked prerelease");

const metadata = JSON.parse(execFileSync("cargo", ["metadata", "--format-version", "1", "--locked"], {
  encoding: "utf8",
  maxBuffer: 16 * 1024 * 1024,
}));
const wasmtime = metadata.packages.find((entry) => entry.name === "wasmtime");
assert(wasmtime?.version === "48.0.1", "Cargo.lock must resolve Wasmtime 48.0.1 exactly");

const witAttribute = execFileSync("git", ["check-attr", "eol", "--", "sdk/wit/aura-runtime.wit"], {
  encoding: "utf8",
}).trim();
assert(witAttribute.endsWith(": eol: lf"), "WIT checkout bytes must use LF on every runner");

process.stdout.write("Wasm CI workflow contracts passed\n");
