import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const contractRoot = path.join(root, "contracts", "v1");
const manifestPath = path.join(contractRoot, "contract-manifest.json");
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));

assert.equal(manifest.contractVersion, 1, "Unsupported shared contract version");
assert.equal(
  manifest.canonicalRepository,
  "johngraven75/CinaVault-Premium",
  "Canonical contract repository changed unexpectedly",
);
assert.deepEqual(
  manifest.requiredPlatforms,
  ["rust", "kotlin", "swift"],
  "Shared contract must require Rust, Kotlin, and Swift",
);

function sha256(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function canonicalTextBytes(bytes) {
  const text = bytes.toString("utf8").replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  return Buffer.from(text, "utf8");
}

function readFixture(name) {
  const filePath = path.join(contractRoot, "golden", name);
  const sourceBytes = fs.readFileSync(filePath);
  const bytes = canonicalTextBytes(sourceBytes);
  const expected = manifest.fixtures?.[name]?.sha256;
  assert.equal(typeof expected, "string", `Missing hash for ${name}`);
  assert.equal(
    sha256(bytes),
    expected,
    `Canonical fixture hash drifted after line-ending normalization: ${name}`,
  );
  return JSON.parse(bytes.toString("utf8"));
}

const metadata = readFixture("metadata-provider-registry.json");
assert.equal(metadata.schemaVersion, 1);
assert.equal(metadata.policy, "all_providers_enabled");
assert.equal(metadata.credentialsStorage, "native_secure_store");
assert.equal(metadata.portableAcrossOperatingSystems, true);
assert(Array.isArray(metadata.providers) && metadata.providers.length > 0);
assert(metadata.providers.every((provider) => provider.enabled === true));
assert.equal(
  new Set(metadata.providers.map((provider) => provider.id)).size,
  metadata.providers.length,
  "Metadata provider IDs must be unique",
);
for (const provider of metadata.providers) {
  for (const field of [
    "id",
    "name",
    "category",
    "enabled",
    "requiresKey",
    "implemented",
    "endpoint",
    "customEndpoint",
  ]) {
    assert(Object.hasOwn(provider, field), `Provider contract missing ${field}`);
  }
}

const artwork = readFixture("artwork-cache-entry.json");
assert.equal(artwork.schemaVersion, 1);
assert.match(artwork.mediaKey, /^media_[A-Za-z0-9_-]+$/);
assert(["poster", "backdrop", "thumbnail"].includes(artwork.kind));
assert.match(artwork.mimeType, /^image\//);
assert(Number.isInteger(artwork.byteLength) && artwork.byteLength > 0);
assert(artwork.byteLength <= 25 * 1024 * 1024);
assert.match(artwork.sha256, /^[a-f0-9]{64}$/);
assert(Number.isInteger(artwork.width) && artwork.width > 0);
assert(Number.isInteger(artwork.height) && artwork.height > 0);
assert.equal(artwork.localPathExposed, false);
assert.match(artwork.deliveryPath, /^\/api\/artwork\//);

const rustContract = fs.readFileSync(
  path.join(root, "src-tauri", "src", "shared_contracts.rs"),
  "utf8",
);
for (const marker of [
  "trait MetadataProviderRegistryInterface",
  "trait ArtworkCacheInterface",
  manifest.fixtures["metadata-provider-registry.json"].sha256,
  manifest.fixtures["artwork-cache-entry.json"].sha256,
]) {
  assert(rustContract.includes(marker), `Rust contract is missing: ${marker}`);
}

console.log(
  `Shared contract v${manifest.contractVersion} verified for ${manifest.requiredPlatforms.join(
    ", ",
  )}.`,
);
