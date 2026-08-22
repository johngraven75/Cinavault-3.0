import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const read = (path) => fs.readFileSync(path, "utf8");

test("external drives use a native picker, health validation, and root discovery", () => {
  const ui = read("src/components/tabs/MediaSourcesTab.tsx");
  const backend = read("src-tauri/src/lib.rs");
  const health = read("src-tauri/src/source_health.rs");
  const scanner = read("src-tauri/src/scanner.rs");
  assert.match(ui, /plugin-dialog/);
  assert.match(ui, /directory: newSourceType !== "file"/);
  assert.match(ui, /validate_source_path/);
  assert.match(backend, /mod source_health;/);
  assert.match(backend, /source_health::validate_source_path/);
  assert.match(backend, /source_health::explore_source_path/);
  assert.match(health, /pub fn explore_source_path/);
  assert.match(ui, /Explore Source/);
  assert.match(ui, /invoke\("explore_source_path"/);
  assert.match(ui, /Discover Drives/);
  assert.match(scanner, /platform_discovery_roots/);
  assert.match(scanner, /extension\(\).*detect_media_type/s);
});

test("AI tab exposes an ungated free-model selection window with reasoning labels", () => {
  const ui = read("src/components/tabs/AIDiagnosticsTab.tsx");
  assert.match(ui, /Hugging Face Free Model Catalog/);
  assert.match(ui, /Browse Free Models/);
  assert.match(ui, /Public, ungated choices only/);
  assert.match(ui, /REASONING/);
  assert.match(ui, /setModel\(candidate\.id\)/);
});

test("external-drive posters are loaded only through an authorized database-backed command", () => {
  const backend = read("src-tauri/src/lib.rs");
  const home = read("src/components/tabs/HomeTab.tsx");
  const kodi = read("src/components/kodi/KodiHomeLayout.tsx");
  assert.match(backend, /fn get_poster_data_url/);
  assert.match(backend, /poster_path = \?1 OR backdrop_path = \?1/);
  assert.match(backend, /MAX_POSTER_BYTES/);
  assert.match(home, /invoke<string>\("get_poster_data_url"/);
  assert.match(kodi, /invoke<string>\("get_poster_data_url"/);
});

test("explicit adult sources classify every video as adult and keep adult-only routing", () => {
  const ui = read("src/components/tabs/MediaSourcesTab.tsx");
  const scanner = read("src-tauri/src/scanner.rs");
  const enrichment = read("src-tauri/src/enrichment.rs");
  assert.match(ui, /option value="adult">Adult Media/);
  assert.match(scanner, /source\.source_type\.eq_ignore_ascii_case\("adult"\)/);
  assert.match(enrichment, /SourceKind::AdultVideo/);
  assert.doesNotMatch(enrichment, /SourceKind::AdultVideo\s*=>\s*fetch_standard_metadata/);
});

test("all Windows version sources advance together for the current release", () => {
  const manifest = JSON.parse(read("build-version.json"));
  assert.equal(JSON.parse(read("package.json")).version, manifest.semanticVersion);
  assert.match(read("src-tauri/Cargo.toml"), new RegExp(`version = "${manifest.semanticVersion.replaceAll(".", "\\.")}"`));
  assert.equal(JSON.parse(read("src-tauri/tauri.conf.json")).version, manifest.semanticVersion);
});
