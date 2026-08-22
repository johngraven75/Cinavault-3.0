# CinaVault Premium Build 140 Installer Rebuild Notes

Build: 140
Date: 2026-06-30
Branch: main
Workflow: `.github/workflows/windows-installer.yml`
Expected artifact: `CinaVault-Premium-Windows-Installer-Build140`
Repository output folder: `releases/build-140/`

## Rebuild request

Rebuild the Windows installer from the current `main` branch and publish the installer artifacts back into the repository under `releases/build-140/` with generated SHA256 sums and an installer upload report.

This file intentionally lives outside `releases/**` so the push triggers the Windows installer workflow. The workflow copies this file into the staged installer artifact folder as `BUILD_NOTES_PGMA.md` before upload and publication.

## Required feature carry-forward

Preserve all existing CinaVault Premium features while publishing as Build 140, including the futuristic application shell, sidebar navigation, Cyber HUD header, Kodi-inspired skins, PGMA Modernized metadata provider, Porn Site Nuxt provider, local Nuxt endpoint support, and PGMA native bridge support.

## Metadata provider requirements

- PGMA Modernized metadata provider.
- Porn Site Nuxt metadata provider.
- Local Nuxt endpoint default: `http://localhost:42069/`.
- PGMA native bridge support retained.

## Verification

```powershell
npm run test:build140
npm run build
cargo test -- --nocapture
cargo check
npm run tauri build
```

## Artifact publication rule

Only publish real generated installer artifacts, hashes, build notes, and installer upload reports into `releases/build-140/`.
