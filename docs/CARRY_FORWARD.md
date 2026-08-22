# CinaVault Premium — Master Carry-Forward Registry

> **Rule:** No accepted feature from any prior build may be removed or hidden without explicit owner approval and a documented entry in the release notes for that build. This file is the authoritative source of truth and is verified automatically on every CI run.

---

## How This Works

On every push and pull request, the CI pipeline runs `tests/carryForwardVerification.test.mjs`, which checks that every feature token listed in the **Feature Registry** below is still present in the active source files. If any token is missing, the build fails with a clear report of what regressed.

Each new build must:
1. Add its features to the **Feature Registry** below.
2. Write a `RELEASE_NOTES_BUILD{N}.md` file in the repo root.
3. Pass all carry-forward governance tests before the Windows installer is built.

---

## Feature Registry

### Build 130–139 (Foundation)

| Feature | Token to Verify | Source File |
|---|---|---|
| Tauri app shell | `App.tsx` | `src/App.tsx` |
| SQLite database layer | `rusqlite` | `src-tauri/Cargo.toml` |
| Media scanner | `scan_library` | `src-tauri/src/scanner.rs` |
| Media player commands | `play_media` | `src-tauri/src/player.rs` |
| Download manager | `start_download` | `src-tauri/src/downloads.rs` |
| IPTV / Live TV | `get_iptv_channels` | `src-tauri/src/iptv.rs` |
| Jellyfin fallback server | `jellyfin` | `src-tauri/src/jellyfin.rs` |
| VPN integration | `vpn` | `src-tauri/src/vpn.rs` |
| Chapter detection | `detect_chapters` | `src-tauri/src/chapters.rs` |
| Duplicate detection | `find_duplicates` | `src-tauri/src/duplicates.rs` |
| Task progress tracking | `TaskProgress` | `src-tauri/src/task_progress.rs` |

### Build 140 (HUD Shell + Google Cast + Metadata Routing)

| Feature | Token to Verify | Source File |
|---|---|---|
| Futuristic HUD app shell | `Build 140 Futuristic Application Shell` | `src/App.tsx` |
| HUD sidebar navigation | `Build 140 Futuristic Sidebar Navigation` | `src/components/Sidebar.tsx` |
| Sidebar active panel indicator | `sidebar-active-panel` | `src/components/Sidebar.tsx` |
| Sidebar active rail indicator | `sidebar-active-rail` | `src/components/Sidebar.tsx` |
| Google Cast service | `googleCast` | `src/services/googleCast.ts` |
| Metadata extension commands | `metadata_ext` | `src-tauri/src/metadata_ext.rs` |
| PGMA bridge | `pgma_bridge` | `src-tauri/src/pgma_bridge.rs` |
| Adult site provider | `adult_site_provider` | `src-tauri/src/adult_site_provider.rs` |
| Library artifacts | `library_artifacts` | `src-tauri/src/library_artifacts.rs` |

### Build 141 (Clean Installers + Cast Typings)

| Feature | Token to Verify | Source File |
|---|---|---|
| Cast type safety | `CastSession` | `src/services/googleCast.ts` |

### Build 142 (Plugin Safety + Card Sizing Fix)

| Feature | Token to Verify | Source File |
|---|---|---|
| Plugin catalog-only safety | `catalog_only` | `src/components/tabs/PluginsTab.tsx` |
| Multi-item poster row sizing | `poster-card` | `src/styles/media-row-poster-final-fix.css` |

### Build 143 (AI Media Agent + Card Clamp)

| Feature | Token to Verify | Source File |
|---|---|---|
| AI Media Agent service | `aiMediaAgent` | `src/services/aiMediaAgent.ts` |
| AI Diagnostics tab | `AIDiagnosticsTab` | `src/components/tabs/AIDiagnosticsTab.tsx` |
| Media card hard clamp | `media-card-hard-fix` | `src/styles/media-card-hard-fix.css` |
| Duplicate safe quarantine | `quarantine` | `src-tauri/src/duplicates.rs` |

### Build 144 (CinaVault Server Foundation)

| Feature | Token to Verify | Source File |
|---|---|---|
| CinaVault proprietary server | `cinavaultServer` | `src/services/serverProvider.ts` |
| Server tab UI | `ServerTab` | `src/components/tabs/ServerTab.tsx` |

### Build 145 (AI Provider Fallback + Full Feature Suite)

