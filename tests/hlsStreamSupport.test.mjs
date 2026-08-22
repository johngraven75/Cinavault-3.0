import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(resolve(ROOT, path), "utf8").replace(/\r\n/g, "\n");

test("HLS URLs are detected and routed through yt-dlp plus FFmpeg", () => {
  const downloads = read("src-tauri/src/downloads.rs");
  for (const token of [
    'lower.ends_with(".m3u8")',
    '"hls".into()',
    'args.push("--downloader".into())',
    'args.push("ffmpeg".into())',
    'args.push("--hls-use-mpegts".into())',
    'args.push("--hls-prefer-native".into())',
  ]) {
    assert.ok(downloads.includes(token), `missing HLS backend token: ${token}`);
  }
});

test("HLS and advanced media commands are exposed through Tauri", () => {
  const lib = read("src-tauri/src/lib.rs");
  for (const token of [
    "downloads::start_media_download",
    "downloads::crawl_media_links",
    "downloads::get_supported_media_types",
    "downloads::check_download_tools",
  ]) {
    assert.ok(lib.includes(token), `missing Tauri command wiring: ${token}`);
  }
});

test("Downloads UI detects HLS and exposes decode/download behavior", () => {
  const ui = read("src/components/tabs/DownloadsTab.tsx");
  for (const token of [
    "HLS stream detected",
    "Decode & Download HLS",
    '"start_media_download"',
    "yt-dlp + FFmpeg",
    "not DRM-protected",
  ]) {
    assert.ok(ui.includes(token), `missing HLS UI token: ${token}`);
  }
});
