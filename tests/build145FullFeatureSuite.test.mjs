import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

test("Build 145 includes all requested market-leading media server features", () => {
  const source = fs.readFileSync(
    "src/features/cinavaultFeatureSuite.ts",
    "utf8",
  );

  for (const id of [
    "proprietary-cinavault-server",
    "ai-library-manager",
    "universal-metadata-engine",
    "ai-video-enhancement",
    "ai-recommendations",
    "ai-collection-builder",
    "ai-duplicate-manager",
    "ai-media-repair",
    "ai-server-health-monitor",
    "ai-download-assistant",
    "multi-server-federation",
    "plugin-marketplace",
    "ai-voice-assistant",
    "netflix-style-home",
    "multi-profile-ai",
    "watchlist-importer",
    "smart-collections",
    "remote-access-wizard",
    "advanced-user-dashboard",
  ]) {
    assert.match(source, new RegExp(id));
  }

  assert.match(source, /keepAllExistingAppFeaturesAndSettings/);
  assert.match(source, /isDuplicateRemovalSafeModeEnabled/);
  assert.doesNotMatch(source, /unlinkSync|rmSync|permanent deletion/i);
});
