# CinaVault 3.0 Volume Registry and Read-Only Inspection Milestone

## Purpose

Add durable local registration for media volumes and a strictly read-only, shallow source-inspection endpoint. This milestone makes the service aware of approved source identities without allowing it to scan recursively, modify a catalogue, alter a source, or remove a record when a volume disappears.

## User outcome

An administrator can register a named volume with one or more prioritized source routes, restart the local service, and see that registered volume again. The administrator can request a small directory sample to confirm a registered and verified volume is readable. A disconnected or unverified volume is reported safely rather than treated as missing content.

## Front end

Not applicable. The existing Tauri desktop client is not wired to the service in this milestone. The HTTP contract is the integration boundary for a future desktop status screen.

## Connector and integration

The service adds these local-only routes:

| Route | Behavior |
|---|---|
| `POST /Cinevault/Volumes` | Persist a validated volume registration to the service-local registry. |
| `GET /Cinevault/Volumes` | Return registered volumes in deterministic order. |
| `POST /Cinevault/Volumes/{id}/Inspect` | Produce a bounded, single-directory, read-only inspection sample. |

The registry location is local application state. It must never be written inside a media share or NAS root. The service still binds to loopback by default, and no Windows firewall, remote access, credentials, or share-mount automation is introduced.

## Back end

The service stores a versioned JSON registry with a backup file. It writes a synchronized staging file before replacing the primary file and retains the previous readable registry for recovery. It persists only metadata required to identify a volume and its routes; it does not persist source credentials, content indexes, hashes, artworks, or catalogue data.

A registration is accepted only when its volume identifier and label are non-empty, routes are unique and prioritized, and SMB sources provide UNC-style routes. A duplicate identifier is rejected rather than overwriting an existing registration.

Inspection is bounded to at most 100 immediate entries. It never recurses into child directories, opens media files, computes hashes, launches a scanner, mounts a share, changes file timestamps, or writes to a source route.

## Safety invariants

1. An offline volume yields `offline` with no inspection sample and no deletion proposal.
2. A missing, mismatched, or unchecked sentinel yields `unverified` with no inspection sample.
3. An unavailable route yields `unavailable` and does not delete the volume registration.
4. An inspection request may reference only a registered volume identifier; arbitrary filesystem paths are not accepted by the API.
5. Reconcile remains dry-run only and is not connected to the inspection endpoint.

## Completion criteria

- A successfully registered volume is visible after creating a new service state from the same registry file.
- Duplicate registration returns a conflict without changing the persisted registry.
- A verified local test volume produces a sorted, bounded immediate-entry sample.
- Offline and unverified volumes return no sampled entries.
- The API contract documents all new routes and the service tests pass without a live background process.

## Deferred work

Network reachability probes, volume GUID discovery, SMB/NFS credential management, recursive media enumeration, persisted catalogue records, hash calculation, source reconciliation writes, Windows Service Control Manager registration, installer packaging, and Tauri UI integration remain out of scope.
