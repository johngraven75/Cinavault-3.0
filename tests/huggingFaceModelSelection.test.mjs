import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

const read = (path) => fs.readFileSync(path, "utf8");
const MODEL = "Qwen/Qwen3-4B-Instruct-2507";

test("CinaVault uses the verified efficient Hugging Face instruction model everywhere", () => {
  const ai = read("src-tauri/src/ai.rs");
  const db = read("src-tauri/src/db.rs");
  const frontend = read("src/services/aiProviderFallback.ts");
  assert.match(ai, new RegExp(MODEL.replaceAll("/", "\\/")));
  assert.match(db, new RegExp(MODEL.replaceAll("/", "\\/")));
  assert.match(frontend, new RegExp(MODEL.replaceAll("/", "\\/")));
});

test("Hugging Face CLI credentials survive application reinstall", () => {
  const ai = read("src-tauri/src/ai.rs");
  assert.match(ai, /cached_hf_token/);
  assert.match(ai, /\.join\("\.cache"\)/);
  assert.match(ai, /\.join\("huggingface"\)/);
  assert.match(ai, /\.join\("token"\)/);
  assert.match(ai, /hf_cache_auto_seeded/);
  assert.match(ai, /db\.set_setting_data\("hf_token", &token\)/);
});

test("existing Mistral defaults migrate without overriding a user-selected model", () => {
  const db = read("src-tauri/src/db.rs");
  assert.match(db, /WHERE key = 'ai_model' AND value = \?2/);
  assert.match(db, /mistralai\/Mistral-7B-Instruct-v0\.3/);
});


test("AI Agent restores persisted Hugging Face credentials before status loading", () => {
  const diagnostics = read("src/components/tabs/AIDiagnosticsTab.tsx");
  const ensureCall = diagnostics.indexOf('invoke("ensure_hf_token")');
  const configCall = diagnostics.indexOf('invoke<AiConfig>("get_ai_config")');
  assert.ok(ensureCall >= 0, "AI Agent must invoke secure token recovery");
  assert.ok(configCall > ensureCall, "token recovery must run before status loading");
});
