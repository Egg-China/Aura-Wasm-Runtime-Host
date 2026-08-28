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
  AURA_COMMIT: "c2d7ec3201825308c360c1a41aeafebcd7145e74",
  AURA_RUN_ID: "33196503483",
  AURA_JAR_SHA256: "2153be49da69c055232872c95a171091a526be24357b6f2b82b5af8f6d2a67c3",
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

const ci = readWorkflow(".github/workflows/ci.yml");
const release = readWorkflow(".github/workflows/release.yml");
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

process.stdout.write("Wasm CI workflow contracts passed\n");
