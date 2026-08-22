# Build 170 implementation status

Build 170 is blocked from release until every item below is complete and verified.

## Required fixes

- [ ] Register `source_health` in the Tauri command handler.
- [ ] Validate source paths before saving and before scanning.
- [ ] Support single-file sources without treating them as directories.
- [ ] Display per-source scan failures in `MediaSourcesTab`.
- [ ] Treat placeholder metadata providers as unavailable/not implemented rather than successful.
- [ ] Verify NFO sidecars after atomic write and report per-file failures.
- [ ] Replace Windscribe/winget calls with a bundled WireGuard-compatible agent.
- [ ] Add WireGuard profile import, profile validation, connect/disconnect/status, and cleanup.
- [ ] Package the agent with MSI and NSIS installers.
- [ ] Run TypeScript tests, Rust tests, production frontend build, and Tauri installer build.
- [ ] Verify MSI, EXE, checksums, clean install, source scan, metadata sidecars, provider diagnostics, VPN profile lifecycle, and uninstall cleanup.

## Release gate

Do not create or publish tag `v1.7.0` until all boxes are checked and CI is green. A release workflow must fail closed if either installer is absent or any verification step fails.
