/**
 * CinaVault Premium — Carry-Forward Verification Test
 * Build 155+
 *
 * Verifies that every feature token from every prior build is still present
 * in the active source files. Fails with a precise regression report if any
 * token is missing.
 *
 * Run: node --test tests/carryForwardVerification.test.mjs
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// ─── Feature Registry ────────────────────────────────────────────────────────
// Each entry: { build, feature, token, file }
// token: string that must appear in file (case-sensitive substring match)
// file: path relative to repo root

const FEATURE_REGISTRY = [
  // ── Builds 130–139: Foundation ──────────────────────────────────────────
  {
    build: "130-139",
    feature: "Tauri app shell",
    token: "Build 140 Futuristic Application Shell",
    file: "src/App.tsx",
  },
  {
    build: "130-139",
    feature: "SQLite database layer",
    token: "rusqlite",
    file: "src-tauri/Cargo.toml",
  },
  {
    build: "130-139",
    feature: "Media scanner",
    token: "scan_sources",
    file: "src-tauri/src/scanner.rs",
  },
  {
    build: "130-139",
    feature: "Media player commands",
    token: "play_media",
    file: "src-tauri/src/player.rs",
  },
  {
    build: "130-139",
    feature: "Download manager",
    token: "start_download",
    file: "src-tauri/src/downloads.rs",
  },
  {
    build: "130-139",
    feature: "IPTV / Live TV",
    token: "add_xtream_profile",
    file: "src-tauri/src/iptv.rs",
  },
  {
    build: "130-139",
    feature: "Jellyfin fallback server",
    token: "jellyfin",
    file: "src-tauri/src/jellyfin.rs",
  },
  {
    build: "130-139",
    feature: "VPN integration",
    token: "vpn",
    file: "src-tauri/src/vpn.rs",
  },
  {
    build: "130-139",
    feature: "Chapter detection",
    token: "generate_chapter_thumbs",
    file: "src-tauri/src/chapters.rs",
  },
  {
    build: "130-139",
    feature: "Duplicate detection",
    token: "find_duplicates",
    file: "src-tauri/src/duplicates.rs",
  },
  {
    build: "130-139",
    feature: "Task progress tracking",
    token: "TaskProgress",
    file: "src-tauri/src/task_progress.rs",
  },

  // ── Build 140: HUD Shell + Google Cast + Metadata Routing ───────────────
  {
    build: "140",
    feature: "Futuristic HUD app shell",
    token: "Build 140 Futuristic Application Shell",
    file: "src/App.tsx",
  },
  {
    build: "140",
    feature: "HUD sidebar navigation",
    token: "Build 140 Futuristic Sidebar Navigation",
    file: "src/components/Sidebar.tsx",
  },
  {
    build: "140",
    feature: "Sidebar active panel indicator",
    token: "sidebar-active-panel",
    file: "src/components/Sidebar.tsx",
  },
  {
    build: "140",
    feature: "Sidebar active rail indicator",
    token: "sidebar-active-rail",
    file: "src/components/Sidebar.tsx",
  },
  {
    build: "140",
    feature: "Google Cast service",
    token: "castToGoogleDevice",
    file: "src/services/googleCast.ts",
  },
  {
    build: "140",
    feature: "Metadata extension commands",
    token: "get_metadata_providers",
    file: "src-tauri/src/metadata_ext.rs",
  },
  {
    build: "140",
    feature: "PGMA bridge",
    token: "refresh_pgma_library",
    file: "src-tauri/src/pgma_bridge.rs",
  },
  {
    build: "140",
    feature: "Adult site provider",
    token: "PORN_SITE_NUXT_DEFAULT_BASE_URL",
    file: "src-tauri/src/adult_site_provider.rs",
  },
  {
    build: "140",
    feature: "Library artifacts",
    token: "sidecar_poster_path_for_video",
    file: "src-tauri/src/library_artifacts.rs",
  },

  // ── Build 141: Clean Installers + Cast Typings ───────────────────────────
  {
    build: "141",
    feature: "Cast type safety",
    token: "GoogleCastMedia",
    file: "src/services/googleCast.ts",
  },

  // ── Build 142: Plugin Safety + Card Sizing Fix ───────────────────────────
  {
    build: "142",
    feature: "Plugin registry",
    token: "FULL_PLUGIN_REGISTRY",
    file: "src/components/tabs/PluginsTab.tsx",
  },

  // ── Build 143: AI Media Agent + Card Clamp ───────────────────────────────
  {
    build: "143",
    feature: "AI Media Agent service",
    token: "AI_MEDIA_AGENT_ENABLED",
    file: "src/services/aiMediaAgent.ts",
  },
  {
    build: "143",
    feature: "AI Diagnostics tab",
    token: "AIDiagnosticsTab",
    file: "src/components/tabs/AIDiagnosticsTab.tsx",
  },
  {
    build: "143",
    feature: "Duplicate safe quarantine",
    token: "shouldQuarantineDuplicate",
    file: "src/services/aiMediaAgent.ts",
  },

  // ── Build 144: CinaVault Server Foundation ───────────────────────────────
  {
    build: "144",
    feature: "CinaVault proprietary server",
    token: "cinavaultServer",
    file: "src/services/serverProvider.ts",
  },
  {
    build: "144",
    feature: "Server tab UI",
    token: "ServerTab",
    file: "src/components/tabs/ServerTab.tsx",
  },

  // ── Build 145: AI Provider Fallback + Full Feature Suite ─────────────────
  {
    build: "145",
    feature: "AI provider fallback",
    token: "getSafeAiProvider",
    file: "src/services/aiProviderFallback.ts",
  },
  {
    build: "145",
    feature: "Advanced tab",
    token: "AdvancedTab",
    file: "src/components/tabs/AdvancedTab.tsx",
  },
  {
    build: "145",
    feature: "Remote access tab",
    token: "RemoteAccessTab",
    file: "src/components/tabs/RemoteAccessTab.tsx",
  },
  {
    build: "145",
    feature: "Security tab",
    token: "SecurityTab",
    file: "src/components/tabs/SecurityTab.tsx",
  },
  {
    build: "145",
    feature: "Downloads tab",
    token: "DownloadsTab",
    file: "src/components/tabs/DownloadsTab.tsx",
  },
  {
    build: "145",
    feature: "Live TV tab",
    token: "LiveTVTab",
    file: "src/components/tabs/LiveTVTab.tsx",
  },
  {
    build: "145",
    feature: "Media sources tab",
    token: "MediaSourcesTab",
    file: "src/components/tabs/MediaSourcesTab.tsx",
  },
  {
    build: "145",
    feature: "Settings tab",
    token: "SettingsTab",
    file: "src/components/tabs/SettingsTab.tsx",
  },

  // ── Build 148: Permanent Media Plugins ───────────────────────────────────
  {
    build: "148",
    feature: "Permanent media plugins",
    token: "permanentMediaPlugins",
    file: "src/plugins/permanentMediaPlugins.ts",
  },
  {
    build: "148",
    feature: "FFmpeg permanent plugin",
    token: "ffmpeg",
    file: "src/plugins/permanentMediaPlugins.ts",
  },
  {
    build: "148",
    feature: "YT-DLP permanent plugin",
    token: "yt-dlp",
    file: "src/plugins/permanentMediaPlugins.ts",
  },
  {
    build: "148",
    feature: "MediaInfo permanent plugin",
    token: "mediainfo",
    file: "src/plugins/permanentMediaPlugins.ts",
  },
  {
    build: "148",
    feature: "Startup plugin service",
    token: "initializePermanentMediaPluginsAtStartup",
    file: "src/services/startupMediaPluginService.ts",
  },
  {
    build: "148",
    feature: "Plugin installed flag",
    token: "installed: true",
    file: "src/plugins/permanentMediaPlugins.ts",
  },
  {
    build: "148",
    feature: "Plugin startup flag",
    token: "startup: true",
    file: "src/plugins/permanentMediaPlugins.ts",
  },
  {
    build: "148",
    feature: "Plugin required flag",
    token: "required: true",
    file: "src/plugins/permanentMediaPlugins.ts",
  },

  // ── Build 149: Poster/NFO Write-Back + Kodi Skin Rewrite ─────────────────
  {
    build: "149",
    feature: "Poster download to disk",
    token: "download_poster_to_sidecar",
    file: "src-tauri/src/enrichment.rs",
  },
  {
    build: "149",
    feature: "NFO sidecar write-back",
    token: "write_nfo_sidecar",
    file: "src-tauri/src/enrichment.rs",
  },
  {
    build: "149",
    feature: "Kodi skin CSS",
    token: "Kodi Skin Rewrite",
    file: "src/styles/kodi-skin.css",
  },
  {
    build: "149",
    feature: "Kodi home layout component",
    token: "KodiHomeLayout",
    file: "src/components/kodi/KodiHomeLayout.tsx",
  },
  {
    build: "149",
    feature: "Kodi theme routing in HomeTab",
    token: "KodiHomeLayout",
    file: "src/components/tabs/HomeTab.tsx",
  },

  // ── Build 150: Plugin Manager Implementation ──────────────────────────────
  {
    build: "150",
    feature: "Plugin repo management",
    token: "get_plugin_repos",
    file: "src-tauri/src/plugins.rs",
  },
  {
    build: "150",
    feature: "Plugin catalog sync",
    token: "sync_plugin_catalog",
    file: "src-tauri/src/plugins.rs",
  },
  {
    build: "150",
    feature: "Plugin install/uninstall",
    token: "install_plugin",
    file: "src-tauri/src/plugins.rs",
  },
  {
    build: "150",
    feature: "Plugin run command",
    token: "run_plugin",
    file: "src-tauri/src/plugins.rs",
  },
  {
    build: "150",
    feature: "Installed plugins list",
    token: "get_installed_plugins",
    file: "src-tauri/src/plugins.rs",
  },

  // ── Build 154: NAS Integration + Logo Branding ───────────────────────────
  {
    build: "154",
    feature: "Synology NAS integration",
    token: "synology_connect",
    file: "src-tauri/src/nas_devices.rs",
  },
  {
    build: "154",
    feature: "WD My Cloud integration",
    token: "wd_mycloud_connect",
    file: "src-tauri/src/nas_devices.rs",
  },
  {
    build: "154",
    feature: "NAS library browser",
    token: "CloudNASTab",
    file: "src/components/tabs/CloudNASTab.tsx",
  },
  {
    build: "154",
    feature: "reqwest cookies feature",
    token: "cookies",
    file: "src-tauri/Cargo.toml",
  },

  // ── Build 165: Real Work, NAS, Adult Metadata, Poster Integrity ──────────
  {
    build: "165",
    feature: "Permanent media tools auto-bootstrap at app startup",
    token: "ensure_media_tools",
    file: "src-tauri/src/media_tools.rs",
  },
  {
    build: "165",
    feature: "AI operational prompts execute library automation",
    token: "AiQueryRoute::LibraryAutomation",
    file: "src-tauri/src/ai.rs",
  },
  {
    build: "165",
    feature: "AI source discovery persists real sources",
    token: "discover_and_add_sources",
    file: "src-tauri/src/scanner.rs",
  },
  {
    build: "165",
    feature: "WD and Synology sources use scanner-compatible network paths",
    token: "network_source_path",
    file: "src-tauri/src/nas_devices.rs",
  },
  {
    build: "165",
    feature: "All adult metadata providers participate in runtime routing",
    token: "configured_adult_provider_order",
    file: "src-tauri/src/metadata.rs",
  },
  {
    build: "165",
    feature: "Poster sidecars are validated and atomically written",
    token: "write_poster_sidecar_bytes",
    file: "src-tauri/src/enrichment.rs",
  },
  {
    build: "165",
    feature: "Media cards handle sidecar poster failures",
    token: "data-poster-fallback",
    file: "src/components/tabs/HomeTab.tsx",
  },
  {
    build: "165",
    feature: "Plugin and provider JSON configuration validation",
    token: "every plugin config is valid, enabled, uniquely identified JSON",
    file: "tests/build165PluginProviderConfig.test.mjs",
  },
  {
    build: "165",
    feature: "Real-work regression coverage",
    token: "Build 165 real-work governance checks",
    file: "tests/build165RealWorkSideEffects.test.mjs",
  },

  // ── Build 155: Full Automation ────────────────────────────────────────────
  {
    build: "155",
    feature: "Automated CI/CD pipeline",
    token: "Auto Build, Test, Cleanup",
    file: ".github/workflows/windows-installer.yml",
  },
  {
    build: "155",
    feature: "Automated maintenance workflow",
    token: "Automated Daily Maintenance",
    file: ".github/workflows/maintenance.yml",
  },
  {
    build: "155",
    feature: "Automated library workflow",
    token: "Automated Library",
    file: ".github/workflows/library-maintenance.yml",
  },
  {
    build: "155",
    feature: "Post-build cleanup script",
    token: "Post-Build Cleanup",
    file: "scripts/cleanup.mjs",
  },
  {
    build: "155",
    feature: "Carry-forward governance test",
    token: "Carry-Forward Verification Test",
    file: "tests/carryForwardVerification.test.mjs",
  },
  {
    build: "2.0.6",
    feature: "Secure Hugging Face token recovery before AI status",
    token: 'invoke("ensure_hf_token")',
    file: "src/components/tabs/AIDiagnosticsTab.tsx",
  },
  {
    build: "2.0.6",
    feature: "Metadata provider initialization at every launch",
    token: "initialize_metadata_providers(&database)",
    file: "src-tauri/src/lib.rs",
  },
  {
    build: "2.0.6",
    feature: "Provider readiness persistence",
    token: "metadata_provider_startup_status",
    file: "src-tauri/src/metadata_ext.rs",
  },
  {
    build: "2.0.6",
    feature: "Kodi metadata response envelope handling",
    token: "const updated = result.updated_item",
    file: "src/components/kodi/KodiHomeLayout.tsx",
  },
  {
    build: "2.0.6",
    feature: "Kodi metadata and poster card state merge",
    token: "{ ...media, ...updated }",
    file: "src/components/kodi/KodiHomeLayout.tsx",
  },
];

// ─── Test: Every feature token must be present in its source file ────────────

test("all carry-forward feature tokens are present in source files", () => {
  const regressions = [];
  const missingFiles = [];
  const checked = [];

  for (const entry of FEATURE_REGISTRY) {
    const absPath = resolve(ROOT, entry.file);

    // Check file exists
    if (!existsSync(absPath)) {
      missingFiles.push(
        `[Build ${entry.build}] FILE MISSING: ${entry.file}  (feature: "${entry.feature}")`,
      );
      continue;
    }

    // Check token is present in file
    const content = readFileSync(absPath, "utf8");
    if (!content.includes(entry.token)) {
      regressions.push(
        `[Build ${entry.build}] TOKEN MISSING: "${entry.token}" not found in ${entry.file}  (feature: "${entry.feature}")`,
      );
    } else {
      checked.push(`[Build ${entry.build}] ✅ ${entry.feature}`);
    }
  }

  // Report all issues together for a clear regression report
  const allIssues = [...missingFiles, ...regressions];

  if (allIssues.length > 0) {
    const report = [
      "",
      "╔══════════════════════════════════════════════════════════════════╗",
      "║         CARRY-FORWARD REGRESSION DETECTED                       ║",
      "╚══════════════════════════════════════════════════════════════════╝",
      "",
      `  ${allIssues.length} feature(s) from prior builds are MISSING from the active source tree.`,
      "",
      ...allIssues.map((issue) => `  ❌ ${issue}`),
      "",
      `  ${checked.length} feature(s) verified present.`,
      "",
      "  To fix: restore the missing tokens/files and re-run this test.",
      "  See docs/CARRY_FORWARD.md for the full feature registry.",
      "",
    ].join("\n");

    assert.fail(report);
  }

  // All tokens verified
  console.log(
    `\n  ✅ Carry-forward verification passed: ${checked.length} feature tokens verified across all builds.\n`,
  );
});

// ─── Test: CARRY_FORWARD.md exists and is non-empty ─────────────────────────

test("docs/CARRY_FORWARD.md exists and contains the feature registry", () => {
  const carryForwardPath = resolve(ROOT, "docs/CARRY_FORWARD.md");
  assert.ok(
    existsSync(carryForwardPath),
    "docs/CARRY_FORWARD.md must exist — it is the authoritative feature registry.",
  );
  const content = readFileSync(carryForwardPath, "utf8");
  assert.ok(
    content.length > 500,
    "docs/CARRY_FORWARD.md appears to be empty or truncated.",
  );
  assert.ok(
    content.includes("Feature Registry"),
    "docs/CARRY_FORWARD.md must contain a 'Feature Registry' section.",
  );
  assert.ok(
    content.includes("Build 155"),
    "docs/CARRY_FORWARD.md must include Build 155 entries.",
  );
});

// ─── Test: cleanup.mjs script exists ────────────────────────────────────────

test("scripts/cleanup.mjs exists and is executable", () => {
  const cleanupPath = resolve(ROOT, "scripts/cleanup.mjs");
  assert.ok(
    existsSync(cleanupPath),
    "scripts/cleanup.mjs must exist — it is required for automated post-build cleanup.",
  );
  const content = readFileSync(cleanupPath, "utf8");
  assert.ok(
    content.includes("Post-Build Cleanup"),
    "scripts/cleanup.mjs must contain the cleanup implementation.",
  );
});

// ─── Test: All three automation workflows exist ──────────────────────────────

test("all three automation workflows exist in .github/workflows/", () => {
  const workflows = [
    ".github/workflows/windows-installer.yml",
    ".github/workflows/maintenance.yml",
    ".github/workflows/library-maintenance.yml",
  ];
  for (const wf of workflows) {
    const absPath = resolve(ROOT, wf);
    assert.ok(existsSync(absPath), `Automation workflow missing: ${wf}`);
    const content = readFileSync(absPath, "utf8");
    assert.ok(content.length > 200, `Workflow file appears empty: ${wf}`);
  }
});
