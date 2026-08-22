// CinaVault Premium — Universal Plugin Compatibility Adapter
// Provides runtime bridge to load, configure, and execute MS-C / MS-B / MS-A plugins

import { invoke } from "@tauri-apps/api/core";
import type {
  PluginEntry,
  PluginPlatform,
  PluginStatus,
} from "./pluginRegistry";

export const PGMA_PLUGIN_ID = "px-pgma-modernized";

export const PGMA_DEFAULT_CONFIG = {
  plexPluginPath: "",
  sourceZipUrl:
    "https://github.com/CodyBerenson/PGMA-Modernized/archive/refs/heads/master.zip",
  defaultTarget: "cinavault-staging",
  notes:
    "PGMA runs through CinaVault's native metadata bridge. A Plex bundle is optional and is never reported as deployed unless a real deployment completes.",
  requiresPlexRestart: true,
  nativeToolchain: "native-rust-pgma-bridge",
  metadataSources: ["nfo", "localArtwork"],
  downloadArtwork: true,
  overwriteExistingMetadata: false,
  limit: 5000,
  autoDeployBundlesOnInstall: false,
  autoRefreshLibraryAfterDeploy: false,
};

// ── Adapter configuration per-platform ──
export interface AdapterConfig {
  platform: PluginPlatform;
  basePath: string; // local plugin install directory
  apiBase?: string; // for server-backed plugins
  apiKey?: string;
}

// ── Plugin runtime manifest (what we persist per installed plugin) ──
export interface InstalledPlugin {
  id: string;
  name: string;
  platform: PluginPlatform;
  version: string;
  installPath: string;
  configJson: string;
  enabled: boolean;
  lastRun?: string;
}

// ── Platform adapters ──

const JELLYFIN_DLL_MAP: Record<string, string> = {
  "jf-opensubtitles": "Jellyfin.Plugin.OpenSubtitles.dll",
  "jf-trakt": "Jellyfin.Plugin.Trakt.dll",
  "jf-fanart": "Jellyfin.Plugin.Fanart.dll",
  "jf-tvdb": "Jellyfin.Plugin.TheTvdb.dll",
  "jf-anidb": "Jellyfin.Plugin.AniDB.dll",
  "jf-anilist": "Jellyfin.Plugin.AniList.dll",
  "jf-kitsu": "Jellyfin.Plugin.Kitsu.dll",
  "jf-webhook": "Jellyfin.Plugin.Webhook.dll",
  "jf-ldap": "Jellyfin.Plugin.LDAP.Auth.dll",
  "jf-reports": "Jellyfin.Plugin.Reports.dll",
  "jf-playback-reporting": "Jellyfin.Plugin.PlaybackReporting.dll",
  "jf-tmdb-boxsets": "Jellyfin.Plugin.TMDbBoxSets.dll",
  "jf-bookshelf": "Jellyfin.Plugin.Bookshelf.dll",
  "jf-kodi-sync": "Jellyfin.Plugin.KodiSyncQueue.dll",
  "jf-dlna": "Jellyfin.Plugin.Dlna.dll",
  "jf-chapter-segments": "Jellyfin.Plugin.ChapterSegments.dll",
};

function shouldLogInvokeFailure(): boolean {
  return (
    typeof window !== "undefined" &&
    Boolean((window as any).__TAURI_INTERNALS__)
  );
}

function defaultConfigForPlugin(pluginId: string): Record<string, any> {
  return pluginId === PGMA_PLUGIN_ID ? { ...PGMA_DEFAULT_CONFIG } : {};
}

function normalizePlatform(platform: any): PluginPlatform {
  return ["jellyfin", "emby", "plex", "cinavault"].includes(platform)
    ? platform
    : "cinavault";
}

function isPgmaDeployAction(action: string): boolean {
  return ["deploy", "install", "update", "upgrade"].includes(action);
}

function isPgmaRefreshAction(action: string): boolean {
  return [
    "start",
    "run",
    "refresh",
    "refresh_library",
    "refreshLibrary",
  ].includes(action);
}

export class PluginAdapterEngine {
  private adapters: Map<PluginPlatform, AdapterConfig> = new Map();
  private installed: Map<string, InstalledPlugin> = new Map();

  constructor() {
    // Default adapter paths per platform
    this.adapters.set("jellyfin", {
      platform: "jellyfin",
      basePath: "%APPDATA%/CinaVault/plugins/jellyfin",
    });
    this.adapters.set("emby", {
      platform: "emby",
      basePath: "%APPDATA%/CinaVault/plugins/emby",
    });
    this.adapters.set("plex", {
      platform: "plex",
      basePath: "%APPDATA%/CinaVault/plugins/plex/Plug-ins",
    });
    this.adapters.set("cinavault", {
      platform: "cinavault",
      basePath: "%APPDATA%/CinaVault/plugins/native",
    });
  }

