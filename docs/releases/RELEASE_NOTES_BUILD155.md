# CinaVault Premium — Build 155 Release Notes

## Summary
Build 155 introduces **full end-to-end automation** for every function in the repository: CI/CD, Windows installer publishing, carry-forward governance, dependency maintenance, security scanning, library enrichment scheduling, NAS sync, plugin catalog refresh, AI diagnostics, stale branch cleanup, and self-healing workflows. No manual intervention is required for any of these functions going forward.

## What's New

### Automation — CI/CD Pipeline (windows-installer.yml)
- **Validate job**: TypeScript type-check + `cargo check` on every push and PR.
- **Test job**: Full governance test suite including carry-forward verification.
- **Build job**: Tauri Windows installer (NSIS `.exe` + WiX `.msi`) built on `windows-latest`.
- **Release job**: Automatically creates a tagged GitHub Release with installers attached.
- **On-failure job**: Posts a failure report as a GitHub issue with logs attached.

### Automation — Maintenance Workflow (maintenance.yml)
- **Scheduled daily**: Runs `npm audit fix`, `cargo update`, stale branch cleanup, and dependency drift detection.
- **Dependabot alerts**: Auto-creates PRs for security patches.
- **Self-healing**: If `cargo check` or `tsc` fails on main, opens a GitHub issue with the error log automatically.

### Automation — Library & Metadata Workflow (library-maintenance.yml)
- **Scheduled weekly**: Triggers library enrichment, poster download, NFO write-back, and duplicate scan via Tauri CLI.
- **NAS sync**: Scheduled Synology QuickConnect and WD My Cloud library sync.
- **Plugin catalog refresh**: Syncs plugin catalog from all registered repos.
- **AI diagnostics report**: Generates and commits an AI health report to `docs/ai-diagnostics/`.

### Automation — Carry-Forward Governance (carryForwardVerification.test.mjs)
- Runs on every CI build.
- Verifies every feature token from every build (130–155) is still present in the active source files.
- Fails the build with a precise regression report if any token is missing.
- Generates `docs/CARRY_FORWARD_REPORT_{BUILD}.md` automatically on each successful run.

### Package.json Scripts
- `test:all` — runs the full test suite (all 20+ governance tests).
- `test:carry-forward` — runs only the carry-forward verification test.
- `test:governance` — runs only the build governance surface tests.

## Preserved Features (Carry-Forward Verified)
All features from Builds 130–154 are verified present. See `docs/CARRY_FORWARD.md` for the full registry.

## Downloads
- Windows EXE installer (NSIS)
- Windows MSI installer (WiX)
- Build/test results log

## Validation
- TypeScript build: passed
- Rust cargo check: passed
- All 20+ governance tests: passed
- Carry-forward verification: passed
- Installer build: completed
