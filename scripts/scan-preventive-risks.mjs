import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const build = JSON.parse(fs.readFileSync(path.join(root, "build-version.json"), "utf8"));
const findings = [];

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

function requireMarker(relativePath, marker, reason) {
  const content = read(relativePath);
  if (!content.includes(marker)) {
    findings.push({ severity: "high", file: relativePath, reason });
  }
}

function rejectPattern(relativePath, pattern, reason) {
  const content = read(relativePath);
  if (pattern.test(content)) {
    findings.push({ severity: "high", file: relativePath, reason });
  }
}

function rejectCrlf(relativePath) {
  if (process.platform === "win32") return;
  const content = fs.readFileSync(path.join(root, relativePath));
  if (content.includes(Buffer.from("\r\n"))) {
    findings.push({
      severity: "medium",
      file: relativePath,
      reason: "CRLF line endings can recreate Windows-only exact-string test failures",
    });
  }
}

for (const required of [
  "schemaVersion",
  "productName",
  "semanticVersion",
  "displayBuild",
  "displayName",
  "releaseTag",
]) {
  assert(Object.hasOwn(build, required), `build-version.json missing ${required}`);
}
assert.equal(build.schemaVersion, 1);
assert.match(build.semanticVersion, /^\d+\.\d+\.\d+$/);
assert.match(build.displayName, /^v\d+(?:\.\d+)? Build \d+(?:\.\d+)?$/);
assert.match(build.releaseTag, /^v\d+-build-\d+(?:\.\d+)?$/);

requireMarker("src/buildInfo.ts", 'import manifest from "../build-version.json"', "UI build identity must derive from build-version.json");
requireMarker("src/main.tsx", "BUILD_INFO.displayName", "Startup diagnostics must use the authoritative build identity");
requireMarker("src/components/Header.tsx", 'import { BUILD_INFO } from "../buildInfo"', "Header build label must use the authoritative build identity");
requireMarker("src/components/Sidebar.tsx", 'import { BUILD_INFO } from "../buildInfo"', "Sidebar build label must use the authoritative build identity");
requireMarker("src-tauri/src/build_identity.rs", 'include_str!("../../build-version.json")', "Rust build identity must derive from build-version.json");
requireMarker("src-tauri/src/lib.rs", "build_identity::get_current_build_info", "Tauri runtime app info must use the typed manifest-driven build identity");
requireMarker("src-tauri/src/main.rs", "cinavault_3_lib::run();", "Windows binary entrypoint must execute the repaired shared Tauri runtime");
requireMarker(".github/workflows/release-build-170.yml", "npm run verify:master-release", "Master-gated packaging must remain blocked by the master release gate");

const packageJson = JSON.parse(read("package.json"));
if (packageJson.version !== build.semanticVersion) findings.push({ severity: "high", file: "package.json", reason: `Package version ${packageJson.version} does not match ${build.semanticVersion}` });
const cargoManifest = read("src-tauri/Cargo.toml");
if (!cargoManifest.includes(`version = "${build.semanticVersion}"`)) findings.push({ severity: "high", file: "src-tauri/Cargo.toml", reason: `Cargo package version does not match ${build.semanticVersion}` });
const tauriConfiguration = JSON.parse(read("src-tauri/tauri.conf.json"));
if (tauriConfiguration.version !== build.semanticVersion) findings.push({ severity: "high", file: "src-tauri/tauri.conf.json", reason: `Tauri bundle version ${tauriConfiguration.version} does not match ${build.semanticVersion}` });

const staleBuildPattern = /(?:Build 168|Build 170|v2 Build 1\.0[0-3]|1\.6\.8|1\.7\.170)/;
for (const relativePath of ["src/components/Header.tsx", "src/components/Sidebar.tsx", "src-tauri/src/lib.rs", "src-tauri/src/main.rs", "src-tauri/src/embedded_server.rs"]) {
  rejectPattern(relativePath, staleBuildPattern, `User-facing runtime code contains a stale build identity instead of ${build.displayName}`);
}

for (const relativePath of ["build-version.json", "scripts/verify-master-build-gate.mjs", "scripts/verify-shared-contracts.mjs", "scripts/scan-preventive-risks.mjs", "contracts/v1/golden/metadata-provider-registry.json", "contracts/v1/golden/artwork-cache-entry.json"]) rejectCrlf(relativePath);

const commandPalette = read("src/components/Header.tsx");
if (/cv-command-palette[\s\S]{0,500}scale:\s*0\./.test(commandPalette)) findings.push({ severity: "high", file: "src/components/Header.tsx", reason: "Command palette reintroduced scale composition associated with WebView2 stalls" });

const providerRegistry = read("src-tauri/src/metadata_provider_config.rs");
if (!providerRegistry.includes("provider.enabled = true")) findings.push({ severity: "high", file: "src-tauri/src/metadata_provider_config.rs", reason: "Provider migration does not enforce the all-enabled policy" });
if (/api[_-]?key|access[_-]?token|client[_-]?secret/i.test(read("contracts/v1/golden/metadata-provider-registry.json"))) findings.push({ severity: "critical", file: "contracts/v1/golden/metadata-provider-registry.json", reason: "Portable provider contract appears to contain credentials" });