  // ── Install a plugin ──
  async installPlugin(plugin: PluginEntry): Promise<boolean> {
    const defaultConfig = defaultConfigForPlugin(plugin.id);
    if (this.installed.has(plugin.id)) {
      await this.setPluginEnabled(plugin.id, true);
      if (plugin.id === PGMA_PLUGIN_ID) {
        await this.setPluginConfig(plugin.id, {
          ...defaultConfig,
          ...this.getPluginConfig(plugin.id),
        });
      }
      return true;
    }

    try {
      await invoke("install_plugin", {
        pluginId: plugin.id,
        name: plugin.name,
        version: plugin.version,
        platforms: plugin.platforms,
        repoUrl: plugin.repo || "",
      });

      let deployResult: any = null;
      if (plugin.id === PGMA_PLUGIN_ID) {
        await invoke("run_plugin", {
          pluginId: plugin.id,
          action: "configure",
          config: JSON.stringify(defaultConfig),
        });
        if (defaultConfig.autoDeployBundlesOnInstall) {
          deployResult = await invoke("run_plugin", {
            pluginId: plugin.id,
            action: "deploy",
            config: JSON.stringify(defaultConfig),
          });
        }
      }

      const installPath =
        plugin.id === PGMA_PLUGIN_ID && deployResult?.targetPath
          ? deployResult.targetPath
          : this.resolveInstallPath(plugin);
      const installed: InstalledPlugin = {
        id: plugin.id,
        name: plugin.name,
        platform: plugin.platforms[0],
        version: plugin.version,
        installPath,
        configJson: JSON.stringify({
          ...defaultConfig,
          lastDeployTarget: deployResult?.targetPath,
          deployedBundles: deployResult?.bundles,
        }),
        enabled: true,
        lastRun: new Date().toISOString(),
      };
      this.installed.set(plugin.id, installed);
      return true;
    } catch (err) {
      if (shouldLogInvokeFailure()) {
        console.warn(`Plugin install failed: ${plugin.id}`, err);
      }
      return false;
    }
  }

  // ── Legacy no-op: catalog entries should remain available until users install them ──
  bootstrapCatalog(plugins: PluginEntry[]): number {
    void plugins;
    return 0;
  }

  // ── Uninstall a plugin ──
  async uninstallPlugin(pluginId: string): Promise<boolean> {
    if (pluginId === PGMA_PLUGIN_ID) {
      console.warn(
        "PGMA is a required adult metadata provider and cannot be removed.",
      );
      return false;
    }
    try {
      await invoke("uninstall_plugin", { pluginId });
      this.installed.delete(pluginId);
      return true;
    } catch (error) {
      if (shouldLogInvokeFailure()) {
        console.warn(`Plugin uninstall failed: ${pluginId}`, error);
      }
      return false;
    }
  }

  // ── Run / activate a plugin ──
  async runPlugin(pluginId: string, action: string = "start"): Promise<any> {
    const configObject =
      pluginId === PGMA_PLUGIN_ID
        ? { ...PGMA_DEFAULT_CONFIG, ...this.getPluginConfig(pluginId) }
        : undefined;
    const config = configObject ? JSON.stringify(configObject) : undefined;
    try {
      let result: any;
      if (pluginId === PGMA_PLUGIN_ID && isPgmaRefreshAction(action)) {
        result = await invoke("refresh_pgma_library", { config });
      } else if (pluginId === PGMA_PLUGIN_ID && isPgmaDeployAction(action)) {
        result = await invoke("run_plugin", {
          pluginId,
          action: "deploy",
          config,
        });
      } else {
        result = await invoke("run_plugin", { pluginId, action, config });
      }

      if (pluginId === PGMA_PLUGIN_ID && result && typeof result === "object") {
        const current = this.installed.get(pluginId);
        const nextConfig = {
          ...PGMA_DEFAULT_CONFIG,
          ...this.getPluginConfig(pluginId),
          lastDeployTarget: result.targetPath,
          deployedBundles: result.bundles,
          lastRefreshStats:
            result.scanned !== undefined
              ? {
                  scanned: result.scanned,
                  matched: result.matched,
                  updated: result.updated,
                  artworkDownloaded: result.artworkDownloaded,
                  skipped: result.skipped,
                  errors: result.errors,
                  message: result.message,
                }
              : undefined,
          requiresPlexRestart: result.requiresPlexRestart ?? true,
        };
        if (current) {
          this.installed.set(pluginId, {
            ...current,
            installPath: result.targetPath || current.installPath,
            configJson: JSON.stringify(nextConfig),
            enabled: true,
            lastRun: new Date().toISOString(),
          });
        }
      }
      return result;
    } catch (err) {
      if (shouldLogInvokeFailure()) {
        console.warn(`Plugin run failed: ${pluginId}`, err);
      }
      return { success: false, error: String(err) };
    }
  }

