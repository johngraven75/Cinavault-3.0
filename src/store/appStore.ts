// CinaVault Premium — Global State Store (Zustand) with Persistence
import { create } from "zustand";
import { sanitizeMetadataProviders } from "../utils/pluginUiSafety";

export type TabId =
  | "home"
  | "sources"
  | "downloads"
  | "livetv"
  | "server"
  | "security"
  | "remote"
  | "advanced"
  | "cloud"
  | "plugins"
  | "ai"
  | "hf-models"
  | "settings";

const VALID_TAB_IDS: readonly TabId[] = [
  "home",
  "sources",
  "downloads",
  "livetv",
  "server",
  "security",
  "remote",
  "advanced",
  "cloud",
  "plugins",
  "ai",
  "hf-models",
  "settings",
];

function isTabId(value: string): value is TabId {
  return VALID_TAB_IDS.includes(value as TabId);
}

export interface MediaItem {
  id?: number;
  title: string;
  file_path: string;
  media_type: string;
  year?: number;
  rating?: number;
  overview?: string;
  poster_path?: string;
  backdrop_path?: string;
  genre?: string;
  duration?: number;
  file_size?: number;
  resolution?: string;
  codec?: string;
  verified: boolean;
  watched: boolean;
  favorite: boolean;
  date_added: string;
  last_played?: string;
  tmdb_id?: string;
  imdb_id?: string;
  source_id?: number;
  nfo_path?: string;
}

export interface MediaSource {
  id?: number;
  path: string;
  source_type: string;
  name: string;
  enabled: boolean;
  last_scanned?: string;
  item_count: number;
}

// ── Metadata Provider State ──
export interface MetadataProvider {
  id: string;
  name: string;
  category: string;
  enabled: boolean;
}

// ── Scheduled Task State ──
export type TaskFrequency =
  "manual" | "on_scan" | "daily" | "weekly" | "on_import" | "never";

export interface ScheduledTaskConfig {
  thumbnails: TaskFrequency;
  chapter_images: TaskFrequency;
  metadata_check: TaskFrequency;
  match_unmatch: TaskFrequency;
}

export interface LibraryEnrichmentResult {
  type: "library_enrichment";
  status: string;
  mode: string;
  items_scanned: number;
  metadata_items_enriched: number;
  metadata_fields_updated: number;
  metadata_updated?: number;
  titles_improved: number;
  items_reclassified_as_adult: number;
  files_renamed: number;
  rename_collisions_skipped: number;
  rename_failures: number;
  low_confidence_metadata_only: number;
  skipped_missing_files: number;
  skipped_non_video_items: number;
  posters_downloaded?: number;
  sidecars_written?: number;
  provider_errors: string[];
}

// ── Cloud Service State ──
export type CloudServiceStatus =
  "connected" | "disconnected" | "connecting" | "error";

export interface CloudServiceState {
  id: string;
  status: CloudServiceStatus;
  account?: string;
  syncPath?: string;
  lastSync?: string;
}

export interface AppState {
  // Navigation
  activeTab: TabId;
  sidebarCollapsed: boolean;
  setActiveTab: (tab: TabId) => void;
  toggleSidebar: () => void;

  // Theme
  currentTheme: string;
  setTheme: (theme: string) => void;

  // Library
  mediaItems: MediaItem[];
  setMediaItems: (
    items: MediaItem[] | ((current: MediaItem[]) => MediaItem[]),
  ) => void;
  selectedMedia: MediaItem | null;
  setSelectedMedia: (item: MediaItem | null) => void;
  libraryView: "list" | "card";
  setLibraryView: (view: "list" | "card") => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;

  // Sources
  sources: MediaSource[];
  setSources: (sources: MediaSource[]) => void;

  // Scanning
  scanning: boolean;
  scanProgress: { total: number; current: number };
  setScanning: (s: boolean) => void;
  setScanProgress: (p: { total: number; current: number }) => void;

  // Server
  serverRunning: boolean;
  serverType: string;
  serverUrl: string;
  setServerStatus: (running: boolean, type_: string, url: string) => void;

  // Downloads
  downloading: boolean;
  setDownloading: (d: boolean) => void;

  // VPN
  vpnConnected: boolean;
  vpnLocation: string;
  setVpnStatus: (connected: boolean, location: string) => void;

  // AI
  aiProcessing: boolean;
  aiResult: any;
  setAiProcessing: (p: boolean) => void;
  setAiResult: (r: any) => void;

  // Settings
  settings: Record<string, string>;
  setSettings: (s: Record<string, string>) => void;
  setSetting: (key: string, value: string) => void;

  // Feature Settings (Advanced tab) — Premium is now default
  featureSettings: Record<string, { enabled: boolean; config: any }>;
  setFeatureSettings: (
    fs: Record<string, { enabled: boolean; config: any }>,
  ) => void;
  toggleFeature: (key: string) => void;

