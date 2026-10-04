# CinaVault 3.0 v3 Build 1 â Windows Installer Build Notes

## Build identity

- Product: CinaVault 3.0
- Edition: Server Foundation / Windows reference edition
- Semantic version: `3.0.0`
- Display name: `v3.0 Build 1`
- Release tag: `v3-build-1`
- Source branch: `main`

## Release workflow

- Workflow: `.github/workflows/release.yml`
- Runner: GitHub Actions `windows-latest`
- Build command: `npm run tauri build`
- Required outputs: Tauri MSI and NSIS setup EXE
- Publication: GitHub Release `v3-build-1`

## Validation gates

The workflow runs the repositoryâs TypeScript validation, carry-forward regression suite, Vite production build, native Rust check, Windows resource preparation, Tauri packaging, artifact presence checks, and GitHub Release publication. The Windows installer artifacts are uploaded only when both an `.msi` and `.exe` are present.

The Windows-specific master-gated workflow also defines MSI installation, application launch, and uninstall acceptance. This release workflow remains the v3 Build 1 publication path and does not alter the authoritative build identity.

## Artifact integrity

The release workflow copies only generated `.msi` and `.exe` installer outputs into the release asset directory. GitHub Release assets are therefore tied to the workflowâs verified source commit and are not hand-created or committed binaries.

## Known scope

This is the Windows desktop/reference installer for the CinaVault 3.0 foundation line. The loopback service remains non-destructive and does not claim production Windows Service registration, recursive catalogue writes, or remote-access enablement.
