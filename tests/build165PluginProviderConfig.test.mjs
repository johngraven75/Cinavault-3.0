/**
 * Build 165 plugin/provider configuration contract.
 * Parses every shipped JSON config and verifies that adult-provider startup
 * seeds, runtime routing, and the unified metadata engine stay aligned.
 */
import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const CONFIG_DIR = resolve(ROOT, "plugins", "configs");
const ADULT_PROVIDERS = [
  "tpdb",
  "stashdb",
  "pgma",
  "porn_site_nuxt",
  "iafd",
  "phoenixadult",
];

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

test("every plugin config is valid, enabled, uniquely identified JSON", () => {
  assert.ok(existsSync(CONFIG_DIR), "plugins/configs must exist");
  const files = readdirSync(CONFIG_DIR).filter((file) =>
    file.endsWith(".json"),
  );
  assert.ok(files.length >= 20, "expected the complete plugin config catalog");

  const keys = new Set();
  for (const file of files) {
    const config = readJson(resolve(CONFIG_DIR, file));
    assert.equal(typeof config, "object", `${file}: config must be an object`);
    assert.ok(config._plugin?.trim(), `${file}: _plugin is required`);
    assert.ok(config._key?.trim(), `${file}: _key is required`);
    assert.ok(config._category?.trim(), `${file}: _category is required`);
    assert.ok(config._description?.trim(), `${file}: _description is required`);
    assert.equal(
      typeof config.enabled,
      "boolean",
      `${file}: enabled must be boolean`,
    );
    assert.equal(
      config.enabled,
      true,
      `${file}: shipped features must start enabled`,
    );
    assert.ok(!keys.has(config._key), `${file}: duplicate _key ${config._key}`);
    keys.add(config._key);

    for (const [field, value] of Object.entries(config)) {
      if (
        typeof value === "string" &&
        value.trim() &&
        (field.endsWith("_url") || field === "endpoint" || field === "base_url")
      ) {
        assert.match(
          value,
          /^(https?:\/\/|cinavault:\/\/)/,
          `${file}: ${field} must be a usable URL`,
        );
      }
    }
  }
});

test("all adult providers are enabled and aligned from config to startup routing", () => {
  const engine = readJson(resolve(CONFIG_DIR, "cv-metadata-engine.json"));
  const dbSource = readFileSync(
    resolve(ROOT, "src-tauri", "src", "db.rs"),
    "utf8",
  );
  const metadataSource = readFileSync(
    resolve(ROOT, "src-tauri", "src", "metadata.rs"),
    "utf8",
  );

  for (const provider of ADULT_PROVIDERS) {
    const configPath = resolve(CONFIG_DIR, `${provider}.json`);
    assert.ok(
      existsSync(configPath),
      `missing adult provider config: ${provider}`,
    );
    const config = readJson(configPath);
    assert.equal(config._key, provider);
    assert.equal(config.enabled, true);
    assert.ok(
      engine.adult_providers.includes(provider),
      `metadata engine omits ${provider}`,
    );
    assert.match(
      dbSource,
      new RegExp(`["']${provider}["']`),
      `startup database defaults omit ${provider}`,
    );
    assert.match(
      metadataSource,
      new RegExp(`["']${provider}["']`),
      `runtime metadata routing omits ${provider}`,
    );
  }
});
