# CinaVault 3.0 Foundation Build 1 — Carry-Forward Report

## Purpose

Create an independent CinaVault 3.0 repository and establish the first safe boundary for a future Windows media-server service. The service foundation proves a loopback-only API and a non-destructive volume reconciliation contract; it is not a production server release.

## Front end

The existing React/Tauri desktop application remains the administrative-client baseline. Its package, Tauri bundle, Cargo manifest, and build identity are now branded for CinaVault 3.0. No new screens or Tauri-to-service calls were added in this build.

Validation: TypeScript strict type checking and the Vite production build pass. The inherited node regression suite passes after updating the build-identity assertion from the old v2 line to the new v3 line.

## Connector and integration



.



A new standalone local service contract is defined at `contracts/v3/cinavault-service-foundation.openapi.yaml`. The service exposes `/health`, `/Cinevault/Volumes`, and `/Cinevault/Volumes/ReconcilePlan`. It binds to `127.0.0.1:8097` by default and requires an explicit bind address to listen elsewhere.

The reconcile endpoint rejects any request with `dry_run: false` using HTTP 409. No external network listener, public firewall rule, UNC credential, Windows service registration, or remote-access feature is enabled in this foundation build.

## Back end

The new `server/cinavault-server` crate supplies typed volume routes, health, power policy, and sentinel state. Its reconcile planner returns only `ready_dry_run`, `offline`, or `aborted_unverified_volume` outcomes. Offline or unverified volumes always yield an empty change list, preventing deletion or purge plans.

Validation: `cargo fmt --check` passes. `cargo test` passes with five tests covering offline volumes, missing sentinels, verified dry-run status, health contract identity, and non-dry-run rejection.

## Completion

| Item | Result |
| --- | --- |
| Independent private repository | Created: `johngraven75/Cinavault-3.0` |
| Original CinaVault Premium repository | Not modified by this build workflow |
| Frontend strict type check | Passed |
| Frontend production build | Passed; Vite issued a non-blocking dynamic-import chunking warning |
| Inherited development gate | Passed with `releaseAuthorized: false` |
| Inherited node test suite | Passed: 25 tests |
| New service test suite | Passed: 5 tests |
| Release status | **Not authorized**; Windows service installation, WiX packaging, persistent catalogue, and production integration remain deferred |

## Next recommended build

Implement persistent local volume registration and a read-only source inspection step. Require canonical UNC/Volume GUID identity, sentinel verification, and a dry-run diff before adding any scan, catalogue, hash, or migration write path.

## Foundation Build 2 — Durable Volume Registry and Read-Only Inspection

### Purpose

The second foundation build adds durable, service-local volume registration and a shallow inspection capability while preserving the original no-write reconciliation boundary. It does not create a media catalogue, run recursive scans, mount shares, persist credentials, or modify a registered source.

### Front end

Not applicable. The desktop client remains unchanged and has not yet been wired to the new service endpoints.

### Connector and integration

The versioned loopback contract now supports `POST /Cinevault/Volumes` and `POST /Cinevault/Volumes/{id}/Inspect`. Registration accepts structured volume metadata only; inspection accepts a registered identifier rather than an arbitrary filesystem path. SMB registration requires a UNC route, and the response returns no entries for offline or unverified volumes.

### Back end

The service stores a versioned `volumes.v1.json` registry in service-local application state, using a synchronized staging file and retaining a backup of the prior readable record. Registry writes complete before in-memory state changes. Inspection samples at most 100 immediate entries, does not recurse, and does not open source files or perform catalogue writes.

### Verification

| Item | Result |
| --- | --- |
| Service formatting | Passed: `cargo fmt --check` |
| Service safety and route tests | Passed: 10 tests |
| Desktop strict type check | Passed |
| Desktop production build | Passed; existing Vite chunking warning remains non-blocking |
| Inherited development gate and desktop regression suite | Passed: 25 tests |
| Release authorization | **False**; Windows service installation and installer validation remain outstanding |

### Next recommended build

Add real Windows volume identity discovery and a health probe that records reachability without mounting a share or altering registry data. After that, design a read-only recursive inventory proposal with an explicit diff and user approval gate before any catalogue or filesystem write is introduced.


## Foundation Build 3 — Windows-Aware Volume Identity and Route Probe

### Purpose

The third foundation build adds a transient, non-mutating route probe for registered volumes. It reports route availability and a durable Windows volume identity where Windows exposes one, without changing a volume registration, library record, source route, or filesystem object.

### Front end

Not applicable. The existing desktop client is unchanged and no service-status interface has been introduced.

### Connector and integration

The loopback-only contract now supports `POST /Cinevault/Volumes/{id}/Probe`. The request accepts only a registered volume identifier. Offline and unverified volumes return no route or identity, while an unavailable route returns an explicit `unavailable` status without removing the registration.

### Back end

On Windows, the service uses the native volume API to obtain the containing volume path, volume GUID, serial number, and filesystem name for an already registered route. If Windows cannot expose a volume identity, or on non-Windows platforms, the response uses a plainly labelled path-fingerprint fallback. The result is never persisted in `volumes.v1.json`.

### Verification

| Item | Result |
|---|---|
| Service formatting | Passed: `cargo fmt --check` |
| Linux service suite | Passed: 13 tests |
| Windows source validation | Passed: `cargo check --target x86_64-pc-windows-gnu` |
| Release authorization | **False**; the probe is not a Windows Service or installer release |

### Next recommended build

Implement a scheduled, non-mutating health-probe policy only after defining administrator opt-in, backoff, offline-volume handling, and a local audit record. Do not introduce recursive inventory, catalogue writes, source repair, or remote access before a separate approval package.
