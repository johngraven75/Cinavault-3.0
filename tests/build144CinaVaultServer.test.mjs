import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

test("Build 144 adds proprietary CinaVault Server without removing Jellyfin compatibility", () => {
  const server = fs.readFileSync("src/server/cinavaultServer.ts", "utf8");
  const provider = fs.readFileSync("src/services/serverProvider.ts", "utf8");

  assert.match(server, /serverName:\s*"CinaVault Server"/);
  assert.match(server, /preserveJellyfinCompatibility:\s*true/);
  assert.match(server, /media-library/);
  assert.match(server, /users/);
  assert.match(server, /permissions/);
  assert.match(server, /metadata/);
  assert.match(server, /poster-retrieval/);
  assert.match(server, /transcoding/);
  assert.match(server, /remote-access/);
  assert.match(server, /plugins/);
  assert.match(server, /duplicate-management/);
  assert.match(server, /filename-normalization/);
  assert.match(server, /ai-media-agent/);

  assert.match(provider, /primary:\s*"cinavault-server"/);
  assert.match(provider, /fallback:\s*"jellyfin-compatible"/);
});
