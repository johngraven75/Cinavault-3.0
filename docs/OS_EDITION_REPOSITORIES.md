# CinaVault 3.0 OS Edition Repositories

CinaVault 3.0 is separate from CinaVault Premium. Each supported OS edition has its own repository and must retain the same v3 product identity.

| OS edition | Repository | Primary build command |
| --- | --- | --- |
| Windows | `johngraven75/Cinavault-3.0` | `npm run tauri:build:windows` |
| Ubuntu Linux | `johngraven75/Cinavault-3.0-Linux` | `npm run tauri:build:linux:ubuntu` |
| Android | `johngraven75/Cinavault-3.0-Android` | `npm run android:build` |
| iOS | `johngraven75/Cinavault-3.0-iOS` | `npm run ios:build` |

The Windows edition remains the reference for feature behavior and visual parity. Platform-specific exceptions must document the reason, equivalent user outcome, and regression coverage in `docs/platform-parity.json`.
