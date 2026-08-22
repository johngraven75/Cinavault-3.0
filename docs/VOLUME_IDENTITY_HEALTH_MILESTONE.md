# CinaVault 3.0 Volume Identity and Health-Probe Milestone

## Purpose

Add a read-only probe that reports whether a registered volume’s preferred route is reachable and, where the operating system supports it, reports a durable volume identity. The probe must not mount a share, modify the local registry, update a library record, trigger scanning, or change filesystem metadata.

## User outcome

An administrator can ask the local service to probe one registered volume. The response identifies the selected route, reports its reachability, and provides a Windows volume GUID, serial number, and filesystem information when available. On non-Windows hosts or unsupported routes, the response uses a clearly labelled degraded path-fingerprint identity rather than inventing a Windows identifier.

## Front end

Not applicable. The Tauri client remains unchanged. The local HTTP contract is the only integration surface in this milestone.

## Connector and integration

The service adds `POST /Cinevault/Volumes/{id}/Probe`. It accepts only a registered volume identifier, not a caller-supplied path. The route uses the same offline and sentinel gates as source inspection, so it does not contact a source that is already known offline or unverified.

The endpoint returns a transient response only. It does not alter the durable registry, modify `Volume.health`, persist timestamps, create mounts, obtain credentials, or publish an external network endpoint.

## Back end

On Windows, the probe calls the native volume APIs against an already registered and selected route to obtain the containing volume path, volume GUID, serial number, and filesystem name. The implementation falls back safely when a route does not expose a Windows volume identity, such as an unavailable share.

On non-Windows systems, the probe returns an explicitly degraded path-fingerprint identity for a readable local route. This preserves testability without misrepresenting a non-Windows path as a Windows volume GUID.

## Safety invariants

1. Offline and unverified volumes return no route probe and no identity value.
2. A probe can target only a registered volume identifier.
3. The probe performs a bounded readability check only; it does not enumerate the source directory or inspect media files.
4. Returned identity data is transient. It is not written into `volumes.v1.json` in this milestone.
5. Any OS discovery failure returns a degraded or unavailable response rather than changing a route, sentinel, or registry record.

## Completion criteria

- A verified local volume reports a `ready` probe result and a non-empty identity value.
- An offline or unverified volume returns no identity and does not access its route.
- A missing selected route returns `unavailable` without removing the registration.
- The probe endpoint returns 404 for unknown volume identifiers.
- Service tests cover each safety state and the existing desktop verification suite remains green.

## Deferred work

Binding a discovered identity to a registered volume, Volume GUID migration, periodic health scheduling, SMB/NFS authentication, remote-NAS reachability diagnostics, recursive inventory, catalogue writes, Windows Service Control Manager installation, and installer packaging remain deferred.
