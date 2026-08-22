# CinaVault 3.0

CinaVault 3.0 is a Windows-native media-server and desktop-management build that evolves the existing CinaVault Premium application toward the approved technical design. It retains the React/Rust/Tauri desktop client as an administration surface while introducing a separately hosted server foundation for safe library management, NAS-aware storage, and future Jellyfin-compatible services.

## Repository scope

This repository is intentionally independent from `CinaVault-Premium`. It begins from a clean source baseline with historical release artifacts excluded, so the 3.0 service, storage, and migration work can proceed without changing the original application repository.

## Current implementation sequence

1. Stabilize the inherited desktop baseline and release checks.
2. Add the Windows service and safe volume foundation.
3. Introduce non-destructive source reconciliation, UNC-first storage, and migration safeguards.
4. Add the adult-aware metadata, artwork, playback, and privacy layers in approved phases.

## Development

```bash
npm install
npm run tauri dev
```

Build a local desktop package with:

```bash
npm run tauri build
```

The server foundation is being introduced as a separate component; do not assume the Tauri desktop build is the final server deployment artifact.

## Baseline provenance

The initial source baseline was copied from the `johngraven75/CinaVault-Premium` main branch at commit `a1f1d96`, excluding generated output, historical releases, logs, and release artifacts. See `docs/CINAVAULT_3_BASELINE.md` for the migration boundary and implementation rules.
