import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

test("Build 143 AI Media Agent is permanently enabled and safe", () => {
  const source = fs.readFileSync("src/services/aiMediaAgent.ts", "utf8");
  assert.match(source, /AI_MEDIA_AGENT_ENABLED\s*=\s*true/);
  assert.match(source, /identify-media/);
  assert.match(source, /retrieve-posters/);
  assert.match(source, /enrich-metadata/);
  assert.match(source, /normalize-filename/);
  assert.match(source, /quarantine-duplicates/);
  assert.doesNotMatch(
    source,
    /unlinkSync|rmSync|deleteFile|permanently delete/i,
  );
});
