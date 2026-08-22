import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const target = resolve("tests/carryForwardVerification.test.mjs");
let source = readFileSync(target, "utf8");

const replacements = new Map([
  ["Build 140 Futuristic Sidebar Navigation", "NAV_ITEMS"],
  ["sidebar-active-panel", "media-center-active-nav"],
  ["sidebar-active-rail", "media-center-active-rail"],
]);

for (const [legacyToken, currentToken] of replacements) {
  if (!source.includes(legacyToken)) {
    throw new Error(`Expected legacy carry-forward token not found in registry: ${legacyToken}`);
  }
  source = source.replaceAll(legacyToken, currentToken);
}

writeFileSync(target, source, "utf8");
console.log("Normalized renamed carry-forward markers for the current main implementation.");
