import { invoke } from "@tauri-apps/api/core";
import {
  arePermanentMediaPluginsReady,
  getStartupMediaPlugins,
} from "../plugins/permanentMediaPlugins";

export function initializePermanentMediaPluginsAtStartup() {
  return {
    ready: arePermanentMediaPluginsReady(),
    startupPlugins: getStartupMediaPlugins(),
    message:
      "Permanent media plugins are available immediately; validation and repair continue in the background.",
  };
}

export type MediaToolStartupResult = {
  type: "media_tools_startup";
  status: "ready" | "missing_tools";
  ready: boolean;
  automatic: boolean;
  authorization_prompt_required: boolean;
  tools: Array<{ id: string; installed: boolean; version?: string | null }>;
  installations?: unknown[];
};

type MediaToolStatusResponse = {
  ready: boolean;
  tools: Array<{ id: string; installed: boolean; version?: string | null }>;
};

let startupStatusPromise: Promise<MediaToolStartupResult> | null = null;

function normalizeStatus(response: MediaToolStatusResponse): MediaToolStartupResult {
  return {
    type: "media_tools_startup",
    status: response.ready ? "ready" : "missing_tools",
    ready: response.ready,
    automatic: false,
    authorization_prompt_required: false,
    tools: response.tools,
  };
}

export async function ensurePermanentMediaPluginsAtStartup(): Promise<MediaToolStartupResult> {
  if (!startupStatusPromise) {
    startupStatusPromise = invoke<MediaToolStatusResponse>("get_media_tools_status")
      .then(normalizeStatus)
      .finally(() => {
        window.setTimeout(() => {
          startupStatusPromise = null;
        }, 30_000);
      });
  }

  return startupStatusPromise;
}

export async function repairPermanentMediaTools(): Promise<MediaToolStartupResult> {
  const result = await invoke<MediaToolStartupResult>("ensure_media_tools");
  startupStatusPromise = Promise.resolve(result);
  return result;
}
