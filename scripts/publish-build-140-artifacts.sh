#!/usr/bin/env bash
set -euo pipefail

VERSION="1.0.140"
BUILD="140"
REPO_FULL_NAME="johngraven75/CinaVault-Premium"
RELEASE_DIR="releases/build-${BUILD}"
ARTIFACT_NAME="CinaVault-Premium-Windows-Installer-Build${BUILD}"
EXE_NAME="CinaVault Premium_${VERSION}_x64-setup.exe"
MSI_NAME="CinaVault Premium_${VERSION}_x64_en-US.msi"

log() {
  printf '\n==> %s\n' "$1"
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Missing required command: %s\n' "$1" >&2
    exit 1
  fi
}

repo_root() {
  git rev-parse --show-toplevel 2>/dev/null || pwd
}

find_first_file() {
  local pattern="$1"
  shift
  local root
  for root in "$@"; do
    [ -d "$root" ] || continue
    find "$root" -type f -iname "$pattern" 2>/dev/null | head -n 1
  done
}

need_cmd git
need_cmd node
need_cmd sha256sum

ROOT="$(repo_root)"
cd "$ROOT"

if [ ! -d .git ]; then
  printf 'Run this from inside a git clone of %s.\n' "$REPO_FULL_NAME" >&2
  exit 1
fi

log "Syncing main branch"
git fetch origin main
git checkout main
git pull --rebase origin main

log "Aligning version numbers to ${VERSION} / Build ${BUILD}"
VERSION="$VERSION" BUILD="$BUILD" node <<'NODE'
const fs = require('fs');
const version = process.env.VERSION;
const build = process.env.BUILD;

function writeJson(path, updater) {
  if (!fs.existsSync(path)) return;
  const data = JSON.parse(fs.readFileSync(path, 'utf8'));
  updater(data);
  fs.writeFileSync(path, JSON.stringify(data, null, 2) + '\n');
}

writeJson('package.json', data => {
  data.version = version;
});

writeJson('package-lock.json', data => {
  data.version = version;
  if (data.packages && data.packages['']) {
    data.packages[''].version = version;
  }
});

writeJson('src-tauri/tauri.conf.json', data => {
  data.version = version;
});

function replaceInFile(path, replacements) {
  if (!fs.existsSync(path)) return;
  let text = fs.readFileSync(path, 'utf8');
  for (const [pattern, replacement] of replacements) {
    text = text.replace(pattern, replacement);
  }
  fs.writeFileSync(path, text);
}

replaceInFile('src-tauri/Cargo.toml', [
  [/^version = "[^"]+"/m, `version = "${version}"`],
]);

replaceInFile('src/components/tabs/SettingsTab.tsx', [
  [/v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9]+)? · Build [0-9]+ · Tauri v2 \+ React 18/g, `v${version} · Build ${build} · Tauri v2 + React 18`],
]);
NODE

log "Locating Build ${BUILD} installer artifacts"
mkdir -p "$RELEASE_DIR"

CANDIDATE_DIRS=(
  "$ROOT/src-tauri/target/release/bundle"
  "$ROOT/releases/build-${BUILD}"
  "$HOME/cinavault-build-${BUILD}-clean-release/assets"
  "$HOME/cinavault-build-${BUILD}-clean-release"
  "$HOME/OneDrive/Documents/Desktop"
  "$HOME/Desktop"
  "/c/Users/johng/cinavault-build-${BUILD}-clean-release/assets"
  "/c/Users/johng/cinavault-build-${BUILD}-clean-release"
)

EXE_SOURCE="$(find_first_file "*${VERSION}*setup.exe" "${CANDIDATE_DIRS[@]}")"
MSI_SOURCE="$(find_first_file "*${VERSION}*.msi" "${CANDIDATE_DIRS[@]}")"

if [ -z "$EXE_SOURCE" ] || [ -z "$MSI_SOURCE" ]; then
  printf 'Could not find both Build %s installer artifacts.\n' "$BUILD" >&2
  printf 'Expected an EXE matching *%s*setup.exe and an MSI matching *%s*.msi.\n' "$VERSION" "$VERSION" >&2
  printf 'Checked these folders:\n' >&2
  printf ' - %s\n' "${CANDIDATE_DIRS[@]}" >&2
  exit 1
fi

printf 'EXE: %s\n' "$EXE_SOURCE"
printf 'MSI: %s\n' "$MSI_SOURCE"
cp -f "$EXE_SOURCE" "$RELEASE_DIR/$EXE_NAME"
cp -f "$MSI_SOURCE" "$RELEASE_DIR/$MSI_NAME"

log "Writing Build ${BUILD} release notes"
cat > "$RELEASE_DIR/RELEASE-NOTES.md" <<EOF
# CinaVault Premium Build ${BUILD} Notes

Version: ${VERSION}
Build: ${BUILD}
Artifact name: ${ARTIFACT_NAME}
Release folder: ${RELEASE_DIR}

Build ${BUILD} highlights:
- Keeps the Hyper-Neon Fusion Cyber HUD, Quantum Grid navigation, holographic cards, quick stats, and terminal panel experience from Build 137.
- Restores PGMA Modernized and Porn Site Nuxt metadata provider routing through Tauri command handlers.
- Requires the JavaScript surface regression test before installer creation so metadata provider wiring regressions are caught.
- Continues scanning all enabled media sources even when one source is missing or fails.
- Reports per-source scan counts, skipped disabled sources, failed sources, and ingestion errors instead of silently swallowing library upsert failures.
- Sets source item counts from files found, not only newly added files, so rescans no longer zero out source counts.
- Publishes Windows NSIS setup EXE, MSI, release notes, build notes, and SHA256 manifest.

Version alignment:
- package.json: ${VERSION}
- package-lock.json: ${VERSION}
- src-tauri/tauri.conf.json: ${VERSION}
- src-tauri/Cargo.toml: ${VERSION}
- About panel: v${VERSION} / Build ${BUILD}
EOF

if [ -f BUILD_NOTES_PGMA.md ]; then
  cp -f BUILD_NOTES_PGMA.md "$RELEASE_DIR/BUILD_NOTES_PGMA.md"
fi

log "Writing SHA256 manifest"
(
  cd "$RELEASE_DIR"
  rm -f SHA256SUMS.txt
  for file in *; do
    [ -f "$file" ] || continue
    [ "$file" = "SHA256SUMS.txt" ] && continue
    sha256sum "$file"
  done | sort -k 2 > SHA256SUMS.txt
)

log "Committing Build ${BUILD} repository updates"
git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src/components/tabs/SettingsTab.tsx "$RELEASE_DIR"

if git diff --cached --quiet; then
  printf 'No repository changes to commit. Build %s already matches.\n' "$BUILD"
else
  git commit -m "Publish Build ${BUILD} artifacts and version alignment"
  git push origin main
fi

log "Done"
printf 'Build %s / version %s is aligned and published in %s.\n' "$BUILD" "$VERSION" "$RELEASE_DIR"