  // Metadata Providers — selectable per-provider
  metadataProviders: MetadataProvider[];
  setMetadataProviders: (p: MetadataProvider[]) => void;
  toggleMetadataProvider: (id: string) => void;
  enableAllProviders: (category?: string) => void;
  disableAllProviders: (category?: string) => void;

  // Scheduled Tasks
  scheduledTasks: ScheduledTaskConfig;
  setScheduledTasks: (t: ScheduledTaskConfig) => void;
  setTaskFrequency: (
    task: keyof ScheduledTaskConfig,
    freq: TaskFrequency,
  ) => void;

  // Cloud Services
  cloudServices: Record<string, CloudServiceState>;
  setCloudService: (id: string, state: Partial<CloudServiceState>) => void;

  // Status ticker
  statusMessages: string[];
  addStatusMessage: (msg: string) => void;

  // Loading
  loading: boolean;
  setLoading: (l: boolean) => void;

  // Persistence — collect all persistable state
  getPersistedState: () => Record<string, string>;
  restorePersistedState: (data: Record<string, string>) => void;
}

// ── Default metadata providers (all enabled by default — Premium standard) ──
const DEFAULT_PROVIDERS: MetadataProvider[] = [
  // Movies & TV
  { id: "tmdb", name: "TMDb", category: "Movies & TV", enabled: true },
  { id: "omdb", name: "OMDb", category: "Movies & TV", enabled: true },
  { id: "tvdb", name: "TVDB", category: "Movies & TV", enabled: true },
  { id: "trakt", name: "Trakt", category: "Movies & TV", enabled: true },
  { id: "imdb", name: "IMDb", category: "Movies & TV", enabled: true },
  {
    id: "rotten_tomatoes",
    name: "Rotten Tomatoes",
    category: "Movies & TV",
    enabled: true,
  },
  { id: "cinemeta", name: "CINEMETA", category: "Movies & TV", enabled: true },
  { id: "tvmaze", name: "TVMaze", category: "Movies & TV", enabled: true },
  // Music
  { id: "musicbrainz", name: "MusicBrainz", category: "Music", enabled: true },
  { id: "audiodb", name: "AudioDB", category: "Music", enabled: true },
  { id: "lastfm", name: "Last.fm", category: "Music", enabled: true },
  { id: "discogs", name: "Discogs", category: "Music", enabled: true },
  // Anime
  { id: "anidb", name: "AniDB", category: "Anime", enabled: true },
  { id: "anilist", name: "AniList", category: "Anime", enabled: true },
  { id: "myanimelist", name: "MyAnimeList", category: "Anime", enabled: true },
  { id: "kitsu", name: "Kitsu", category: "Anime", enabled: true },
  // Artwork
  { id: "fanarttv", name: "Fanart.tv", category: "Artwork", enabled: true },
  {
    id: "tmdb_images",
    name: "TheMovieDB Images",
    category: "Artwork",
    enabled: true,
  },
  // Adult
  { id: "pgma", name: "PGMA Modernized", category: "Adult", enabled: true },
  {
    id: "porn_site_nuxt",
    name: "Porn Site Nuxt",
    category: "Adult",
    enabled: true,
  },
  { id: "theporndb", name: "ThePornDB", category: "Adult", enabled: true },
  { id: "stashdb", name: "StashDB", category: "Adult", enabled: true },
  {
    id: "phoenixadult",
    name: "PhoenixAdult",
    category: "Adult",
    enabled: true,
  },
  { id: "iafd", name: "IAFD", category: "Adult", enabled: true },
  // Subtitles
  {
    id: "opensubtitles",
    name: "OpenSubtitles",
    category: "Subtitles",
    enabled: true,
  },
  { id: "subscene", name: "Subscene", category: "Subtitles", enabled: true },
  // Other
  { id: "igdb", name: "IGDB", category: "Other", enabled: true },
  { id: "openlibrary", name: "OpenLibrary", category: "Other", enabled: true },
  { id: "goodreads", name: "GoodReads", category: "Other", enabled: true },
  { id: "epg_guide", name: "EPG Guide", category: "Other", enabled: true },
  // Agents
  { id: "plex_agents", name: "MS-A Agents", category: "Agents", enabled: true },
  {
    id: "emby_providers",
    name: "MS-B Providers",
    category: "Agents",
    enabled: true,
  },
  {
    id: "jellyfin_providers",
    name: "MS-C Providers",
    category: "Agents",
    enabled: true,
  },
];

const DEFAULT_SCHEDULED_TASKS: ScheduledTaskConfig = {
  thumbnails: "on_scan",
  chapter_images: "on_scan",
  metadata_check: "daily",
  match_unmatch: "on_import",
};

