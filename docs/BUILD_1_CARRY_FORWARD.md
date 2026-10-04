# CinaVault 3.0 v3 Build 1 — Carry-Forward Report

## Purpose

Build and publish the Windows reference installer for the CinaVault 3.0 foundation line while carrying forward the repository’s existing service-safety boundary and validation history.

## Front end

The React/Tauri desktop client remains the Windows reference implementation. No user-interface feature was introduced by this release operation. The release workflow validates TypeScript, runs the carry-forward regression suite, and performs the Vite production build before packaging.

## Connector / integration

The Windows release uses the existing Tauri bundle configuration and GitHub Actions publication path. The workflow prepares the required Windows resources, builds the MSI and NSIS installers on `windows-latest`, stages only generated installer artifacts, and publishes them to the immutable `v3-build-1` GitHub Release. No credentials, private keys, or user media data are included.

## Back end

The existing CinaVault 3.0 service foundation is carried forward unchanged. Its loopback-only API, non-destructive volume reconciliation, and Windows-aware route-probe boundaries remain in effect. This installer release does not claim production Windows Service registration, remote access, recursive inventory, catalogue writes, or source repair.

## Verification and release status

The release workflow performs the supported regression, static, frontend, Rust, resource-preparation, packaging, and artifact-presence checks. It requires both a Windows MSI and an NSIS setup EXE before upload and publication. The Windows master-gated workflow additionally defines install, launch, and uninstall acceptance for future runs that use that stricter gate.

The release is authorized for the desktop/reference installer scope represented by `v3-build-1`. Deferred foundation items remain explicitly deferred rather than being represented as complete: production Windows Service integration, persistent catalogue behavior, remote access, and broader cross-platform edition parity.

## Carry-forward rule

Future builds must retain this report's scope statement, update the build identity and validation evidence, and publish a new carry-forward report and build-notes file alongside each version's release.