| Feature | Token to Verify | Source File |
|---|---|---|
| AI provider fallback | `aiProviderFallback` | `src/services/aiProviderFallback.ts` |
| Advanced tab | `AdvancedTab` | `src/components/tabs/AdvancedTab.tsx` |
| Remote access tab | `RemoteAccessTab` | `src/components/tabs/RemoteAccessTab.tsx` |
| Security tab | `SecurityTab` | `src/components/tabs/SecurityTab.tsx` |
| Downloads tab | `DownloadsTab` | `src/components/tabs/DownloadsTab.tsx` |
| Live TV tab | `LiveTVTab` | `src/components/tabs/LiveTVTab.tsx` |
| Media sources tab | `MediaSourcesTab` | `src/components/tabs/MediaSourcesTab.tsx` |
| Settings tab | `SettingsTab` | `src/components/tabs/SettingsTab.tsx` |

### Build 147 (Clean Media Rows)

| Feature | Token to Verify | Source File |
|---|---|---|
| Media row poster fix | `media-row-poster-final-fix` | `src/styles/media-row-poster-final-fix.css` |

### Build 148 (Permanent Media Plugins)

| Feature | Token to Verify | Source File |
|---|---|---|
| Permanent media plugins | `permanentMediaPlugins` | `src/plugins/permanentMediaPlugins.ts` |
| FFmpeg permanent plugin | `ffmpeg` | `src/plugins/permanentMediaPlugins.ts` |
| YT-DLP permanent plugin | `yt-dlp` | `src/plugins/permanentMediaPlugins.ts` |
| MediaInfo permanent plugin | `mediainfo` | `src/plugins/permanentMediaPlugins.ts` |
| Startup plugin service | `startupMediaPluginService` | `src/services/startupMediaPluginService.ts` |
| Plugin installed flag | `installed: true` | `src/plugins/permanentMediaPlugins.ts` |
| Plugin startup flag | `startup: true` | `src/plugins/permanentMediaPlugins.ts` |
| Plugin required flag | `required: true` | `src/plugins/permanentMediaPlugins.ts` |

### Build 149 (Poster/NFO Write-Back + Kodi Skin Rewrite)

| Feature | Token to Verify | Source File |
|---|---|---|
| Poster download to disk | `download_poster_to_sidecar` | `src-tauri/src/enrichment.rs` |
| NFO sidecar write-back | `write_nfo_sidecar` | `src-tauri/src/enrichment.rs` |
| Kodi skin CSS | `kodi-skin` | `src/styles/kodi-skin.css` |
| Kodi home layout component | `KodiHomeLayout` | `src/components/kodi/KodiHomeLayout.tsx` |
| Kodi theme routing in HomeTab | `KodiHomeLayout` | `src/components/tabs/HomeTab.tsx` |

### Build 150 (Plugin Manager Implementation)

| Feature | Token to Verify | Source File |
|---|---|---|
| Plugin repo management | `get_plugin_repos` | `src-tauri/src/plugins.rs` |
| Plugin catalog sync | `sync_plugin_catalog` | `src-tauri/src/plugins.rs` |
| Plugin install/uninstall | `install_plugin` | `src-tauri/src/plugins.rs` |
| Plugin run command | `run_plugin` | `src-tauri/src/plugins.rs` |
| Installed plugins list | `get_installed_plugins` | `src-tauri/src/plugins.rs` |

### Build 154 (NAS Integration + Logo Branding)

| Feature | Token to Verify | Source File |
|---|---|---|
| Synology NAS integration | `synology_connect` | `src-tauri/src/nas_devices.rs` |
| WD My Cloud integration | `wd_mycloud_connect` | `src-tauri/src/nas_devices.rs` |
| NAS library browser | `CloudNASTab` | `src/components/tabs/CloudNASTab.tsx` |
| CinaVault logo branding | `cinavault-logo.png` | `public/branding/cinavault-logo.png` |
| reqwest cookies feature | `cookies` | `src-tauri/Cargo.toml` |

### Build 155 (Full Automation)

| Feature | Token to Verify | Source File |
|---|---|---|
| Automated CI/CD pipeline | `windows-installer.yml` | `.github/workflows/windows-installer.yml` |
| Automated maintenance workflow | `maintenance.yml` | `.github/workflows/maintenance.yml` |
| Automated library enrichment workflow | `library-maintenance.yml` | `.github/workflows/library-maintenance.yml` |
| Carry-forward governance test | `carryForwardVerification` | `tests/carryForwardVerification.test.mjs` |
| Full test suite npm script | `test:all` | `package.json` |

