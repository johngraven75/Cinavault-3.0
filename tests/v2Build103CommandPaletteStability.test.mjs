import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");
const readJson = async (path) => JSON.parse(await read(path));

test("v2 Build 1.03 command palette stability layer remains last", async () => {
  const main = await read("src/main.tsx");
  const buildInfo = await read("src/buildInfo.ts");
  const build = await readJson("build-version.json");
  const uiStabilityIndex = main.indexOf('import "./styles/ui-stability.css"');
  const commandStabilityIndex = main.indexOf(
    'import "./styles/command-palette-stability.css"',
  );

  assert.ok(uiStabilityIndex >= 0, "existing v2 Build 1.02 stability layer is retained");
  assert.ok(
    commandStabilityIndex > uiStabilityIndex,
    "Ctrl+K overrides load after earlier shell styles",
  );
  assert.match(buildInfo, /build-version\.json/);
  assert.match(buildInfo, /BUILD_INFO/);
  assert.match(main, /BUILD_INFO\.displayName/);
  assert.equal(typeof build.displayName, "string");
  assert.match(build.displayName, /^v2(?:\.\d+)? Build \d+(?:\.\d+)?$/);
  assert.equal(typeof build.releaseTag, "string");
  assert.ok(build.releaseTag.startsWith("v2-build-"));
});

test("Ctrl+K overlay avoids WebView2 blur and transform composition", async () => {
  const css = await read("src/styles/command-palette-stability.css");
  const header = await read("src/components/Header.tsx");

  assert.match(css, /\.cv-command-palette-backdrop\s*\{/);
  assert.match(css, /backdrop-filter:\s*none\s*!important/);
  assert.match(css, /-webkit-backdrop-filter:\s*none\s*!important/);
  assert.match(css, /transform:\s*none\s*!important/);
  assert.match(css, /will-change:\s*auto\s*!important/);
  assert.match(css, /contain:\s*layout paint style/);
  assert.match(css, /background:\s*#02040d\s*!important/);
  assert.doesNotMatch(
    header,
    /cv-command-palette[\s\S]{0,500}scale:\s*0\./,
    "command palette must not reintroduce scale composition",
  );
});

test("command input and application surfaces remain explicitly dark", async () => {
  const css = await read("src/styles/command-palette-stability.css");

  assert.match(css, /\.cv-command-input\s*\{/);
  assert.match(css, /color-scheme:\s*dark/);
  assert.match(css, /html,[\s\S]*body,[\s\S]*#root/);
  assert.doesNotMatch(
    css,
    /background:\s*(white|#fff(?:fff)?|rgba?\(255,\s*255,\s*255,\s*1\))/i,
  );
});
