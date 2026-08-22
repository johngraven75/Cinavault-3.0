import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

test("Google Cast service and clearly marked Cast UI remain installed", () => {
  assert.equal(fs.existsSync("src/services/castingService.ts"), true);
  assert.equal(fs.existsSync("src/components/CastButton.tsx"), true);

  const service = fs.readFileSync("src/services/castingService.ts", "utf8");
  const ui = fs.readFileSync("src/components/CastButton.tsx", "utf8");

  assert.match(service, /discover_casting_devices/);
  assert.match(service, /start_casting/);
  assert.match(service, /CastingDeviceType = "chromecast"/);

  assert.match(ui, /📺 Cast|Cast/);
  assert.match(ui, /Google Cast/);
  assert.match(ui, /data-testid="cinavault-cast-button"/);
  assert.match(ui, /data-testid="cinavault-cast-tab"/);
});
