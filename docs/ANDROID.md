# CinaVault Premium for Android

This branch converts the existing Tauri 2 application into a shared desktop/mobile project so the Android app can reuse the Windows React interface, Zustand state, SQLite data layer, and Tauri command surface.

## Prerequisites

- Node.js and npm
- Rust stable with Android targets
- Android Studio, Android SDK, Android NDK, and Java 17
- Environment variables required by Tauri Android (`JAVA_HOME`, `ANDROID_HOME`, `NDK_HOME`)

## Initialize and run

```bash
npm install
npm run android:init
npm run android:dev
```

Create a release bundle with:

```bash
npm run android:build
```

## Architecture

- `src/` remains the shared React application used by Windows and Android.
- `src-tauri/src/lib.rs` is the shared Tauri backend entry point.
- `src-tauri/src/main.rs` remains the small desktop launcher.
- Android's generated Gradle project is created by `tauri android init`.

## Platform compatibility work still required

The Windows application contains server-host and desktop utility features that Android cannot execute unchanged. These need Android-specific adapters or remote-server behavior:

- launching and supervising Jellyfin/Emby/Plex server processes
- installing or invoking FFmpeg, FFprobe, yt-dlp, MediaInfo, and MKVToolNix binaries
- unrestricted filesystem scans outside Android's Storage Access Framework
- antivirus, VPN, shell, and native player process control
- mDNS and Chromecast behavior that depends on Node desktop packages
- desktop window operations and external-process launching

The Android client should preserve the same screens and controls while routing unsupported local-server actions to a paired CinaVault Premium Windows server. Local Android-safe functions such as settings, metadata browsing, SQLite cache, IPTV playback, cloud/NAS access, downloads, and media browsing can remain native.

## Recommended next implementation order

1. Generate the Android project and establish a clean debug build.
2. Add an Android platform capability service in TypeScript.
3. make the application shell responsive for phone and tablet widths.
4. Replace desktop folder picking with Android document-tree selection.
5. Add a paired-server API for desktop-only commands.
6. Add Android playback, notification, foreground-service, and permission handling.
7. Build and sign an AAB/APK through GitHub Actions.
