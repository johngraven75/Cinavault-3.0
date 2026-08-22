# Build 165 Carry-Forward Audit

## Scope and rule

Build 164 (`v1.6.4`, commit `cd9a357d561ee1d7df99f9bbdcedc53838c08e53`) was audited against the 20 most recent published GitHub releases visible on 2026-07-16. No accepted feature is removed in Build 165. A feature may be retired only after explicit owner direction and a release-note entry.

## Published-release baseline

| # | Published release | Tag | Commit |
|---:|---|---|---|
| 1 | Build 164 | v1.6.4 | cd9a357 |
| 2 | Build 161 | v1.6.1 | 93ebcd6 |
| 3 | Build 160 | v1.6.0 | 5220c52 |
| 4 | Build 157 Windows | v1.0.157-windows | 84edbd9 |
| 5 | Build 253 | build-253 | d798b74 |
| 6 | Build 252 | build-252 | d798b74 |
| 7 | Build 251 | build-251 | d798b74 |
| 8 | Build 250 | build-250 | d798b74 |
| 9 | Build 249 | build-249 | d798b74 |
| 10 | Build 248 | build-248 | d798b74 |
| 11 | Build 247 | build-247 | d798b74 |
| 12 | Build 246 | build-246 | d798b74 |
| 13 | Build 245 | build-245 | d798b74 |
| 14 | Build 244 | build-244 | d798b74 |
| 15 | Build 243 | build-243 | d798b74 |
| 16 | Build 242 | build-242 | d798b74 |
| 17 | Build 241 | build-241 | d26ae41 |
| 18 | Build 154 | build-237 | c416906 |
| 19 | Build 236 | build-236 | 70f8be0 |
| 20 | v1.0.157 | v1.0.157 | b63a20b |

Several numbered releases point to the same source commit. The audit compared every distinct baseline commit as well as the release names/tags and the existing master carry-forward registry.

## Regressions found and restored

| Area | Build 164 failure | Build 165 restoration | Verification |
|---|---|---|---|
| AI metadata/title management | Operational prompts could be classified as diagnostics or inference and return text without mutating the library. | Operational prompts route to `ai_library_manage`, including metadata enrichment, title normalization, duplicate analysis, and poster/NFO work. | Rust routing test plus JavaScript command-routing governance. |
| AI source discovery | The UI only emitted a success-looking status message. | Native discovery walks platform media roots, detects real media directories, and inserts new enabled sources in SQLite. | Rust test creates media directories and verifies persisted database rows. |
| WD My Cloud | API login and source addition did not create a scanner-compatible path. | One authenticated cookie/token session is reused, SMB credentials are established, and reachable UNC sources are persisted. | Unit/config tests verify credential encoding, session reuse, UNC construction, and reachability gate. |
| Synology QuickConnect | DSM shares were represented as API paths rather than mounted scanner sources. | QuickConnect resolves and authenticates, SMB sessions are established, and reachable UNC shares are persisted. | Unit/config tests verify QuickConnect/DSM routing and UNC source construction. |
| Adult metadata | Scan routing skipped restored providers or stopped after one unavailable provider. | Adult-only provider order now includes TPDB, StashDB, Porn Site Nuxt, IAFD, PhoenixAdult, and PGMA without overwriting curated fields. | Rust provider-chain and fill-only update tests; JSON startup/config validation. |
| Poster sidecars | Downloads could accept invalid/HTML payloads, write non-atomically, and leave remote URLs on cards. | Poster bytes are size/type/signature validated, written through a temporary file and atomic rename, migrated to local sidecars, persisted, and rendered with card fallbacks. | Rust filesystem side-effect test and UI/runtime governance tests. |
| Plugins/provider configs | JSON validity and startup enablement were not comprehensively gated. | Every config is parsed, uniquely keyed, enabled, and checked for usable URLs; adult providers are checked in both startup defaults and runtime routing. | `build165PluginProviderConfig.test.mjs`. |
| Release safety | Pull-request workflows could publish a release. | PRs validate and build artifacts but release publication is limited to the main branch. | Build-number drift and workflow governance tests. |

## Carry-forward disposition

All features in `docs/CARRY_FORWARD.md` remain registered. Build 165 adds stronger behavioral checks on top of token presence. It does not remove the legacy release entry point; that entry point delegates to the current verified installer workflow.

## Release acceptance gates

Build 165 is accepted only when all of the following are green:

1. TypeScript validation and production frontend build.
2. All Node governance and real-work tests.
3. All locked Rust tests, including database/filesystem side effects.
4. Tauri Windows MSI and NSIS packaging.
5. Artifact staging with `SHA256SUMS.txt`.
6. Main-branch publication of GitHub release `v1.6.5` with MSI, EXE, and checksums.

## Attached code-review disposition

| Finding | Build 165 disposition | Proof gate |
| --- | --- | --- |
| Full catalog installed at startup | Startup scope is permanent media tools plus PGMA only. Catalog entries remain available for user selection. | `build165RealWorkSideEffects.test.mjs` |
| Settings saved but not restored / incomplete autosave | Native settings load before initialization; all persistent store slices trigger autosave with local backup. | Frontend build and governance test |
| Cast button not wired | Cast control calls the native Google Cast bridge and exposes actual pending/error state. | Frontend build and governance test |
| Plugin failures shown as success | Local state changes only after native success; manifests and registry are persisted; unsupported execution returns an error. | Rust plugin tests and governance test |
| Stale feature toggles | Desired value is calculated before the call and committed locally only after backend success. | Frontend build |
| IPTV listener cleanup | Named callbacks are reused for registration and removal. | Governance test |
| Cloud operations claimed success on failure | Provider keys are normalized and every connect/sync/browse/disconnect path reports native errors without applying success state. | Frontend build and governance test |
| Theme reinitialized plugins | Plugin initialization is startup-only; theme application is a separate effect. | Governance test |
| PGMA uninstall silently re-enabled | UI and native runtime both reject removal with an explicit error. | Governance test |
| Version label drift | Visible settings label is v1.6.5 / Build 165. | Version drift tests |
| Poisoned mutex panics | Plugin I/O locks return structured errors; no mutex `unwrap` remains. | Rust tests |
