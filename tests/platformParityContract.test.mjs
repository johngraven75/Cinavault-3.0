import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const contractUrl = new URL("../docs/platform-parity.json", import.meta.url);
const contract = JSON.parse(await readFile(contractUrl, "utf8"));

const destinationIds = new Set(contract.destinations.map((entry) => entry.id));
const capabilityIds = new Set(contract.crossPlatformCapabilities);
const defectIds = new Set(contract.defectParity.map((entry) => entry.id));

test("platform parity contract uses the current Windows Premium reference", () => {
  assert.equal(contract.schemaVersion, 1);
  assert.equal(contract.reference.repository, "johngraven75/CinaVault-Premium");
  assert.equal(contract.reference.release, "v2-build-1.13");
  assert.equal(contract.reference.displayName, "v2.13 Build 1.13");
  assert.equal(contract.reference.semanticVersion, "2.0.13");
  assert.equal(contract.reference.platform, "windows");
  assert.deepEqual(contract.includedRepositories, [
    "johngraven75/CinaVault-Premium",
    "johngraven75/cinavault-android",
    "johngraven75/Cinavault-Server-Premium-Edition-iOS",
  ]);
  assert.deepEqual(contract.excludedRepositories, [
    {
      repository: "johngraven75/Cinavault-Reimagined",
      reason:
        "Work in this repository is exclusive to that repository and must not be copied into or out of the Premium parity program.",
    },
  ]);
});

test("all Windows primary destinations are required on Android and iOS", () => {
  for (const id of [
    "library",
    "sources",
    "downloads",
    "live-tv",
    "server",
    "security",
    "remote",
    "advanced",
    "cloud-nas",
    "extensions",
    "ai-autopilot",
    "hf-models",
    "settings",
  ]) {
    assert.ok(destinationIds.has(id), `missing destination parity requirement: ${id}`);
  }
  assert.ok(contract.destinations.every((entry) => entry.required === true));
});

test("security, automation, casting, relay, visual, and recovery parity remain mandatory", () => {
  for (const capability of [
    "encrypted-session-storage",
    "authenticated-byte-range-streaming",
    "opaque-media-identifiers",
    "metadata-and-poster-refresh",
    "automatic-source-scan-on-add",
    "casting-device-discovery",
    "automatic-nat-traversal-status",
    "https-cloud-relay-status",
    "global-command-search",
    "runtime-error-recovery",
  ]) {
    assert.ok(capabilityIds.has(capability), `missing capability: ${capability}`);
  }
});

test("all known cross-platform defect classes are tracked", () => {
  for (const id of [
    "CVP-001",
    "CVP-002",
    "CVP-003",
    "CVP-004",
    "CVP-005",
    "CVP-006",
    "CVP-007",
    "CVP-008",
    "CVP-009",
  ]) {
    assert.ok(defectIds.has(id), `missing defect parity record: ${id}`);
  }
  assert.equal(contract.changePolicy.fullFileReplacementsOnly, true);
  assert.equal(contract.changePolicy.noRegressions, true);
  assert.equal(contract.changePolicy.crossPlatformAuditRequired, true);
});
