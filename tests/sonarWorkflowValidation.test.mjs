import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const workflow = readFileSync(resolve(ROOT, ".github/workflows/sonarcloud.yml"), "utf8").replace(/\r\n/g, "\n");

test("SonarCloud workflow gates execution without secrets in step conditions", () => {
  assert.ok(workflow.includes("id: sonar-config"));
  assert.ok(workflow.includes('echo "ready=true" >> "$GITHUB_OUTPUT"'));
  assert.ok(workflow.includes('echo "ready=false" >> "$GITHUB_OUTPUT"'));
  assert.ok(workflow.includes("if: ${{ steps.sonar-config.outputs.ready != 'true' }}"));
  assert.equal(
    workflow.includes("if: ${{ secrets.SONAR_TOKEN && vars.SONAR_PROJECT_KEY && vars.SONAR_ORGANIZATION }}"),
    false,
  );
  assert.equal(
    workflow.includes("if: ${{ !secrets.SONAR_TOKEN || !vars.SONAR_PROJECT_KEY || !vars.SONAR_ORGANIZATION }}"),
    false,
  );
});
