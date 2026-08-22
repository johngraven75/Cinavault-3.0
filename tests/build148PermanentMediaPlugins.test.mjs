import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

test("Build 148 permanently installs and starts media helper plugins", () => {
  const plugins = fs.readFileSync(
    "src/plugins/permanentMediaPlugins.ts",
    "utf8",
  );
  const startup = fs.readFileSync(
    "src/services/startupMediaPluginService.ts",
    "utf8",
  );

  for (const id of ["ffmpeg", "yt-dlp", "mediainfo"]) {
    assert.match(plugins, new RegExp(`id:\\s*"${id}"`));
  }

  assert.match(plugins, /installed:\s*true/);
  assert.match(plugins, /enabled:\s*true/);
  assert.match(plugins, /startup:\s*true/);
  assert.match(plugins, /required:\s*true/);
  assert.match(startup, /initializePermanentMediaPluginsAtStartup/);
});