for (const [marker, reason] of [
  ["metadata_enrichment_runtime::check_media_item_metadata", "Check Metadata must route through the keyless/cached enrichment runtime"],
  ["metadata_enrichment_runtime::run_library_enrichment", "Library enrichment must route through the keyless/cached enrichment runtime"],
  ["pub app_data_dir: PathBuf", "AppState must expose the application-data root for durable artwork caching"],
  ['create_dir_all(app_dir.join("artwork"))', "Startup must create the application-owned artwork cache"],
]) requireMarker("src-tauri/src/lib.rs", marker, reason);

for (const [marker, reason] of [
  ["metadata_ext_without_repaired_commands.rs", "Build script must generate a metadata extension fallback without the wrapped command macro"],
  ["metadata_guard_without_commands.rs", "Build script must sanitize internal metadata guard command macros"],
  ['&["run_library_enrichment"]', "Build script must keep legacy enrichment callable without exporting a duplicate command"],
]) requireMarker("src-tauri/build.rs", marker, reason);

const assetProtocol = tauriConfiguration?.app?.security?.assetProtocol;
if (assetProtocol?.enable !== true) findings.push({ severity: "high", file: "src-tauri/tauri.conf.json", reason: "Tauri asset protocol must be enabled so cached local posters can render" });
if (!Array.isArray(assetProtocol?.scope) || !assetProtocol.scope.includes("$APPDATA/artwork/**/*")) findings.push({ severity: "high", file: "src-tauri/tauri.conf.json", reason: "Tauri asset scope must include only the CinaVault application artwork cache" });

for (const [marker, reason] of [
  ["https://api.tvmaze.com/search/shows", "Keyless TV metadata provider is missing"],
  ["https://v3-cinemeta.strem.io", "Keyless movie metadata provider is missing"],
  ["fetch_keyless_match", "Media-type-aware keyless provider routing is missing"],
  ['app_data_dir.join("artwork")', "Artwork is not cached under application-owned storage"],
  ["Remote artwork must use HTTPS", "Artwork cache no longer enforces encrypted provider transport"],
]) requireMarker("src-tauri/src/metadata_keyless.rs", marker, reason);

for (const [marker, reason] of [
  ["run_keyless_prepass", "Bulk enrichment no longer performs the keyless metadata prepass"],
  ["fetch_keyless_match", "Metadata runtime no longer routes by media type to keyless providers"],
  ["cache_remote_artwork", "Metadata runtime no longer persists provider poster bytes"],
  ["update_media_metadata_data", "Metadata runtime no longer writes resolved metadata back to SQLite"],
  ["live_metadata_poster_acceptance_tvmaze_series", "Live TV metadata/poster acceptance test is missing"],
  ["live_metadata_poster_acceptance_cinemeta_movie", "Live movie metadata/poster acceptance test is missing"],
]) requireMarker("src-tauri/src/metadata_enrichment_runtime.rs", marker, reason);

requireMarker("package.json", "live_metadata_poster_acceptance_", "Live metadata command must execute both TVMaze and Cinemeta acceptance fixtures");
requireMarker("src/components/tabs/HomeTab.tsx", "convertFileSrc(path)", "Library media cards must convert application-cache poster paths into renderable asset URLs");

const installerWorkflows = [".github/workflows/windows-build-1-04-validation.yml", ".github/workflows/v2-build-1-04-release.yml", ".github/workflows/windows-installer.yml"];
for (const workflow of installerWorkflows) {
  for (const [marker, reason] of [
    ["npm run test:metadata-live", "Installer workflow must run live metadata acceptance"],
    ["live_metadata_poster_acceptance_tvmaze_series", "Installer workflow must prove the TVMaze series acceptance test executed"],
    ["live_metadata_poster_acceptance_cinemeta_movie", "Installer workflow must prove the Cinemeta movie acceptance test executed"],
    ["test result: ok\\. 2 passed; 0 failed", "Installer workflow must require exactly two passing live acceptance tests"],
  ]) requireMarker(workflow, marker, reason);
}
requireMarker(".github/workflows/windows-installer.yml", "npm run verify:master-release", "Manual release publication must reconfirm the master release gate");

fs.mkdirSync(path.join(root, "master-evidence"), { recursive: true });
fs.writeFileSync(path.join(root, "master-evidence", "preventive-risk-findings.json"), `${JSON.stringify({ build: build.displayName, findings }, null, 2)}\n`);

if (findings.length > 0) {
  for (const finding of findings) console.error(`[${finding.severity}] ${finding.file}: ${finding.reason}`);
  process.exitCode = 1;
} else {
  console.log(`Preventive risk scan passed for ${build.displayName}.`);
}
