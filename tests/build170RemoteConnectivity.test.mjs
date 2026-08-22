import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function read(relativePath) {
  const absolutePath = resolve(ROOT, relativePath);
  assert.ok(existsSync(absolutePath), `Required carry-forward file is missing: ${relativePath}`);
  return readFileSync(absolutePath, "utf8").replace(/\r\n/g, "\n");
}

function readJson(relativePath) {
  return JSON.parse(read(relativePath));
}

function requireTokens(source, tokens, label) {
  for (const token of tokens) {
    assert.ok(source.includes(token), `${label} token missing: ${token}`);
  }
}

test("Build 170 native connectivity remains present in the current build", () => {
  const source = read("src-tauri/src/remote_connectivity.rs");
  requireTokens(source, [
    "start_remote_connectivity",
    "stop_remote_connectivity",
    "get_remote_connectivity_status",
    "map_upnp",
    "map_nat_pmp",
    "CINAVAULT_CLOUDFLARE_TUNNEL_TOKEN",
    ".trycloudflare.com",
    "encrypted_transport_required",
  ], "Build 170 native connectivity");
});

test("current Tauri startup preserves encrypted remote transport", () => {
  const main = read("src-tauri/src/main.rs");
  const identity = read("src-tauri/src/build_identity.rs");
  const build = readJson("build-version.json");
  requireTokens(main, [
    "mod remote_connectivity;",
    "remote_connectivity::configure",
    "remote_connectivity::start_remote_connectivity",
    "build_identity::get_current_build_info()",
  ], "current main wiring");
  requireTokens(identity, [
    'include_str!("../../build-version.json")',
    '"encryptedRemoteTransport": true',
    '"opaqueRemoteMediaKeys": true',
    '"aiMediaAutopilot": true',
  ], "authoritative build identity");
  assert.equal(readJson("package.json").version, build.semanticVersion);
});

test("remote API hides local paths and exposes opaque media keys", () => {
  const source = read("src-tauri/src/embedded_server.rs");
  requireTokens(source, [
    "struct RemoteMediaItem",
    "media_key",
    "REMOTE_MEDIA_KEY_DOMAIN",
    'format!("/api/artwork/{key}/{kind}")',
    'format!("/api/stream/{key}")',
    "local_paths_exposed: false",
  ], "remote API");
  const remoteStruct = source.slice(source.indexOf("struct RemoteMediaItem"), source.indexOf("fn open_database"));
  assert.ok(!remoteStruct.includes("file_path"), "RemoteMediaItem must never serialize a local file path");
});

test("source ingestion scans enriches and refreshes through pagination", () => {
  const source = read("src/components/tabs/MediaSourcesTab.tsx");
  requireTokens(source, [
    'invoke<number>("add_source"',
    'invoke<ScanResult>("scan_single_source"',
    '"run_library_enrichment"',
    'new Event("cinavault:source-added")',
    'new CustomEvent("cinavault:library-refresh"',
    "media cards and posters will reload in pages",
    "AI is identifying media and retrieving posters",
  ], "source pipeline");
  assert.ok(!source.includes('invoke<MediaItem[]>("get_media_items")'), "Sources tab must not perform an uncapped full-library fetch");
  assert.ok(!source.includes("DEMO_SOURCES"), "Backend source failures must not be hidden behind demo sources");
});

test("AI Media Autopilot still manages recurring library work", () => {
  const source = read("src/services/aiMediaAutopilot.ts");
  requireTokens(source, [
    '"scan_sources"',
    '"run_library_enrichment"',
    '"check_media_item_metadata"',
    '"get_media_items"',
    '"cinavault:library-refresh"',
    "setInterval",
  ], "AI Media Autopilot");
});

test("structural spatial redesign remains intact", () => {
  requireTokens(read("src/App.tsx"), ["ExperienceBackdrop", "cv-command-deck", "cv-workspace-panel"], "application shell");
  requireTokens(read("src/components/Header.tsx"), ["Ctrl K", "cv-command-palette", "BUILD_INFO.displayName"], "command deck");
  requireTokens(read("src/components/Sidebar.tsx"), ["Spatial Media OS", "Casting Center", "BUILD_INFO.displayName"], "navigation");
  requireTokens(read("src/styles/experience-shell.css"), [".cv-command-deck", ".cv-command-palette", "prefers-reduced-motion"], "experience styles");
});

test("current packaging follows the authoritative manifest", () => {
  const build = readJson("build-version.json");
  const config = readJson("src-tauri/tauri.conf.json");
  const packageJson = readJson("package.json");
  const cargo = read("src-tauri/Cargo.toml");
  assert.equal(config.version, build.semanticVersion);
  assert.equal(packageJson.version, build.semanticVersion);
  assert.ok(cargo.includes(`version = "${build.semanticVersion}"`));
  assert.ok(config.bundle.resources.includes("tools/cloudflared/*"));
});

test("Cargo dependencies retain both NAT traversal protocols", () => {
  const cargo = read("src-tauri/Cargo.toml");
  assert.ok(cargo.includes('igd-next = "0.17.1"'));
  assert.ok(cargo.includes('natpmp = { version = "0.5", features = ["tokio"] }'));
});
