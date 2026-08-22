import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

test("Build 143 media cards are hard-clamped to standard poster size", () => {
  const css = fs.readFileSync("src/styles/media-card-hard-fix.css", "utf8");
  assert.match(css, /--cv-poster-card-width:\s*165px/);
  assert.match(css, /repeat\(auto-fill,\s*minmax\(145px,\s*165px\)\)/);
  assert.match(css, /height:\s*var\(--cv-poster-card-height\)\s*!important/);
  assert.match(css, /flex:\s*0 0 var\(--cv-poster-card-width\)\s*!important/);
});
