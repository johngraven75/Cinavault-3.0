import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const recordPath = path.join(root, "build-verification", "current-build.json");
const policyPath = path.join(root, "docs", "MASTER_BUILD_COMPLETION_GATE.md");
const modeArgument = process.argv.find((value) => value.startsWith("--mode="));
const mode = modeArgument?.split("=")[1] || process.env.MASTER_GATE_MODE || "development";

const allowedStatuses = new Set([
  "verified",
  "fixed_and_verified",
  "fixed_not_executable_in_environment",
  "external_blocker",
  "in_progress",
  "not_applicable",
]);
const releaseStatuses = new Set([
  "verified",
  "fixed_and_verified",
  "external_blocker",
  "not_applicable",
]);
const requiredPhases = [
  "repositoryAudit",
  "rootCauseDiagnosis",
  "implementation",
  "dependencyConfiguration",
  "staticReview",
  "cleanBuild",
  "automatedTests",
  "runtimeUi",
  "persistenceMigration",
  "networkApi",
  "securityPrivacy",
  "performanceReliability",
  "crossPlatformParity",
  "packaging",
  "installationLaunch",
  "finalRegression",
];
const requiredAcceptanceCriteria = [
  "liveMetadataPull",
  "metadataDatabaseWrite",
  "posterInformation",
  "posterBytes",
  "secureArtworkDelivery",
  "correctLibraryCard",
  "uncappedLibraryTotal",
  "providerPersistence",
  "windowsInstallers",
];

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function normalizeRepositoryId(value) {
  return String(value || "").trim().replace(/\\/g, "/").toLowerCase();
}

function validateEntry(name, entry, releaseMode) {
  assert(entry && typeof entry === "object", `${name} must be an object`);
  assert(allowedStatuses.has(entry.status), `${name} has invalid status: ${entry.status}`);
  assert(
    typeof entry.evidence === "string" && entry.evidence.trim().length >= 12,
    `${name} must include concrete evidence`,
  );
  if (releaseMode) {
    assert(releaseStatuses.has(entry.status), `${name} is not release-complete: ${entry.status}`);
    if (entry.status === "external_blocker" || entry.status === "not_applicable") {
      assert(
        entry.evidence.length >= 40,
        `${name} must explain the exact blocker or architectural reason`,
      );
    }
  }
}

assert(fs.existsSync(policyPath), "Master build completion policy is missing");
assert(fs.existsSync(recordPath), "Current build verification record is missing");
const policy = fs.readFileSync(policyPath, "utf8");
for (const marker of [
  "Required workflow",
  "Release rule",
  "Cross-platform rule",
  "Persistent-state rule",
  "Current metadata/poster release requirements",
]) {
  assert(policy.includes(marker), `Master completion policy is missing section: ${marker}`);
}

const record = readJson(recordPath);
assert.equal(record.schemaVersion, 1, "Unsupported master gate schema version");
assert(typeof record.build === "string" && record.build.trim(), "Build identity is required");
assert(Array.isArray(record.platforms), "Platform list is required");
for (const platform of ["windows", "linux-ubuntu", "android", "ios"]) {
  assert(record.platforms.includes(platform), `Platform parity scope is missing ${platform}`);
}

assert(Array.isArray(record.excludedRepositories), "excludedRepositories must be an array");
const excludedRepositories = new Set(record.excludedRepositories.map(normalizeRepositoryId));
assert(
  excludedRepositories.has(normalizeRepositoryId("johngraven75/Cinavault-Reimagined")),
  "Cinavault-Reimagined must remain explicitly excluded",
);

const releaseMode = mode === "release";
for (const phase of requiredPhases) {
  validateEntry(`phase.${phase}`, record.phases?.[phase], releaseMode);
}
for (const criterion of requiredAcceptanceCriteria) {
  validateEntry(`acceptance.${criterion}`, record.acceptanceCriteria?.[criterion], releaseMode);
}

assert(Array.isArray(record.externalBlockers), "externalBlockers must be an array");
assert(Array.isArray(record.criticalDefects), "criticalDefects must be an array");

if (releaseMode) {
  assert.equal(record.status, "verified", "Overall build status must be verified");
  assert.equal(record.releaseAuthorized, true, "Release authorization must be true");
  assert.equal(record.criticalDefects.length, 0, "Critical defects remain unresolved");
  for (const blocker of record.externalBlockers) {
    assert(typeof blocker === "object", "Each external blocker must be structured");
    for (const field of ["cause", "platform", "reason", "requiredResolution"]) {
      assert(
        typeof blocker[field] === "string" && blocker[field].trim().length >= 8,
        `External blocker is missing ${field}`,
      );
    }
  }
}

console.log(
  `Master build completion gate passed in ${mode} mode for ${record.build}. Release authorized: ${record.releaseAuthorized}.`,
);