  // ── Get plugin config ──
  getPluginConfig(pluginId: string): Record<string, any> {
    const p = this.installed.get(pluginId);
    if (!p) return defaultConfigForPlugin(pluginId);
    try {
      return {
        ...defaultConfigForPlugin(pluginId),
        ...JSON.parse(p.configJson),
      };
    } catch {
      return defaultConfigForPlugin(pluginId);
    }
  }

  // ── Set plugin config ──
  async setPluginConfig(
    pluginId: string,
    config: Record<string, any>,
  ): Promise<void> {
    const nextConfig = { ...defaultConfigForPlugin(pluginId), ...config };
    await invoke("run_plugin", {
      pluginId,
      action: "configure",
      config: JSON.stringify(nextConfig),
    });
    const p = this.installed.get(pluginId);
    if (p) {
      this.installed.set(pluginId, {
        ...p,
        configJson: JSON.stringify(nextConfig),
      });
    }
  }

  // ── Enable / disable installed plugin ──
  async setPluginEnabled(pluginId: string, enabled: boolean): Promise<void> {
    await invoke("run_plugin", {
      pluginId,
      action: enabled ? "enable" : "disable",
    });
    const p = this.installed.get(pluginId);
    if (p) {
      this.installed.set(pluginId, { ...p, enabled });
    }
  }

  // ── Check compatibility ──
  checkCompatibility(plugin: PluginEntry): {
    compatible: boolean;
    reason: string;
  } {
    if (plugin.id === PGMA_PLUGIN_ID) {
      return {
        compatible: true,
        reason: "Native bundle deployer + CinaVault metadata bridge",
      };
    }

    // CinaVault native plugins are always compatible
    if (plugin.cinavaultNative) {
      return { compatible: true, reason: "CinaVault native adapter available" };
    }

    // MS-C .NET plugins: compatible via DLL bridge
    if (plugin.platforms.includes("jellyfin")) {
      const hasDll = JELLYFIN_DLL_MAP[plugin.id];
      return {
        compatible: true,
        reason: hasDll
          ? `MS-C DLL bridge: ${hasDll}`
          : "MS-C API-compatible adapter",
      };
    }

    // MS-B plugins: compatible via REST API bridge
    if (plugin.platforms.includes("emby")) {
      return { compatible: true, reason: "MS-B REST API adapter" };
    }

    // MS-A tools: compatible via CLI/process bridge
    if (plugin.platforms.includes("plex")) {
      return { compatible: true, reason: "MS-A tool bridge (CLI/Python)" };
    }

    return { compatible: false, reason: "No adapter available" };
  }

  // ── Resolve install path ──
  private resolveInstallPath(plugin: PluginEntry): string {
    if (plugin.id === PGMA_PLUGIN_ID) {
      return "%APPDATA%/CinaVault/plugins/plex/Plug-ins";
    }
    const platform = plugin.platforms[0] || "cinavault";
    const adapter = this.adapters.get(platform);
    const base = adapter?.basePath || "plugins";
    return `${base}/${plugin.id}`;
  }

  // ── Get all installed plugins ──
  getInstalled(): InstalledPlugin[] {
    return Array.from(this.installed.values());
  }

  getInstalledPlugin(pluginId: string): InstalledPlugin | undefined {
    return this.installed.get(pluginId);
  }

  // ── Check if a plugin is installed ──
  isInstalled(pluginId: string): boolean {
    return this.installed.has(pluginId);
  }

  // ── Load installed plugins from backend ──
  async loadFromBackend(): Promise<void> {
    try {
      const plugins = await invoke<any[]>("get_installed_plugins");
      const loaded = new Map<string, InstalledPlugin>();
      for (const p of plugins) {
        const id = String(p.id || p.pluginId || p.name || "");
        if (!id) continue;
        loaded.set(id, {
          id,
          name: p.name,
          platform: normalizePlatform(p.platform),
          version: p.version || "1.0.0",
          installPath: p.installPath || "",
          configJson:
            p.configJson || JSON.stringify(defaultConfigForPlugin(id)),
          enabled: p.enabled !== false,
        });
      }
      this.installed = loaded;
    } catch {
      // No backend — running in dev mode
    }
  }
}

// ── Singleton instance ──
export const pluginEngine = new PluginAdapterEngine();
