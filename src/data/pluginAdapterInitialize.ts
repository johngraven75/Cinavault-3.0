import { invoke } from "@tauri-apps/api/core";
import { FULL_PLUGIN_REGISTRY, type PluginEntry } from "./pluginRegistry";
import { getStartupMediaPlugins } from "../plugins/permanentMediaPlugins";
import {
  PGMA_PLUGIN_ID,
  PGMA_DEFAULT_CONFIG,
  pluginEngine,
} from "./pluginAdapter";

declare module "./pluginAdapter" {
  interface PluginAdapterEngine {
    initialize(): Promise<void>;
  }
}

type PluginBootConfig = Record<string, unknown>;

type PluginConfigSeed = {
  pluginId: string;
  name: string;
  version: string;
  platform: string;
  category: string;
  configurable: boolean;
  defaultConfig: PluginBootConfig;
};

let quickInitialization: Promise<void> | null = null;
let backgroundMaintenance: Promise<void> | null = null;
let backgroundTimer: number | null = null;

function defaultConfigFor(plugin: PluginEntry): PluginBootConfig {
  if (plugin.id === PGMA_PLUGIN_ID) {
    return { ...PGMA_DEFAULT_CONFIG, enabled: true, installedAtBoot: true };
  }

  return {
    schemaVersion: 1,
    pluginId: plugin.id,
    name: plugin.name,
    version: plugin.version,
    platform: plugin.platforms[0] || "cinavault",
    enabled: true,
    installedAtBoot: true,
    configurable: plugin.configurable,
    category: plugin.category,
    status: "ready",
    source: "cinavault-default",
  };
}

function configSeeds(): PluginConfigSeed[] {
  return FULL_PLUGIN_REGISTRY.map((plugin) => ({
    pluginId: plugin.id,
    name: plugin.name,
    version: plugin.version,
    platform: plugin.platforms[0] || "cinavault",
    category: plugin.category,
    configurable: plugin.configurable,
    defaultConfig: defaultConfigFor(plugin),
  }));
}

function isValidConfig(raw: string | undefined): boolean {
  if (!raw || !raw.trim()) return false;
  try {
    const parsed: unknown = JSON.parse(raw);
    return Boolean(parsed && typeof parsed === "object" && !Array.isArray(parsed));
  } catch {
    return false;
  }
}

async function installAndValidatePlugin(plugin: PluginEntry): Promise<void> {
  const installed = pluginEngine.getInstalledPlugin(plugin.id);
  const needsInstall = !installed;
  const config = defaultConfigFor(plugin);

  if (needsInstall) {
    await invoke("install_plugin", {
      pluginId: plugin.id,
      name: plugin.name,
      version: plugin.version,
      platforms: plugin.platforms,
      repoUrl: plugin.repo || "",
    });
  }

  const currentConfig = pluginEngine.getInstalledPlugin(plugin.id)?.configJson;
  if (needsInstall || !isValidConfig(currentConfig)) {
    await invoke("run_plugin", {
      pluginId: plugin.id,
      action: "configure",
      config: JSON.stringify(config),
    });
  }

  await invoke("run_plugin", {
    pluginId: plugin.id,
    action: "enable",
  });
}

async function runBounded<T>(
  items: readonly T[],
  concurrency: number,
  worker: (item: T) => Promise<void>,
): Promise<void> {
  let nextIndex = 0;
  const runnerCount = Math.max(1, Math.min(concurrency, items.length || 1));

  const runners = Array.from({ length: runnerCount }, async () => {
    while (nextIndex < items.length) {
      const currentIndex = nextIndex;
      nextIndex += 1;
      await worker(items[currentIndex]);
    }
  });

  await Promise.all(runners);
}

async function maintainPluginsInBackground(): Promise<void> {
  try {
    const seeds = configSeeds();
    await invoke("ensure_plugin_config_files", { seeds });

    const startupPluginIds = new Set<string>([
      ...getStartupMediaPlugins().map((plugin) => plugin.id),
      PGMA_PLUGIN_ID,
    ]);
    const startupPlugins = FULL_PLUGIN_REGISTRY.filter((plugin) =>
      startupPluginIds.has(plugin.id),
    );

    await runBounded(startupPlugins, 3, async (plugin) => {
      try {
        await installAndValidatePlugin(plugin);
      } catch (error) {
        console.warn(`Plugin background validation skipped for ${plugin.id}:`, error);
      }
    });

    await invoke("ensure_plugin_config_files", { seeds });
    await pluginEngine.loadFromBackend();
  } catch (error) {
    console.warn("Plugin background maintenance did not complete:", error);
  }
}

function scheduleBackgroundMaintenance(): void {
  if (backgroundMaintenance || backgroundTimer !== null) return;

  const start = (): void => {
    backgroundTimer = null;
    backgroundMaintenance = maintainPluginsInBackground().finally(() => {
      backgroundMaintenance = null;
    });
  };

  if (typeof window === "undefined") {
    start();
    return;
  }

  backgroundTimer = window.setTimeout(start, 300);
}

async function initializePluginEngine(): Promise<void> {
  if (!quickInitialization) {
    quickInitialization = pluginEngine
      .loadFromBackend()
      .catch((error: unknown) => {
        console.warn("Installed plugins could not be loaded during startup:", error);
      })
      .then(() => {
        scheduleBackgroundMaintenance();
      });
  }

  await quickInitialization;
}

pluginEngine.initialize = initializePluginEngine;

export {};
