/**
 * Build 165 real-work governance checks.
 * These assertions reject status-only implementations by requiring the native
 * side-effect calls and the UI-to-command routing that reaches them.
 */
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(resolve(ROOT, path), "utf8");

test("AI source discovery mutates the source database and the button invokes it", () => {
  const scanner = read("src-tauri/src/scanner.rs");
  const sourcesUi = read("src/components/tabs/MediaSourcesTab.tsx");
  assert.match(scanner, /pub async fn discover_media_sources/);
  assert.match(scanner, /discover_and_add_sources/);
  assert.match(scanner, /db\.add_source_data/);
  assert.match(
    scanner,
    /discovery_adds_real_database_sources_from_media_directories/,
  );
  assert.match(sourcesUi, /invoke[^]*"discover_media_sources"/);
  assert.doesNotMatch(
    sourcesUi,
    /const aiDiscover = \(\) => \{\s*addStatusMessage/,
    "AI discovery must not be a status-only callback",
  );
});

test("AI operational prompts and quick actions reach real native commands", () => {
  const ai = read("src-tauri/src/ai.rs");
  const automation = read("src-tauri/src/ai_automation.rs");
  const ui = read("src/components/tabs/AIDiagnosticsTab.tsx");
  assert.match(ai, /AiQueryRoute::LibraryAutomation/);
  assert.match(ai, /ai_automation::ai_library_manage/);
  assert.match(ai, /AiQueryRoute::SourceDiscovery/);
  assert.match(automation, /scanner::scan_sources/);
  assert.match(automation, /enrichment::run_library_enrichment/);
  assert.match(automation, /duplicates::find_duplicates/);
  assert.match(ui, /invoke\("ai_library_manage"/);
  assert.match(ui, /invoke\("run_library_enrichment"/);
});

test("Synology and WD shares become reachable scanner filesystem sources", () => {
  const nas = read("src-tauri/src/nas_devices.rs");
  const scanner = read("src-tauri/src/scanner.rs");
  assert.match(nas, /fn network_source_path/);
  assert.match(nas, /authenticate_windows_shares/);
  assert.match(nas, /ensure_network_source_reachable/);
  assert.match(nas, /wd_mycloud_login/);
  assert.match(nas, /cookie_store\(true\)/);
  assert.match(nas, /db\.add_source_data/);
  assert.match(scanner, /WalkDir::new\(path\)/);
  assert.doesNotMatch(
    nas,
    /let source_path = format!\("{}:\/\/{}:{}{}"/,
    "NAS libraries must not be stored as unscannable HTTP URLs",
  );
});

test("poster acquisition persists verified sidecars and media cards handle failures", () => {
  const enrichment = read("src-tauri/src/enrichment.rs");
  const home = read("src/components/tabs/HomeTab.tsx");
  const kodi = read("src/components/kodi/KodiHomeLayout.tsx");
  assert.match(enrichment, /valid_poster_payload/);
  assert.match(enrichment, /write_poster_sidecar_bytes/);
  assert.match(enrichment, /file\.sync_all/);
  assert.match(enrichment, /std::fs::rename\(&temporary_path, &sidecar_path\)/);
  assert.match(enrichment, /db\.update_media_metadata_data/);
  assert.match(
    enrichment,
    /acquired_poster_is_validated_and_atomically_written_as_a_sidecar/,
  );
  for (const ui of [home, kodi]) {
    assert.match(ui, /convertFileSrc/);
    assert.match(ui, /onError=\{\(\) => setFailed\(true\)\}/);
    assert.match(ui, /data-poster-source/);
    assert.match(ui, /data-poster-fallback/);
  }
});

test("cloud commands read real folders, persist sources, and reject fake success", () => {
  const cloud = read("src-tauri/src/cloud_storage.rs");
  const main = read("src-tauri/src/main.rs");
  assert.match(cloud, /resolve_provider_path/);
  assert.match(cloud, /list_directory_entries/);
  assert.match(cloud, /count_media_files/);
  assert.match(cloud, /db\.add_source_data/);
  assert.match(
    cloud,
    /unreadable_cloud_folder_returns_an_error_instead_of_success/,
  );
  assert.doesNotMatch(main, /Cloud sync placeholder/);
  assert.doesNotMatch(main, /Ok\(vec!\[\]\)/);
});

test("permanent media tools automatically check and install at application startup", () => {
  const tools = read("src-tauri/src/media_tools.rs");
  const app = read("src/App.tsx");
  const downloads = read("src/components/tabs/DownloadsTab.tsx");
  const catalog = read("src/plugins/permanentMediaPlugins.ts");
  for (const tool of [
    "ffmpeg",
    "ffprobe",
    "yt-dlp",
    "mediainfo",
    "mkvtoolnix",
  ]) {
    assert.match(tools, new RegExp(`id: "${tool}"`));
  }
  assert.match(tools, /--disable-interactivity/);
  assert.match(tools, /--accept-package-agreements/);
  assert.match(app, /ensurePermanentMediaPluginsAtStartup/);
  assert.match(downloads, /get_media_tools_status/);
  assert.match(downloads, /ensure_media_tools/);
  assert.doesNotMatch(catalog, /autoInstall: false/);
});

test("settings restore from the backend and all persistent slices autosave", () => {
  const app = read("src/App.tsx");
  assert.match(app, /invoke<Record<string, string>>\("get_all_settings"\)/);
  assert.match(app, /hasRestoredSettings\.current/);
  for (const slice of [
    "settings",
    "featureSettings",
    "metadataProviders",
    "scheduledTasks",
    "cloudServices",
    "libraryView",
  ]) {
    assert.match(app, new RegExp(`\\b${slice}\\b`));
  }
  assert.equal(
    (app.match(/pluginEngine\.initialize\(\)/g) || []).length,
    1,
    "theme changes must not reinitialize the plugin engine",
  );
});

test("cloud and plugin failures cannot be converted into local success", () => {
  const cloudUi = read("src/components/tabs/CloudNASTab.tsx");
  const adapter = read("src/data/pluginAdapter.ts");
  const runtime = read("src-tauri/src/plugins.rs");
  assert.match(cloudUi, /backendProvider/);
  assert.match(cloudUi, /provider: backendProvider\(id\)/);
  assert.match(cloudUi, /Sync failed/);
  assert.match(cloudUi, /Browse failed/);
  assert.doesNotMatch(cloudUi, /catch \{\}/);
  assert.doesNotMatch(adapter, /action: "configure"[^]*?catch \{\}/);
  assert.match(
    runtime,
    /no executable runtime registered; no work was performed/,
  );
  assert.match(runtime, /atomic_write/);
  assert.match(runtime, /save_manifest/);
  assert.match(
    runtime,
    /PGMA is a required adult metadata provider and cannot be removed/,
  );
});

test("startup installs only permanent plugins and adult providers default enabled", () => {
  const initialize = read("src/data/pluginAdapterInitialize.ts");
  const store = read("src/store/appStore.ts");
  assert.match(initialize, /getStartupMediaPlugins/);
  assert.match(initialize, /startupPluginIds\.has\(plugin\.id\)/);
  assert.doesNotMatch(
    initialize,
    /for \(const plugin of FULL_PLUGIN_REGISTRY\)/,
  );
  for (const id of [
    "pgma",
    "porn_site_nuxt",
    "theporndb",
    "stashdb",
    "phoenixadult",
    "iafd",
  ]) {
    assert.match(
      store,
      new RegExp(`id: "${id}"[\\s\\S]{0,160}enabled: true`),
      `${id} must be enabled at first startup`,
    );
  }
});

test("cast and IPTV controls invoke real work and clean up exact listeners", () => {
  const cast = read("src/components/tabs/CastingTab.tsx");
  const service = read("src/services/castingService.ts");
  const iptv = read("src/components/IPTVPlayer.tsx");
  assert.match(cast, /startCasting/);
  assert.match(cast, /await startCasting/);
  assert.match(service, /invoke<string>\("start_casting"/);
  assert.match(iptv, /removeEventListener\("timeupdate", updateTime\)/);
  assert.match(iptv, /removeEventListener\("play", handlePlay\)/);
  assert.match(iptv, /removeEventListener\("error", handleError\)/);
  assert.doesNotMatch(iptv, /removeEventListener\("[^"]+", \(\) =>/);
});
