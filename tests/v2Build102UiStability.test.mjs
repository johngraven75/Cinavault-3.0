import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function read(relativePath) {
  const absolutePath = resolve(ROOT, relativePath);
  assert.ok(existsSync(absolutePath), `Required v2 Build 1.02 file is missing: ${relativePath}`);
  return readFileSync(absolutePath, "utf8").replaceAll("\r\n", "\n");
}

function requireTokens(source, tokens, label) {
  for (const token of tokens) {
    assert.ok(source.includes(token), `${label} token missing: ${token}`);
  }
}

test("v2 Build 1.02 catches tab and shell render failures", () => {
  const boundary = read("src/components/RuntimeErrorBoundary.tsx");
  requireTokens(
    boundary,
    [
      "getDerivedStateFromError",
      "componentDidCatch",
      "recoverToLibrary",
      'setActiveTab("home")',
      "cinavault_last_ui_error",
      "Return to Library",
      "Reload interface",
    ],
    "runtime recovery boundary",
  );
  assert.ok(!boundary.includes("localStorage.clear"), "UI recovery must not erase user settings");
});

test("v2 Build 1.02 installs global diagnostics and navigation recovery", () => {
  const main = read("src/main.tsx");
  requireTokens(
    main,
    [
      "RuntimeErrorBoundary",
      "ui-stability.css",
      'window.addEventListener("error"',
      'window.addEventListener("unhandledrejection"',
      "installNavigationWatchdog",
      "navigation-watchdog-hidden-panel",
      "navigation-watchdog-missing-panel",
      'setActiveTab("home")',
      'build: "v2 Build 1.02"',
    ],
    "application stability wiring",
  );
});

test("v2 Build 1.02 prevents white compositor frames without removing motion", () => {
  const styles = read("src/styles/ui-stability.css");
  requireTokens(
    styles,
    [
      "color-scheme: dark",
      "#02040a",
      ".cv-workspace-panel",
      "filter: none !important",
      "will-change: opacity, transform",
      ".cv-runtime-fallback",
      "@media (prefers-reduced-motion: reduce)",
    ],
    "WebView stability styles",
  );
  assert.ok(
    styles.includes("transition: transform 160ms ease"),
    "Stability fix must retain responsive interface motion",
  );
});
