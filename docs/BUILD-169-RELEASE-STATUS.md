# CinaVault Premium Build 169 Release Status

## Release state

**Published and verified.**

- Release name: `CinaVault Premium Build 169`
- Release tag: `build-169`
- Release page: <https://github.com/johngraven75/CinaVault-Premium/releases/tag/build-169>
- Successful release workflow run: <https://github.com/johngraven75/CinaVault-Premium/actions/runs/30143125904>
- Validation merge commit: `435663d1e3902762db354515308907e48350e3d1`
- Release workflow source commit: `0ca6b7b123d5c10451e4cc859c73512903cc2ecf`

## Implemented

- Future Horizon Casting Center UI
- Automatic casting-device discovery and selection without manual device IP entry
- Chromecast, AirPlay, Samsung Smart View, and DLNA device categories
- Native Tauri casting command module
- SSDP discovery for DLNA and Smart View renderers
- mDNS discovery probes for Chromecast and AirPlay receivers
- Native reachability verification and connection state
- AirPlay playback handoff
- Casting session state and playback control bridge
- Embedded CinaVault media server on port `32400`
- Account-password and access-key authentication for remote clients
- Account-scoped session tokens and permission enforcement
- Authenticated server, library, media-item, and byte-range streaming APIs
- Automated CI, installer build, release publishing, and safe repository maintenance workflows

## Verified gates

1. npm clean install — passed
2. TypeScript validation — passed
3. Carry-forward regression tests — passed
4. Production frontend build — passed
5. Native Rust compilation — passed
6. Genuine Windows bundle-resource preparation — passed
7. Windows Rust validation — passed
8. Windows MSI build — passed
9. Windows NSIS build — passed
10. Installer collection and type verification — passed
11. Installer artifact upload — passed
12. GitHub Release publication — passed
13. Release-success reporting — passed

## Published Windows installers

### NSIS installer

- File: `CinaVault Premium_1.7.169_x64-setup.exe`
- Size: `11,809,753` bytes
- SHA-256: `11d69e4b4a5736e9d873bb87f55d6b99109b4dfa33ca2b354895a1e96a314871`

### MSI installer

- File: `CinaVault Premium_1.7.169_x64_en-US.msi`
- Size: `15,069,184` bytes
- SHA-256: `d9f302f9e31775411241e6c26dd9f9d0f36f68831a381c554088071176415271`

## Workflow artifact verification

- Artifact: `cinavault-build-169-windows-installers-0ca6b7b123d5c10451e4cc859c73512903cc2ecf`
- Artifact size: `26,621,675` bytes
- Artifact SHA-256 digest: `ade84ada1dca11f31f8940442741cc6fd324753d1f3d05346136d99f6398b6e1`

The release workflow required both an `.msi` and an `.exe` before publication and completed with a successful conclusion.
