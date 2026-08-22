# CinaVault Beta 4 Build 1 Summary

Date: 2026-05-06 09:40:51 -04:00
Branch Target: beta-4
Build Sequence: Beta 4 / Build 1

## Included Work
- Added second-stage branded splash screen after initial loader using the new logo package.
- Added prominent in-app branding (header mark, sidebar brand block, About banner).
- Added new theme preset: `MediaFire Fusion` with matching blue/orange/red palette.
- Added platform organization folders: `platforms/windows`, `platforms/android`, `platforms/ios`.

## Verification
- `npm run build`: PASS
- `cargo check`: PASS (warnings only)
- `npm run tauri build`: PASS

## Artifacts
- `releases/CinaVault Premium_1.0.0_x64-setup-beta4-build1.exe`
- `releases/CinaVault Premium_1.0.0_x64_en-US-beta4-build1.msi`

## SHA256
- setup-beta4-build1.exe: 736ECE9829FC44B9B3126328E08C294A34F47B4B23FF6C36506FCADAAB1CDE70
- msi-beta4-build1.msi: 4DE367D6FBDA0742E42086A3987E3408732FE7473D6FD6E3D77140FF98A7589D