// ── Premium feature defaults (all enabled) ──
const DEFAULT_FEATURE_SETTINGS: Record<
  string,
  { enabled: boolean; config: any }
> = {
  smart_collections: { enabled: true, config: {} },
  poster_sync: { enabled: true, config: {} },
  unified_library: { enabled: true, config: {} },
  watchlist: { enabled: true, config: {} },
  skip_intro: { enabled: true, config: {} },
  skip_outro: { enabled: true, config: {} },
  auto_next: { enabled: true, config: {} },
  auto_subtitles: { enabled: true, config: {} },
  chapter_thumbs: { enabled: true, config: {} },
  hw_transcoding: { enabled: true, config: {} },
  motion_effects: { enabled: true, config: {} },
  splash_screen: { enabled: true, config: {} },
  particle_effects: { enabled: true, config: {} },
  ai_visualizer: { enabled: true, config: {} },
  glassmorphism: { enabled: true, config: {} },
  starfield_header: { enabled: true, config: {} },
  animated_sidebar: { enabled: true, config: {} },
  emby_sdk: { enabled: true, config: {} },
  vpn_integration: { enabled: true, config: {} },
  ai_diagnostics: { enabled: true, config: {} },
  duplicate_finder: { enabled: true, config: {} },
  iptv_support: { enabled: true, config: {} },
  plugin_system: { enabled: true, config: {} },
};

// ── Premium settings defaults ──
const DEFAULT_SETTINGS: Record<string, string> = {
  theme: "vidhub_flagship",
  splash_enabled: "true",
  sidebar_collapsed: "false",
  motion_enabled: "true",
  skip_intro: "true",
  skip_outro: "true",
  auto_next: "true",
  auto_subtitles: "true",
  chapter_thumbs_enabled: "true",
  prefer_embedded_titles: "true",
  smart_collections: "true",
  poster_sync: "true",
  unified_library: "true",
  watchlist_enabled: "true",
  hw_transcoding: "true",
  quality_control: "auto",
  remote_access_enabled: "true",
  remote_manually_specify_port: "false",
  remote_public_port: "32400",
  remote_secure_connections: "preferred",
  remote_preferred_relay: "false",
  remote_allow_fallback: "true",
  remote_upload_limit_mbps: "20",
  remote_allowed_networks: "",
  remote_enable_upnp: "true",
  remote_enable_natpmp: "true",
  default_player: "system",
  particle_effects: "true",
  ai_visualizer: "true",
  glassmorphism: "true",
  starfield_header: "true",
  window_opacity: "100",
};

