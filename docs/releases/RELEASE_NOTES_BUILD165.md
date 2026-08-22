# CinaVault Premium Build 165

Build 165 restores real, observable work to AI media-management commands and strengthens the permanent carry-forward contract.

## Restored and improved

- FFmpeg, FFprobe, yt-dlp, MediaInfo, and MKVToolNix are checked and silently installed/loaded at application startup without an in-app authorization step.
- AI metadata enrichment and title cleanup now invoke native library automation and report actual changed/error counts.
- AI source discovery now finds real media directories and persists enabled SQLite sources.
- WD My Cloud username/password authentication reuses a live session and creates reachable scanner-compatible shares.
- Synology QuickConnect resolves, authenticates, mounts, and persists reachable shares as sources.
- Adult metadata startup/runtime routing covers TPDB, StashDB, Porn Site Nuxt, IAFD, PhoenixAdult, and PGMA.
- Provider and plugin JSON files are CI-validated for syntax, identity, enablement, uniqueness, and usable endpoints.
- Poster acquisition validates image payloads, writes sidecars atomically, persists local paths, and handles card rendering failures.
- Regression tests verify actual database and filesystem effects, not success strings.
- Installer release publication is main-branch-only and produces MSI, NSIS EXE, and SHA-256 checksums.

No prior feature was intentionally removed. See `docs/BUILD_165_CARRY_FORWARD_AUDIT.md` for the release-by-release audit and acceptance gates.

## Reliability audit follow-up

- Backend settings now restore before use, every persisted settings slice autosaves, and theme changes no longer reinitialize the plugin engine.
- Startup plugin validation is limited to permanent media tools plus the required PGMA bridge; the full catalog remains available but is not silently installed.
- Plugin install/configure/enable state is persisted to real manifests, backend failures propagate to the UI, PGMA cannot be removed, and unsupported plugin execution cannot report fake success.
- Cloud connect, sync, browse, and disconnect operations now retain error state and only report success after the native operation succeeds.
- Google Drive UI identifiers are normalized to the native `googledrive` provider key.
- Google Cast now invokes the native casting command, IPTV listeners are removed with their original callbacks, and feature toggles update locally only after backend success.
- Adult provider defaults are enabled in the frontend as well as the native provider chain.
- Settings now display v1.6.5 / Build 165 consistently.
