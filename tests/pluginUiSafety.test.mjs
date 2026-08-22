import test from "node:test";
import assert from "node:assert/strict";

import {
  applyPluginRuntimeState,
  getMetadataProviderInitials,
  getUnreadStatusMessages,
  matchesPluginSearch,
  sanitizeMetadataProviders,
} from "../src/utils/pluginUiSafety.ts";

test("sanitizeMetadataProviders keeps defaults and repairs malformed persisted providers", () => {
  const defaults = [
    { id: "tmdb", name: "TMDb", category: "Movies & TV", enabled: true },
    { id: "theporndb", name: "ThePornDB", category: "Adult", enabled: false },
  ];

  const persisted = [
    { id: "tmdb", name: "TMDb", category: "Movies & TV", enabled: false },
    { id: "theporndb", category: "Adult", enabled: true },
    { id: "", name: "", category: "", enabled: true },
  ];

  assert.deepEqual(sanitizeMetadataProviders(persisted, defaults), [
    { id: "tmdb", name: "TMDb", category: "Movies & TV", enabled: false },
    { id: "theporndb", name: "ThePornDB", category: "Adult", enabled: true },
  ]);
});

test("getMetadataProviderInitials never throws on missing provider names", () => {
  assert.equal(getMetadataProviderInitials("PhoenixAdult"), "PH");
  assert.equal(getMetadataProviderInitials(""), "??");
  assert.equal(getMetadataProviderInitials(undefined), "??");
});

test("matchesPluginSearch tolerates missing description and tags", () => {
  const plugin = {
    name: "ThePornDB",
    description: undefined,
    tags: undefined,
  };

  assert.equal(matchesPluginSearch(plugin, "porn"), true);
  assert.equal(matchesPluginSearch(plugin, "metadata"), false);
  assert.equal(matchesPluginSearch(plugin, ""), true);
});

test("applyPluginRuntimeState does not mark the whole catalog installed", () => {
  const registry = [
    { id: "cv-core", status: "active", cinavaultNative: true },
    { id: "jf-trakt", status: "available", cinavaultNative: true },
    { id: "px-bazarr", status: "available", cinavaultNative: true },
  ];

  assert.deepEqual(
    applyPluginRuntimeState(registry, [{ id: "jf-trakt", enabled: true }]).map(
      (plugin) => ({
        id: plugin.id,
        status: plugin.status,
      }),
    ),
    [
      { id: "cv-core", status: "active" },
      { id: "jf-trakt", status: "active" },
      { id: "px-bazarr", status: "available" },
    ],
  );
});

test("applyPluginRuntimeState reflects disabled installed plugins", () => {
  const registry = [
    { id: "jf-webhook", status: "available", cinavaultNative: false },
  ];

  assert.equal(
    applyPluginRuntimeState(registry, [{ id: "jf-webhook", enabled: false }])[0]
      .status,
    "disabled",
  );
});

test("getUnreadStatusMessages returns newest unread messages after the last read index", () => {
  assert.deepEqual(
    getUnreadStatusMessages(
      ["Started", "Scan complete", "Plugin installed"],
      0,
    ),
    ["Scan complete", "Plugin installed"],
  );
  assert.deepEqual(getUnreadStatusMessages(["Started"], 99), []);
});
