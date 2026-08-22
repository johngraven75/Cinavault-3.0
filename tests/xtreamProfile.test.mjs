import test from "node:test";
import assert from "node:assert/strict";

import {
  buildAddXtreamProfileArgs,
  normalizeXtreamServerUrl,
} from "../src/utils/xtreamProfile.ts";

test("buildAddXtreamProfileArgs sends serverUrl for the Tauri command", () => {
  const args = buildAddXtreamProfileArgs({
    name: "Premium IPTV",
    server_url: " provider.example.com:8080 ",
    username: " viewer ",
    password: " secret ",
  });

  assert.deepEqual(args, {
    name: "Premium IPTV",
    serverUrl: "http://provider.example.com:8080",
    username: "viewer",
    password: "secret",
  });
  assert.equal(Object.hasOwn(args, "server_url"), false);
});

test("normalizeXtreamServerUrl strips common Xtream endpoint paths to the server base", () => {
  assert.equal(
    normalizeXtreamServerUrl(
      "https://provider.example.com:8443/player_api.php?username=u&password=p",
    ),
    "https://provider.example.com:8443",
  );
  assert.equal(
    normalizeXtreamServerUrl(
      "http://provider.example.com:8080/get.php?username=u&password=p&type=m3u_plus",
    ),
    "http://provider.example.com:8080",
  );
});

test("buildAddXtreamProfileArgs rejects unusable server URLs with a helpful message", () => {
  assert.throws(
    () =>
      buildAddXtreamProfileArgs({
        name: "Premium IPTV",
        server_url: "ftp://provider.example.com",
        username: "viewer",
        password: "secret",
      }),
    /Server URL must start with http:\/\/ or https:\/\//,
  );
});
