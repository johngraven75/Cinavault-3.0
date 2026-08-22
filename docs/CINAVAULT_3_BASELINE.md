# CinaVault 3.0 Independent Baseline

## Purpose

CinaVault 3.0 is a separate codebase for implementing the approved Windows service and safe media-library foundations. Its purpose is to evolve the existing desktop media-management experience without changing the `CinaVault-Premium` repository.

## Source boundary

The initial source baseline was copied from `johngraven75/CinaVault-Premium` branch `main` at commit `a1f1d96` on August 21, 2026. The following generated or historical materials were intentionally excluded:

- Git history and remote configuration.
- Generated dependency and build directories.
- Historical releases, release artifacts, release triggers, and build-trigger directories.
- Logs, test-result output, periodic diagnostics, cleanup reports, and promotional media.

This repository begins with a clean independent history. The original project is unchanged by this setup.

## Architectural direction

CinaVault 3.0 retains the Tauri desktop application as a local administration and future tray/client surface. The planned media-server core will be introduced as a separately hosted Windows service with a documented local API/IPC boundary. The desktop application must not be relied upon as the final long-lived server host.

## Implementation rules

1. Do not migrate or delete a user library without a verified backup, dry-run report, explicit user confirmation, and rollback path.
2. Treat UNC paths and durable volume identity as the storage foundation. Mapped drives are a convenience layer, not canonical service storage.
3. Keep catalogue, artwork cache, database, logs, and transcode scratch on local storage; do not place application state on NAS shares.
4. Treat missing or unverifiable volumes as offline, not deleted.
5. Preserve existing API and library data where possible; add new server capability behind explicit contracts.
6. Complete existing release and verification gates before promoting a 3.0 release.

## First implementation milestone

The first milestone establishes a service-boundary scaffold, a safe volume-domain model, a non-destructive reconcile planner, and tests for storage safety. It does not yet claim full Jellyfin compatibility, adult metadata completion, or production Windows service installation.
