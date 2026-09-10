# CinaVault 3.0

CinaVault 3.0 is an independent media-server and desktop-management program, separate from CinaVault Premium. The Windows edition remains the reference implementation while Ubuntu Linux, Android, and iOS editions live in their own OS-specific repositories and share the same v3 product identity, parity contract, and release gate.

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
