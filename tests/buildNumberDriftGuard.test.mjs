import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(resolve(root, path), "utf8");
const readJson = (path) => JSON.parse(read(path));

function cargoVersion(text) {
  const match = text.match(/^version\s*=\s*"([^"]+)"/m);
  assert.ok(match, "Cargo.toml package version was not found");
  return match[1];
}

test("authoritative build identity is internally consistent", () => {
  const manifest = readJson("build-version.json");
  const expectedDisplayName = `${manifest.releaseCycle} Build ${manifest.displayBuild}`;

  assert.equal(manifest.schemaVersion, 1);
  assert.equal(manifest.displayName, expectedDisplayName);
  assert.match(manifest.semanticVersion, /^\d+\.\d+\.\d+$/);
  assert.match(manifest.releaseTag, /^v2-build-\d+\.\d+$/);
});

test("version manifests carry forward together for the current build", () => {
  const manifest = readJson("build-version.json");
  const packageJson = readJson("package.json");
  const packageLock = readJson("package-lock.json");
  const tauriConfig = readJson("src-tauri/tauri.conf.json");
  const cargoToml = read("src-tauri/Cargo.toml");

  assert.equal(packageJson.version, manifest.semanticVersion);
  assert.equal(packageLock.version, manifest.semanticVersion);
  assert.equal(packageLock.packages[""].version, manifest.semanticVersion);
  assert.equal(tauriConfig.version, manifest.semanticVersion);
  assert.equal(cargoVersion(cargoToml), manifest.semanticVersion);
  assert.match(tauriConfig.app.windows[0].title, new RegExp(manifest.displayName.replaceAll(".", "\\.")));
});

test("current Windows release workflow uses the authoritative identity", () => {
  const manifest = readJson("build-version.json");
  const workflow = read(".github/workflows/v2-build-1-04-release.yml");

  assert.match(workflow, new RegExp(`APP_VERSION:\\s*${manifest.semanticVersion.replaceAll(".", "\\.")}`));
  assert.match(workflow, new RegExp(`TAG_NAME:\\s*v${manifest.semanticVersion.replaceAll(".", "\\.")}`));
  assert.ok(workflow.includes(manifest.displayName));
  assert.ok(workflow.includes(manifest.releaseTag));
  assert.ok(workflow.includes("SHA256SUMS.txt"));
  assert.ok(workflow.includes("fail_on_unmatched_files: true"));
});
