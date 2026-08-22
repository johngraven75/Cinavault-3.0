import type { PluginEntry } from "../data/pluginRegistry";
import type { MetadataProvider } from "../store/appStore";

export const PGMA_PLUGIN_ID = "px-pgma-modernized";

const PGMA_CATALOG_ENTRY: PluginEntry = {
  id: PGMA_PLUGIN_ID,
  name: "PGMA Modernized",
  description:
    "Preinstalled PGMA integration with Plex bundle deployment plus a native CinaVault metadata bridge that reads local sidecar/artwork data and writes matching fields into the library.",
  version: "master",
  author: "CodyBerenson / CinaVault",
  platforms: ["plex", "cinavault"],
  category: "metadata",
  status: "active",
  icon: "🧩",
  repo: "https://github.com/CodyBerenson/PGMA-Modernized",
  configurable: true,
  premium: false,
  cinavaultNative: true,
  tags: ["plex", "metadata", "bundle", "pgma", "library", "artwork"],
};

type MetadataProviderLike = Partial<MetadataProvider> | null | undefined;
type PluginSearchCandidate = Partial<
  Pick<PluginEntry, "name" | "description" | "tags">
>;
type PluginRuntimeState =
  { id?: unknown; enabled?: unknown } | null | undefined;
type PluginStatusCandidate = Pick<PluginEntry, "id" | "status"> &
  Partial<Pick<PluginEntry, "cinavaultNative">>;

export function matchesPluginSearch(
  plugin: PluginSearchCandidate,
  rawSearch: string,
): boolean {
  const search = rawSearch.trim().toLowerCase();
  if (!search) return true;

  const name = typeof plugin.name === "string" ? plugin.name.toLowerCase() : "";
  const description =
    typeof plugin.description === "string"
      ? plugin.description.toLowerCase()
      : "";
  const tags = Array.isArray(plugin.tags)
    ? plugin.tags
        .filter((tag): tag is string => typeof tag === "string")
        .map((tag) => tag.toLowerCase())
    : [];

  return (
    name.includes(search) ||
    description.includes(search) ||
    tags.some((tag) => tag.includes(search))
  );
}

export function getMetadataProviderInitials(name?: string | null): string {
  const trimmed = typeof name === "string" ? name.trim() : "";
  return trimmed ? trimmed.slice(0, 2).toUpperCase() : "??";
}

export function sanitizeMetadataProviders(
  persisted: unknown,
  defaults: MetadataProvider[],
): MetadataProvider[] {
  const merged = new Map<string, MetadataProvider>(
    defaults.map((provider) => [provider.id, { ...provider }]),
  );

  if (!Array.isArray(persisted)) {
    return Array.from(merged.values());
  }

  for (const candidate of persisted as MetadataProviderLike[]) {
    if (
      !candidate ||
      typeof candidate !== "object" ||
      typeof candidate.id !== "string"
    ) {
      continue;
    }

    const fallback = merged.get(candidate.id);
    const name =
      typeof candidate.name === "string" && candidate.name.trim()
        ? candidate.name.trim()
        : fallback?.name;
    const category =
      typeof candidate.category === "string" && candidate.category.trim()
        ? candidate.category.trim()
        : fallback?.category;

    if (!name || !category) {
      continue;
    }

    merged.set(candidate.id, {
      id: candidate.id,
      name,
      category,
      enabled:
        typeof candidate.enabled === "boolean"
          ? candidate.enabled
          : (fallback?.enabled ?? false),
    });
  }

  return Array.from(merged.values());
}

export function applyPluginRuntimeState<T extends PluginStatusCandidate>(
  registry: T[],
  installed: PluginRuntimeState[],
): T[] {
  const installedById = new Map<string, { enabled: boolean }>();

  for (const plugin of installed) {
    if (!plugin || typeof plugin.id !== "string" || !plugin.id.trim()) continue;
    installedById.set(plugin.id, {
      enabled: plugin.enabled !== false,
    });
  }

  const registryWithPreinstalled = registry.some(
    (plugin) => plugin.id === PGMA_PLUGIN_ID,
  )
    ? registry
    : [...registry, PGMA_CATALOG_ENTRY as unknown as T];

  return registryWithPreinstalled
    .filter((plugin) => plugin.id !== "px-pgma-modernized")
    .map((plugin) => {
      const runtime = installedById.get(plugin.id);
      if (!runtime) return { ...plugin };

      return {
        ...plugin,
        status: runtime.enabled
          ? plugin.cinavaultNative
            ? "active"
            : "installed"
          : "disabled",
      };
    });
}

export function getUnreadStatusMessages(
  messages: string[],
  lastReadIndex: number,
): string[] {
  const safeIndex = Number.isFinite(lastReadIndex)
    ? Math.max(0, Math.floor(lastReadIndex))
    : 0;
  return messages.slice(Math.min(safeIndex + 1, messages.length));
}
