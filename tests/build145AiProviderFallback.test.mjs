import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

test("Build 145 AI provider falls back when Hugging Face credentials are missing", () => {
  const source = fs.readFileSync("src/services/aiProviderFallback.ts", "utf8");
  assert.match(source, /getSafeAiProvider/);
  assert.match(source, /provider:\s*"local"/);
  assert.match(source, /shouldSuppressUnauthorizedModelError/);
  assert.match(source, /401/);
  assert.match(source, /403/);
});
