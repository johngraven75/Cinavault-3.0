import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(new URL("..", import.meta.url).pathname);
const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");

test("Downloads tab wires MediaInfo and MKVToolNix inspection commands", () => {
  const ui = read("src/components/tabs/DownloadsTab.tsx");
  assert.match(ui, /inspect_with_mediainfo/);
  assert.match(ui, /inspect_with_mkvtoolnix/);
  assert.match(ui, /invoke<unknown>\(command, \{ path \}\)/);
  assert.match(ui, /setInspectionResult\(result\)/);
  assert.match(ui, /setToolError\(`Automatic tool repair failed/);
});

test("Downloads tab exposes repair progress and does not silently ignore invalid inspection input", () => {
  const ui = read("src/components/tabs/DownloadsTab.tsx");
  assert.match(ui, /toolRepairing/);
  assert.match(ui, /Repairing\.\.\./);
  assert.match(ui, /Enter a media file path before running an inspection/);
  assert.doesNotMatch(ui, /if \(!path\) return;/);
});

test("native media tools resolve PATH and standard Windows installation locations", () => {
  const tools = read("src-tauri/src/media_tools.rs");
  assert.match(tools, /fn executable_candidates/);
  assert.match(tools, /ProgramFiles/);
  assert.match(tools, /MediaInfo/);
  assert.match(tools, /MKVToolNix/);
  assert.match(tools, /fn resolve_executable/);
  assert.match(tools, /Command::new\(resolve_executable\(executable\)\)/);
});
