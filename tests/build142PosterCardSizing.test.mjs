import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

test("Build 142 poster cards use standard multi-item row sizing", () => {
  const css = fs.readFileSync("src/styles/poster-card-standard.css", "utf8");
  assert.match(css, /repeat\(auto-fill,\s*minmax\(150px,\s*190px\)\)/);
  assert.match(css, /max-width:\s*190px/);
  assert.match(css, /aspect-ratio:\s*2\s*\/\s*3/);
  assert.match(css, /max-height:\s*285px/);
});
