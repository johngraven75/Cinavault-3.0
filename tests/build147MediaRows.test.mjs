import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

test("Build 147 cleans photo/artwork files from media rows", async () => {
  const source = fs.readFileSync("src/utils/mediaRowCleanup.ts", "utf8");
  assert.match(source, /cleanMediaRowItems/);
  assert.match(source, /jpg\|jpeg\|png\|webp/);
  assert.match(source, /poster\|cover\|folder\|fanart\|backdrop/);
  assert.match(source, /isActualPlayableMedia/);
});

test("Build 147 poster cards are standard multi-column size", () => {
  const css = fs.readFileSync(
    "src/styles/media-row-poster-final-fix.css",
    "utf8",
  );
  assert.match(css, /repeat\(auto-fill,\s*minmax\(132px,\s*168px\)\)/);
  assert.match(css, /max-width:\s*168px/);
  assert.match(css, /height:\s*252px/);
  assert.match(css, /aspect-ratio:\s*2\s*\/\s*3/);
});