export const useAppStore = create<AppState>((set, get) => ({
  // Navigation
  activeTab: "home",
  sidebarCollapsed: false,
  setActiveTab: (tab) => set({ activeTab: tab }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),

  // Theme
  currentTheme: "vidhub_flagship",
  setTheme: (theme) => set({ currentTheme: theme }),

  // Library
  mediaItems: [],
  setMediaItems: (items) =>
    set((s) => ({
      mediaItems: typeof items === "function" ? items(s.mediaItems) : items,
    })),
  selectedMedia: null,
  setSelectedMedia: (item) => set({ selectedMedia: item }),
  libraryView: "card",
  setLibraryView: (view) => set({ libraryView: view }),
  searchQuery: "",
  setSearchQuery: (q) => set({ searchQuery: q }),

  // Sources
  sources: [],
  setSources: (sources) => set({ sources }),

  // Scanning
  scanning: false,
  scanProgress: { total: 0, current: 0 },
  setScanning: (s) => set({ scanning: s }),
  setScanProgress: (p) => set({ scanProgress: p }),

  // Server
  serverRunning: false,
  serverType: "jellyfin",
  serverUrl: "http://localhost:8096",
  setServerStatus: (running, type_, url) =>
    set({ serverRunning: running, serverType: type_, serverUrl: url }),

  // Downloads
  downloading: false,
  setDownloading: (d) => set({ downloading: d }),

  // VPN
  vpnConnected: false,
  vpnLocation: "",
  setVpnStatus: (connected, location) =>
    set({ vpnConnected: connected, vpnLocation: location }),

  // AI
  aiProcessing: false,
  aiResult: null,
  setAiProcessing: (p) => set({ aiProcessing: p }),
  setAiResult: (r) => set({ aiResult: r }),

  // Settings — Premium defaults applied
  settings: { ...DEFAULT_SETTINGS },
  setSettings: (s) => set({ settings: { ...DEFAULT_SETTINGS, ...s } }),
  setSetting: (key, value) =>
    set((s) => ({ settings: { ...s.settings, [key]: value } })),

  // Feature Settings — all Premium features ON by default
  featureSettings: { ...DEFAULT_FEATURE_SETTINGS },
  setFeatureSettings: (fs) => set({ featureSettings: fs }),
  toggleFeature: (key) =>
    set((s) => {
      const current = s.featureSettings[key] || { enabled: false, config: {} };
      return {
        featureSettings: {
          ...s.featureSettings,
          [key]: { ...current, enabled: !current.enabled },
        },
      };
    }),

  // Metadata Providers
  metadataProviders: [...DEFAULT_PROVIDERS],
  setMetadataProviders: (p) => set({ metadataProviders: p }),
  toggleMetadataProvider: (id) =>
    set((s) => ({
      metadataProviders: s.metadataProviders.map((p) =>
        p.id === id ? { ...p, enabled: !p.enabled } : p,
      ),
    })),
  enableAllProviders: (category) =>
    set((s) => ({
      metadataProviders: s.metadataProviders.map((p) =>
        !category || p.category === category ? { ...p, enabled: true } : p,
      ),
    })),
  disableAllProviders: (category) =>
    set((s) => ({
      metadataProviders: s.metadataProviders.map((p) =>
        !category || p.category === category ? { ...p, enabled: false } : p,
      ),
    })),

  // Scheduled Tasks
  scheduledTasks: { ...DEFAULT_SCHEDULED_TASKS },
  setScheduledTasks: (t) => set({ scheduledTasks: t }),
  setTaskFrequency: (task, freq) =>
    set((s) => ({
      scheduledTasks: { ...s.scheduledTasks, [task]: freq },
    })),

  // Cloud Services
  cloudServices: {
    onedrive: { id: "onedrive", status: "disconnected" },
    gdrive: { id: "gdrive", status: "disconnected" },
    dropbox: { id: "dropbox", status: "disconnected" },
  },
  setCloudService: (id, state) =>
    set((s) => ({
      cloudServices: {
        ...s.cloudServices,
        [id]: { ...s.cloudServices[id], ...state },
      },
    })),

  // Status
  statusMessages: [
    "CinaVault Premium initialized",
    "All systems operational — Premium Edition",
  ],
  addStatusMessage: (msg) =>
    set((s) => ({
      statusMessages: [...s.statusMessages.slice(-19), msg],
    })),

  // Loading
  loading: false,
  setLoading: (l) => set({ loading: l }),

  // ── Persistence: collect all saveable state into a flat Record ──
  getPersistedState: () => {
    const s = get();
    return {
      ...s.settings,
      _activeTab: s.activeTab,
      _sidebarCollapsed: String(s.sidebarCollapsed),
      _currentTheme: s.currentTheme,
      _libraryView: s.libraryView,
      _featureSettings: JSON.stringify(s.featureSettings),
      _metadataProviders: JSON.stringify(s.metadataProviders),
      _scheduledTasks: JSON.stringify(s.scheduledTasks),
      _cloudServices: JSON.stringify(s.cloudServices),
    };
  },

  // ── Persistence: restore state from a flat Record ──
  restorePersistedState: (data) => {
    const settings: Record<string, string> = {};
    let activeTab: TabId = "home";
    let sidebarCollapsed = false;
    let currentTheme = "vidhub_flagship";
    let libraryView: "list" | "card" = "card";
    let featureSettings = { ...DEFAULT_FEATURE_SETTINGS };
    let metadataProviders = [...DEFAULT_PROVIDERS];
    let scheduledTasks = { ...DEFAULT_SCHEDULED_TASKS };
    let cloudServices = get().cloudServices;

    for (const [key, value] of Object.entries(data)) {
      if (key === "_activeTab") {
        activeTab = isTabId(value) ? value : "home";
      } else if (key === "_sidebarCollapsed") {
        sidebarCollapsed = value === "true";
      } else if (key === "_currentTheme") {
        currentTheme = value;
      } else if (key === "_libraryView") {
        libraryView = value as "list" | "card";
      } else if (key === "_featureSettings") {
        try {
          featureSettings = {
            ...DEFAULT_FEATURE_SETTINGS,
            ...JSON.parse(value),
          };
        } catch {}
      } else if (key === "_metadataProviders") {
        try {
          metadataProviders = sanitizeMetadataProviders(
            JSON.parse(value),
            DEFAULT_PROVIDERS,
          );
        } catch {}
      } else if (key === "_scheduledTasks") {
        try {
          scheduledTasks = { ...DEFAULT_SCHEDULED_TASKS, ...JSON.parse(value) };
        } catch {}
      } else if (key === "_cloudServices") {
        try {
          cloudServices = { ...cloudServices, ...JSON.parse(value) };
        } catch {}
      } else {
        settings[key] = value;
      }
    }

    set({
      settings: { ...DEFAULT_SETTINGS, ...settings },
      activeTab,
      sidebarCollapsed,
      currentTheme,
      libraryView,
      featureSettings,
      metadataProviders,
      scheduledTasks,
      cloudServices,
    });
  },
}));
