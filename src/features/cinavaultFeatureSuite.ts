export type CinaVaultFeature = {
  id: string;
  name: string;
  enabled: true;
  category: string;
  description: string;
  safeMode?: boolean;
};

export const CINAVAULT_FEATURE_SUITE: CinaVaultFeature[] = [
  {
    id: "proprietary-cinavault-server",
    name: "Proprietary CinaVault Server",
    enabled: true,
    category: "server",
    description:
      "Primary CinaVault-owned media server with Jellyfin-compatible fallback.",
  },
  {
    id: "ai-library-manager",
    name: "AI Library Manager",
    enabled: true,
    category: "ai",
    description:
      "Identifies media, enriches metadata, retrieves posters, normalizes filenames, and manages duplicates safely.",
  },
  {
    id: "universal-metadata-engine",
    name: "Universal Metadata Engine",
    enabled: true,
    category: "metadata",
    description:
      "Aggregates metadata from multiple providers, NFO files, and custom sources.",
  },
  {
    id: "ai-video-enhancement",
    name: "AI Video Enhancement",
    enabled: true,
    category: "enhancement",
    description:
      "Framework for AI upscaling, denoising, HDR enhancement, frame interpolation, and audio enhancement.",
  },
  {
    id: "ai-recommendations",
    name: "AI Recommendations",
    enabled: true,
    category: "discovery",
    description:
      "Context-aware recommendations using watch history, mood, genres, runtime, and profile preferences.",
  },
  {
    id: "ai-collection-builder",
    name: "AI Collection Builder",
    enabled: true,
    category: "organization",
    description:
      "Automatically builds cinematic universes, actor/director sets, holiday lists, award collections, and smart collections.",
  },
  {
    id: "ai-duplicate-manager",
    name: "AI Duplicate Manager",
    enabled: true,
    category: "storage",
    safeMode: true,
    description:
      "Detects duplicate files by quality, codec, edition, audio, subtitles, and hash; quarantines before removal.",
  },
  {
    id: "ai-media-repair",
    name: "AI Media Repair",
    enabled: true,
    category: "repair",
    description:
      "Detects missing posters, broken metadata, corrupt files, missing subtitles, missing chapters, and repair opportunities.",
  },
  {
    id: "ai-server-health-monitor",
    name: "AI Server Health Monitor",
    enabled: true,
    category: "admin",
    description:
      "Tracks storage, CPU, RAM, GPU, transcoding load, bandwidth, failed scans, and server health.",
  },
  {
    id: "ai-download-assistant",
    name: "AI Download Assistant",
    enabled: true,
    category: "downloads",
    description:
      "Ranks release quality, avoids duplicates, identifies codecs/HDR, and flags suspicious downloads.",
  },
  {
    id: "multi-server-federation",
    name: "Multi-Server Federation",
    enabled: true,
    category: "server",
    description:
      "Unifies local, NAS, remote, and partner servers into one library view.",
  },
  {
    id: "plugin-marketplace",
    name: "Plugin Marketplace",
    enabled: true,
    category: "plugins",
    description:
      "One-click plugin install, updates, screenshots, ratings, compatibility, and rollback support.",
  },
  {
    id: "ai-voice-assistant",
    name: "AI Voice Assistant",
    enabled: true,
    category: "voice",
    description:
      "Voice control for playback, search, watchlists, continuation, and recommendations.",
  },
  {
    id: "netflix-style-home",
    name: "Netflix-Style Home Screen",
    enabled: true,
    category: "ui",
    description:
      "Dynamic rows including Continue Watching, Trending, Recently Added, Because You Watched, 4K HDR, and New Episodes.",
  },
  {
    id: "multi-profile-ai",
    name: "Multi-Profile AI",
    enabled: true,
    category: "profiles",
    description:
      "Separate AI learning and recommendations for each user profile.",
  },
  {
    id: "watchlist-importer",
    name: "AI Watchlist Importer",
    enabled: true,
    category: "import",
    description:
      "Imports watchlists from Plex, Jellyfin, Emby, IMDb, Letterboxd, Trakt, TMDb, CSV, and JSON.",
  },
  {
    id: "smart-collections",
    name: "AI Smart Collections",
    enabled: true,
    category: "organization",
    description:
      "Auto-updating smart collections for awards, decades, codecs, holidays, genres, studios, and quality tiers.",
  },
  {
    id: "remote-access-wizard",
    name: "Remote Access Wizard",
    enabled: true,
    category: "networking",
    description:
      "Guided setup for ports, HTTPS, certificates, reverse proxies, and connectivity checks.",
  },
  {
    id: "advanced-user-dashboard",
    name: "Advanced User Dashboard",
    enabled: true,
    category: "analytics",
    description:
      "Watch stats, storage analytics, bitrate distribution, transcoding history, bandwidth usage, and viewing history.",
  },
];

export function getEnabledCinaVaultFeatures() {
  return CINAVAULT_FEATURE_SUITE.filter((feature) => feature.enabled);
}

export function getCinaVaultFeatureById(id: string) {
  return CINAVAULT_FEATURE_SUITE.find((feature) => feature.id === id);
}

export function isDuplicateRemovalSafeModeEnabled() {
  return true;
}

export function keepAllExistingAppFeaturesAndSettings() {
  return true;
}
