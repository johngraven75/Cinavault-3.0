# CinaVault 3.0 Service Foundation Milestone

## Purpose

Establish a separately runnable local server foundation and a safe volume/reconcile domain model. This milestone creates the boundary needed to move long-lived media-server behavior out of the interactive Tauri desktop process.

## User outcome

A future desktop client can discover a local CinaVault service, inspect known volumes, and request a **dry-run** reconciliation plan. A disconnected or unverified volume must never produce a deletion instruction.

## Scope

### Back end

- Add a standalone `cinavault-server` Rust binary with a loopback-only HTTP listener by default.
- Add a versioned `/health` endpoint and additive `/Cinevault/*` endpoints.
- Add `Volume`, `VolumeRoute`, `VolumeKind`, `PowerPolicy`, and `VolumeHealth` domain types.
- Add a deterministic reconcile planner that returns `Proceed`, `Offline`, or `AbortedUnverifiedVolume` outcomes.
- Make reconcile dry-run the API default. The foundation does not modify any library, media path, or desktop SQLite data.

### Connector / integration

- Define the first local API contract in an OpenAPI document.
- Bind only to `127.0.0.1` unless an explicit bind address is supplied when the service starts.
- Do not expose a public firewall rule, auto-start service, credentials, or remote access in this milestone.

### Front end

- Not applicable in this milestone. The existing Tauri client remains unchanged until a later service-status integration package.

## Safety invariants

1. A volume with a missing or mismatched sentinel is unverified; its reconciliation aborts before producing destructive actions.
2. An offline volume never yields a deletion or purge action.
3. The initial planner uses only in-memory request data; it does not walk folders, touch NAS shares, or mutate a catalogue.
4. The service binds to loopback by default and includes no authentication bypass for a remote caller.

## Completion criteria

- `cargo test` passes for the server crate.
- The health endpoint reports a stable service identity and semantic version.
- The volumes endpoint returns a typed empty list when no volumes are registered.
- A dry-run plan with an unverified sentinel reports `aborted_unverified_volume` and no changes.
- A dry-run plan for an offline volume reports `offline` and no changes.
- The OpenAPI contract matches the implemented routes and outcome types.

## Deferred work

Windows SCM installation, WiX packaging, Credential Manager, UNC canonicalisation, persistent catalogue storage, file hashing, actual directory scanning, Jellyfin REST compatibility, Tauri UI wiring, and production authentication are deliberately deferred. They require dedicated design and validation packages after this safe boundary is established.