---

### Build 165 (Real Work + NAS + Metadata + Poster Integrity)

| Feature | Token to Verify | Source File |
|---|---|---|
| Automatic FFmpeg/download tool bootstrap | `ensure_media_tools` | `src-tauri/src/media_tools.rs` |
| Operational AI routing | `AiQueryRoute::LibraryAutomation` | `src-tauri/src/ai.rs` |
| Real media-source discovery | `discover_and_add_sources` | `src-tauri/src/scanner.rs` |
| Scanner-compatible WD/Synology sources | `network_source_path` | `src-tauri/src/nas_devices.rs` |
| Complete adult provider routing | `configured_adult_provider_order` | `src-tauri/src/metadata.rs` |
| Validated atomic poster sidecars | `write_poster_sidecar_bytes` | `src-tauri/src/enrichment.rs` |
| Media-card poster fallback | `data-poster-fallback` | `src/components/tabs/HomeTab.tsx` |
| Provider/plugin JSON validation | `build165PluginProviderConfig` | `tests/build165PluginProviderConfig.test.mjs` |
| Real side-effect regression tests | `build165RealWorkSideEffects` | `tests/build165RealWorkSideEffects.test.mjs` |

## Build History Summary

| Build | Key Features Added | Carry-Forward Status |
|---|---|---|
| 130–139 | Foundation: scanner, player, downloads, IPTV, Jellyfin, VPN, chapters, duplicates | ✅ Verified |
| 140 | HUD shell, Google Cast, metadata routing, PGMA/adult providers | ✅ Verified |
| 141 | Clean installers, Cast type safety | ✅ Verified |
| 142 | Plugin safety, poster card sizing fix | ✅ Verified |
| 143 | AI Media Agent, card clamp, safe duplicate quarantine | ✅ Verified |
| 144 | CinaVault proprietary server foundation | ✅ Verified |
| 145 | AI provider fallback, full feature suite (all tabs) | ✅ Verified |
| 147 | Clean media rows, poster sizing | ✅ Verified |
| 148 | Permanent media plugins (FFmpeg, YT-DLP, MediaInfo) | ✅ Verified |
| 149 | Poster/NFO write-back, Kodi skin rewrite | ✅ Verified |
| 150 | Plugin manager fully implemented (8 Tauri commands) | ✅ Verified |
| 151–153 | (No release notes on file — features carried forward from 150) | ⚠️ No release notes |
| 154 | Synology + WD My Cloud NAS integration, CinaVault logo branding | ✅ Verified |
| 155 | Full automation: CI/CD, maintenance, library, carry-forward governance | ✅ Verified |
| 156–164 | Features preserved; Build 164 audited against the 20 most recent published releases | ✅ Verified by Build 165 audit |
| 165 | Real AI work, source discovery, WD/Synology scanning, adult providers, poster integrity | ✅ Current |

---

### Build 166 (Persistent Hugging Face Model Selection)

- Selected `Qwen/Qwen3-4B-Instruct-2507` after a successful authenticated structured-output inference test.
- Restores a valid credential from the standard Hugging Face CLI cache when the application database is empty after reinstall.
- Migrates only the former Mistral default, preserving explicit user model choices.
- Carries forward all Build 165 AI, NAS, metadata-provider, poster-sidecar, media-tool, cloud, casting, and plugin behavior.
- Publishes both MSI and NSIS EXE installers with SHA-256 checksums.

*Last updated: Build 166 — verified by CI carry-forward governance, live Hugging Face inference, and installer tests.*


### v2.0.6 (Persistent AI, Metadata Providers, and Media Cards)

| Feature | Token to Verify | Source File |
|---|---|---|
| Secure Hugging Face token recovery before AI status | `ensure_hf_token` | `src/components/tabs/AIDiagnosticsTab.tsx` |
| Metadata provider initialization at every launch | `initialize_metadata_providers(&database)` | `src-tauri/src/lib.rs` |
| Provider readiness and environment-key import | `metadata_provider_startup_status` | `src-tauri/src/metadata_ext.rs` |
| Kodi media-card metadata response contract | `const updated = result.updated_item` | `src/components/kodi/KodiHomeLayout.tsx` |
| Metadata/poster card state merge | `{ ...media, ...updated }` | `src/components/kodi/KodiHomeLayout.tsx` |

> These v2.0.6 contracts are permanent carry-forward requirements. Removal requires the owner's explicit instruction and a documented release-note removal entry.
