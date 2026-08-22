import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import path from "node:path";

const scanner = fs.readFileSync("src-tauri/src/scanner.rs", "utf8");
const enrichment = fs.readFileSync("src-tauri/src/enrichment.rs", "utf8");
const metadata = fs.readFileSync("src-tauri/src/metadata.rs", "utf8");
const guard = fs.readFileSync("src-tauri/src/metadata_guard.rs", "utf8");
const database = fs.readFileSync("src-tauri/src/db.rs", "utf8");
const tauri = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const adultProviders = [
  "tpdb",
  "stashdb",
  "pgma",
  "porn_site_nuxt",
  "iafd",
  "phoenixadult",
];

test("adult media is labeled during external-drive ingestion", () => {
  assert.match(scanner, /fn scanned_media_type/);
  assert.match(scanner, /media_type: scanned_media_type\(source, file_path, media_type\)/);
  assert.match(scanner, /"adult"\.to_string\(\)/);
});

test("adult enrichment never falls back to standard providers", () => {
  const adultBranch = enrichment.match(
    /SourceKind::AdultVideo => \{([\s\S]*?)\r?\n\s*\}\r?\n\s*SourceKind::StandardVideo/,
  )?.[1];
  assert.ok(adultBranch, "adult provider branch is missing");
  assert.match(enrichment, /fetch_adult_metadata_for_batch/);
  assert.match(metadata, /configured_adult_provider_order/);
  for (const provider of adultProviders) {
    assert.match(metadata, new RegExp(provider), `${provider}: batch routing must remain adult-only`);
  }
  assert.doesNotMatch(adultBranch, /fetch_standard_metadata|fetch_tmdb_metadata|fetch_omdb_metadata/);
});

test("single-item adult metadata routes only to the adult provider chain", () => {
  assert.match(metadata, /if media_item_is_adult\(&item\) \{\s*fetch_adult_item_metadata/);
  assert.match(metadata, /download_poster_to_sidecar/);
  assert.match(metadata, /poster_download\//);
  assert.match(guard, /if adult \{[\s\S]*tpdb_match[\s\S]*stashdb_match/);
  assert.match(guard, /\} else \{[\s\S]*tmdb_match[\s\S]*omdb_match/);
});

test("every adult metadata and poster provider starts enabled with valid JSON", () => {
  const engine = JSON.parse(
    fs.readFileSync("plugins/configs/cv-metadata-engine.json", "utf8"),
  );
  for (const provider of adultProviders) {
    const configPath = path.join("plugins", "configs", `${provider}.json`);
    const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
    assert.equal(config._key, provider, `${provider}: config key mismatch`);
    assert.equal(config.enabled, true, `${provider}: must start enabled`);
    assert.equal(
      config.poster_download,
      true,
      `${provider}: poster downloads must start enabled`,
    );
    assert.ok(
      engine.adult_providers.includes(provider),
      `${provider}: missing from adult startup routing`,
    );
    assert.match(
      database,
      new RegExp(`["']${provider}["']`),
      `${provider}: missing from database startup defaults`,
    );
  }
});

test("provider JSON configs are installed in the Windows application bundle", () => {
  assert.ok(
    tauri.bundle.resources.includes("../plugins/configs/*.json"),
    "Windows bundle must install the provider JSON catalogue",
  );
});
