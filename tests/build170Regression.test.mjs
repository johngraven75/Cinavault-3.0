import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function read(relativePath) {
  const absolutePath = resolve(ROOT, relativePath);
  assert.ok(
    existsSync(absolutePath),
    `Required Build 170 file is missing: ${relativePath}`,
  );
  return readFileSync(absolutePath, "utf8").replace(/\r\n/g, "\n");
}

function requireTokens(source, tokens, label) {
  for (const token of tokens) {
    assert.ok(source.includes(token), `${label} token missing: ${token}`);
  }
}

test("Build 170 native connectivity includes NAT traversal and encrypted relay", () => {
  const source = read("src-tauri/src/remote_connectivity.rs");
  requireTokens(
    source,
    [
      "start_remote_connectivity",
      "stop_remote_connectivity",
      "get_remote_connectivity_status",
      "map_upnp",
      "map_nat_pmp",
      "PortMappingProtocol::TCP",
      "Protocol::TCP",
      "CINAVAULT_CLOUDFLARE_TUNNEL_TOKEN",
      "CINAVAULT_CLOUDFLARE_PUBLIC_URL",
      ".trycloudflare.com",
      "MAPPING_RENEW_SECONDS",
      "encrypted_transport_required",
      'filter(|value| value.starts_with("https://"))',
    ],
    "Build 170 native connectivity",
  );
  assert.ok(
    source.includes("status.preferred_url = status"),
    "preferred remote client URL must be derived from the encrypted relay",
  );
});

test("Build 170 Tauri startup prefers encrypted remote transport", () => {
  const main = read("src-tauri/src/lib.rs");
  const identity = read("src-tauri/src/build_identity.rs");
  requireTokens(
    main,
    [
      "mod remote_connectivity;",
      "remote_connectivity::configure",
      "remote_connectivity::start_remote_connectivity",
      "remote_connectivity::stop_remote_connectivity",
      "remote_connectivity::get_remote_connectivity_status",
    ],
    "Build 170 main wiring",
  );
  requireTokens(identity, [
    '"automaticNatTraversal": true',
    '"cloudRelayFallback": true',
    '"encryptedRemoteTransport": true',
    '"opaqueRemoteMediaKeys": true',
    '"aiMediaAutopilot": true',
    '"spatialExperienceShell": true',
  ], "current build identity");
});

test("Build 170 remote API hides local paths and exposes opaque media keys", () => {
  const source = read("src-tauri/src/embedded_server.rs");
  requireTokens(
    source,
    [
      "struct RemoteMediaItem",
      "media_key",
      "REMOTE_MEDIA_KEY_DOMAIN",
      "Sha256",
      '"/api/artwork/{media_key}"',
      '"/api/stream/{media_key}"',
      "local_paths_exposed: false",
      'remote_transport: "HTTPS relay"',
      "private, no-store, max-age=0",
      "x-content-type-options",
    ],
    "Build 170 remote API",
  );
  const remoteStruct = source.slice(
    source.indexOf("struct RemoteMediaItem"),
    source.indexOf("fn open_database"),
  );
  assert.ok(
    !remoteStruct.includes("file_path"),
    "RemoteMediaItem must never serialize a local file path",
  );
});

test("Build 170 source ingestion scans enriches and refreshes immediately", () => {
  const source = read("src/components/tabs/MediaSourcesTab.tsx");
  requireTokens(
    source,
    [
      'invoke<number>("add_source"',
      'invoke<ScanResult>("scan_single_source"',
      '"run_library_enrichment"',
      'new Event("cinavault:source-added")',
      'new CustomEvent("cinavault:library-refresh"',
      "AI is identifying media and retrieving posters",
    ],
    "Build 170 source pipeline",
  );
  assert.ok(
    !source.includes("DEMO_SOURCES"),
    "Backend source failures must not be hidden behind demo sources",
  );
});

test("Build 170 AI Media Autopilot manages recurring library work", () => {
  const source = read("src/services/aiMediaAutopilot.ts");
  requireTokens(
    source,
    [
      '"scan_sources"',
      '"run_library_enrichment"',
      '"check_media_item_metadata"',
      '"purge_photo_items"',
      '"get_media_items"',
      '"cinavault:source-added"',
      '"cinavault:ai-autopilot-run"',
      '"cinavault:library-refresh"',
      "setInterval",
    ],
    "Build 170 AI Media Autopilot",
  );
});

test("Build 170 user-facing shell is a structural spatial redesign", () => {
  const app = read("src/App.tsx");
  const header = read("src/components/Header.tsx");
  const sidebar = read("src/components/Sidebar.tsx");
  const backdrop = read("src/components/experience/ExperienceBackdrop.tsx");
  const styles = read("src/styles/experience-shell.css");

  requireTokens(
    app,
    [
      "ExperienceBackdrop",
      "cv-shell-frame",
      "cv-command-deck",
      "cv-context-stage",
      "cv-stage-telemetry",
      "cv-workspace-panel",
      "startAiMediaAutopilot",
    ],
    "Build 170 application shell",
  );
  requireTokens(
    header,
    [
      "Ctrl K",
      "cv-command-palette",
      "get_embedded_server_status",
      "get_remote_connectivity_status",
      "AI active",
    ],
    "Build 170 command deck",
  );
  requireTokens(
    sidebar,
    [
      "Spatial Media OS",
      "AI Autopilot",
      "cv-orbital-nav-active",
      "Casting Center",
    ],
    "Build 170 orbital navigation",
  );
  requireTokens(
    backdrop,
    [
      "cv-aurora",
      "cv-orbit-system",
      "cv-particle-field",
      "pointermove",
    ],
    "Build 170 experience backdrop",
  );
  requireTokens(
    styles,
    [
      ".cv-command-deck",
      ".cv-context-stage",
      ".cv-command-palette",
      "@keyframes cvAuroraPulse",
      "@keyframes cvStageSweep",
      "prefers-reduced-motion",
    ],
    "Build 170 experience styles",
  );
});

test("Build 170 library defaults to eight compact cards per desktop row", () => {
  const styles = read("src/styles/build170-library.css");
  requireTokens(
    styles,
    [
      "repeat(8, minmax(0, 1fr))",
      "@media (min-width: 1260px)",
      ".cyber-card-actions .metadata-action-label",
    ],
    "Build 170 compact library",
  );
});

test("Build 170 remote access UI controls and displays live connectivity", () => {
  const ui = read("src/components/tabs/RemoteAccessTab.tsx");
  requireTokens(
    ui,
    [
      "RemoteConnectivityStatus",
      '"get_remote_connectivity_status"',
      '"start_remote_connectivity"',
      '"stop_remote_connectivity"',
      "directAvailable",
      "relayActive",
      "preferredUrl",
      "Automatic NAT Traversal",
      "Automatic Relay Fallback",
    ],
    "Build 170 remote access UI",
  );
});

test("Build 170 packaging includes relay runtime and current release version", () => {
  const config = JSON.parse(read("src-tauri/tauri.conf.json"));
  const manifest = JSON.parse(read("build-version.json"));
  assert.equal(config.version, manifest.semanticVersion);
  assert.ok(
    config.bundle.resources.includes("tools/cloudflared/*"),
    "cloudflared bundle resource is not configured",
  );
  read("src-tauri/tools/cloudflared/README.txt");
});

test("Build 170 Cargo dependencies include both NAT traversal protocols", () => {
  const cargo = read("src-tauri/Cargo.toml");
  assert.ok(cargo.includes('igd-next = "0.17.1"'));
  assert.ok(
    cargo.includes('natpmp = { version = "0.5", features = ["tokio"] }'),
  );
});
