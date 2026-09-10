// Build 140 Futuristic Application Shell compatibility retained by Build 170.
import CastButton from "./components/CastButton";
import "./styles/poster-card-standard.css";
import "./styles/media-row-poster-final-fix.css";
import "./styles/media-card-hard-fix.css";
import "./styles/media-card-final-standard.css";
import { useCallback, useEffect, useMemo, useRef } from "react";
import type { FC, JSX, WheelEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AnimatePresence, motion } from "framer-motion";
import { BrainCircuit, Layers3, RadioTower, Sparkles } from "lucide-react";
import { useAppStore, type TabId } from "./store/appStore";
import { applyTheme } from "./themes";
import "./data/pluginAdapterInitialize";
import Sidebar from "./components/Sidebar";
import Header from "./components/Header";
import ExperienceBackdrop from "./components/experience/ExperienceBackdrop";
import HomeTab from "./components/tabs/HomeTab";
import MediaSourcesTab from "./components/tabs/MediaSourcesTab";
import DownloadsTab from "./components/tabs/DownloadsTab";
import LiveTVTab from "./components/tabs/LiveTVTab";
import ServerTab from "./components/tabs/ServerTab";
import SecurityTab from "./components/tabs/SecurityTab";
import RemoteAccessTab from "./components/tabs/RemoteAccessTab";
import AdvancedTab from "./components/tabs/AdvancedTab";
import CloudNASTab from "./components/tabs/CloudNASTab";
import PluginsTab from "./components/tabs/PluginsTab";
import AIDiagnosticsTab from "./components/tabs/AIDiagnosticsTab";
import HFModelsTab from "./components/tabs/HFModelsTab";
import SettingsTab from "./components/tabs/SettingsTab";
import { pluginEngine } from "./data/pluginAdapter";
import {
  getWheelDeltaPixels,
  getWheelScrolledTop,
} from "./utils/pageWheelScroll";
import { AI_MEDIA_AGENT_ENABLED } from "./services/aiMediaAgent";
import { startAiMediaAutopilot } from "./services/aiMediaAutopilot";
import { getPreferredMediaServer } from "./services/serverProvider";
import { getEnabledCinaVaultFeatures } from "./features/cinavaultFeatureSuite";
import { WINDOW_TITLE } from "./buildInfo";
import {
  ensurePermanentMediaPluginsAtStartup,
  initializePermanentMediaPluginsAtStartup,
} from "./services/startupMediaPluginService";

const TAB_COMPONENTS: Record<TabId, FC> = {
  home: HomeTab,
  sources: MediaSourcesTab,
  downloads: DownloadsTab,
  livetv: LiveTVTab,
  server: ServerTab,
  security: SecurityTab,
  remote: RemoteAccessTab,
  advanced: AdvancedTab,
  cloud: CloudNASTab,
  plugins: PluginsTab,
  ai: AIDiagnosticsTab,
  "hf-models": HFModelsTab,
  settings: SettingsTab,
};

const TAB_TITLES: Record<
  TabId,
  { eyebrow: string; title: string; subtitle: string; mode: string }
> = {
  home: {
    eyebrow: "Cinematic Library",
    title: "The Vault",
    subtitle:
      "An AI-organized media universe with compact visual shelves, instant playback, living artwork, and automatic repair.",
    mode: "Experience",
  },
  sources: {
    eyebrow: "Autonomous Ingestion",
    title: "Source Constellation",
    subtitle:
      "Connect local folders, drives, cloud storage, and network libraries. New sources scan and enrich automatically.",
    mode: "Ingest",
  },
  downloads: {
    eyebrow: "Acquisition Stream",
    title: "Incoming Media",
    subtitle:
      "Observe downloads, imports, and automated handoff into the managed library pipeline.",
    mode: "Queue",
  },
  livetv: {
    eyebrow: "Broadcast Fabric",
    title: "Live Signal",
    subtitle:
      "Navigate channels, guide intelligence, and live streams through a unified cinematic interface.",
    mode: "Broadcast",
  },
  server: {
    eyebrow: "Embedded Media Core",
    title: "Server Nexus",
    subtitle:
      "Control the zero-setup media server, runtime services, streaming health, and connected clients.",
    mode: "Core",
  },
  security: {
    eyebrow: "Trusted Compute",
    title: "Security Matrix",
    subtitle:
      "Manage identity, encryption, VPN protection, threat scanning, and privacy boundaries.",
    mode: "Guard",
  },
  remote: {
    eyebrow: "Anywhere Access",
    title: "Remote Orbit",
    subtitle:
      "Automatic NAT traversal, encrypted cloud relay, account sessions, and remote client reachability.",
    mode: "Relay",
  },
  advanced: {
    eyebrow: "Expert Systems",
    title: "Control Lab",
    subtitle:
      "Deep diagnostics, repair controls, platform tuning, and advanced operational tooling.",
    mode: "Tune",
  },
  cloud: {
    eyebrow: "Storage Fabric",
    title: "Cloud Mesh",
    subtitle:
      "Unify NAS devices, cloud providers, sync paths, and distributed media storage.",
    mode: "Mesh",
  },
  plugins: {
    eyebrow: "Capability Layer",
    title: "Extension Forge",
    subtitle:
      "Manage metadata engines, compatibility bridges, playback tools, and permanent media extensions.",
    mode: "Extend",
  },
  ai: {
    eyebrow: "Autonomous Media Intelligence",
    title: "AI Autopilot",
    subtitle:
      "Observe and guide automated scanning, identification, artwork retrieval, repair, and library optimization.",
    mode: "Neural",
  },
  "hf-models": {
    eyebrow: "Public Model Intelligence",
    title: "Hugging Face Models",
    subtitle: "Search and select free, public, ungated reasoning models for CinaVault AI.",
    mode: "Models",
  },
  settings: {
    eyebrow: "Experience Design",
    title: "Personalize CinaVault",
    subtitle:
      "Shape appearance, behavior, automation policy, and persistent application preferences.",
    mode: "Config",
  },
};

const TAB_MOTION = {
  initial: {
    opacity: 0,
    y: 34,
    scale: 0.975,
    rotateX: 2.2,
    filter: "blur(12px)",
  },
  animate: {
    opacity: 1,
    y: 0,
    scale: 1,
    rotateX: 0,
    filter: "blur(0px)",
  },
  exit: {
    opacity: 0,
    y: -20,
    scale: 0.988,
    rotateX: -1.5,
    filter: "blur(10px)",
  },
};

function findScrollableAncestor(
  target: Element,
  root: HTMLElement,
): HTMLElement {
  let node: HTMLElement | null =
    target instanceof HTMLElement ? target : target.parentElement;

  while (node && node !== root) {
    const style = window.getComputedStyle(node);
    if (
      /(auto|scroll)/.test(style.overflowY) &&
      node.scrollHeight > node.clientHeight
    ) {
      return node;
    }
    node = node.parentElement;
  }

  return root;
}

function canScrollInDirection(
  element: HTMLElement,
  deltaPixels: number,
): boolean {
  if (deltaPixels > 0) {
    return element.scrollTop + element.clientHeight < element.scrollHeight - 1;
  }
  if (deltaPixels < 0) {
    return element.scrollTop > 0;
  }
  return false;
}

function readLocalPersistedState(): Record<string, string> {
  try {
    const raw = localStorage.getItem("cinavault_state");
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? parsed
      : {};
  } catch {
    return {};
  }
}

async function saveAllSettingsToBackend(
  state: Record<string, string>,
): Promise<void> {
  try {
    localStorage.setItem("cinavault_state", JSON.stringify(state));
  } catch (error) {
    console.warn("Local settings backup failed:", error);
  }
  for (const [key, value] of Object.entries(state)) {
    await invoke("set_setting", { key, value });
  }
}

export default function App(): JSX.Element {
  const {
    activeTab,
    currentTheme,
    sidebarCollapsed,
    settings,
    featureSettings,
    metadataProviders,
    scheduledTasks,
    cloudServices,
    libraryView,
    mediaItems,
    setMediaItems,
    addStatusMessage,
    getPersistedState,
    restorePersistedState,
  } = useAppStore();

  const isSaving = useRef(false);
  const hasRestoredSettings = useRef(false);
  const mainScrollRef = useRef<HTMLDivElement | null>(null);
  const activeTitle = TAB_TITLES[activeTab];
  const featureCount = useMemo(() => getEnabledCinaVaultFeatures().length, []);
  const serverName = useMemo(() => getPreferredMediaServer().primary, []);
  const startupPluginsReady = initializePermanentMediaPluginsAtStartup().ready;
  const aiAutopilotEnabled = settings.ai_media_autopilot_enabled !== "false";

  const saveState = useCallback(async () => {
    if (isSaving.current || !hasRestoredSettings.current) return;
    isSaving.current = true;
    try {
      await saveAllSettingsToBackend(getPersistedState());
    } catch (error) {
      console.error("Save state error:", error);
      addStatusMessage("Failed to save settings");
    } finally {
      isSaving.current = false;
    }
  }, [getPersistedState, addStatusMessage]);

  useEffect(() => {
    let cancelled = false;

    const initializeApplication = async () => {
      try {
        restorePersistedState(readLocalPersistedState());
        const localState = readLocalPersistedState();
        let persistedState = localState;
        try {
          const backendState =
            await invoke<Record<string, string>>("get_all_settings");
          persistedState = { ...localState, ...backendState };
        } catch (error) {
          console.warn("Backend settings unavailable; using local state:", error);
        }

        restorePersistedState(persistedState);
        hasRestoredSettings.current = true;
        if (cancelled) return;

        await pluginEngine.initialize();
        const mediaTools = await ensurePermanentMediaPluginsAtStartup();
        if (!mediaTools.ready) {
          const missing = mediaTools.tools
            .filter((tool) => !tool.installed)
            .map((tool) => tool.id)
            .join(", ");
          addStatusMessage(
            `Automatic media-tool setup needs attention: ${missing || "unknown tools"}`,
          );
        } else {
          addStatusMessage(
            "FFmpeg, FFprobe, yt-dlp, MediaInfo, and MKVToolNix loaded",
          );
        }

        const appWindow = getCurrentWindow();
        await appWindow.setTitle(WINDOW_TITLE);
      } catch (error) {
        console.error("Initialization error:", error);
        addStatusMessage("Failed to initialize application");
      }
    };

    void initializeApplication();
    return () => {
      cancelled = true;
    };
  }, [restorePersistedState, addStatusMessage]);

  useEffect(() => {
    return startAiMediaAutopilot({
      enabled: () =>
        useAppStore.getState().settings.ai_media_autopilot_enabled !== "false",
      addStatusMessage,
      setMediaItems,
      intervalMinutes: Number(settings.ai_media_autopilot_interval_minutes || 30),
    });
  }, [addStatusMessage, setMediaItems, settings.ai_media_autopilot_interval_minutes]);

  useEffect(() => {
    applyTheme(currentTheme);
  }, [currentTheme]);

  useEffect(() => {
    const timer = window.setTimeout(() => void saveState(), 1000);
    return () => window.clearTimeout(timer);
  }, [
    activeTab,
    currentTheme,
    sidebarCollapsed,
    settings,
    featureSettings,
    metadataProviders,
    scheduledTasks,
    cloudServices,
    libraryView,
    saveState,
  ]);

  const handleWheel = useCallback((event: WheelEvent<HTMLDivElement>) => {
    if (!(event.target instanceof Element)) return;
    const root = mainScrollRef.current;
    if (!root) return;
    const scrollable = findScrollableAncestor(event.target, root);
    const deltaPixels = getWheelDeltaPixels(
      event.deltaY,
      event.deltaMode,
      root.clientHeight,
    );

    if (!canScrollInDirection(scrollable, deltaPixels)) {
      const parentScroll = findScrollableAncestor(
        scrollable.parentElement || root,
        root,
      );
      if (canScrollInDirection(parentScroll, deltaPixels)) {
        parentScroll.scrollTop = getWheelScrolledTop(
          parentScroll.scrollTop,
          deltaPixels,
          parentScroll.scrollHeight,
          parentScroll.clientHeight,
        );
        event.preventDefault();
      }
    }
  }, []);

  const CurrentTabComponent = TAB_COMPONENTS[activeTab];

  return (
    <div className="app-shell cv-app min-h-screen overflow-hidden bg-[#02040a] text-cv-text">
      <ExperienceBackdrop />

      <motion.div
        className="cv-shell-frame"
        initial={{ opacity: 0, scale: 0.992 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 0.65, ease: [0.16, 1, 0.3, 1] }}
      >
        <Sidebar />
        <CastButton />

        <div data-testid="cinavault-permanent-media-plugins" style={{ display: "none" }}>
          {startupPluginsReady
            ? "Permanent media plugins ready"
            : "Permanent media plugins not ready"}
        </div>
        <div data-testid="cinavault-feature-suite" style={{ display: "none" }}>
          {featureCount} enabled media server features
        </div>
        <div data-testid="cinavault-proprietary-server" style={{ display: "none" }}>
          {serverName}
        </div>
        <div data-testid="cinavault-ai-media-agent" style={{ display: "none" }}>
          {AI_MEDIA_AGENT_ENABLED
            ? "AI Media Agent Enabled"
            : "AI Media Agent Disabled"}
        </div>

        <main className="cv-command-deck">
          <Header />

          <section className="cv-context-stage">
            <div className="relative z-10 grid min-h-[172px] grid-cols-1 items-end gap-5 p-5 lg:grid-cols-[minmax(0,1fr)_380px] lg:p-6">
              <motion.div
                key={`${activeTab}-title`}
                initial={{ opacity: 0, x: -20, filter: "blur(8px)" }}
                animate={{ opacity: 1, x: 0, filter: "blur(0px)" }}
                transition={{ duration: 0.35, ease: [0.16, 1, 0.3, 1] }}
              >
                <div className="cv-stage-kicker">
                  <Sparkles size={12} /> {activeTitle.eyebrow} / {activeTitle.mode}
                </div>
                <h1 className="cv-stage-title">{activeTitle.title}</h1>
                <p className="cv-stage-subtitle">{activeTitle.subtitle}</p>
              </motion.div>

              <div className="cv-stage-telemetry">
                <motion.button
                  type="button"
                  onClick={() =>
                    window.dispatchEvent(
                      new Event("cinavault:ai-autopilot-run"),
                    )
                  }
                  whileHover={{ y: -3, scale: 1.015 }}
                  whileTap={{ scale: 0.985 }}
                  className="cv-telemetry-card text-left"
                  title="Run AI Media Autopilot now"
                >
                  <BrainCircuit size={15} className="mb-2 text-fuchsia-300" />
                  <div className="cv-telemetry-value">
                    {aiAutopilotEnabled ? "Autonomous" : "Manual"}
                  </div>
                  <div className="cv-telemetry-label">AI media handling</div>
                </motion.button>

                <motion.div
                  whileHover={{ y: -3 }}
                  className="cv-telemetry-card"
                >
                  <Layers3 size={15} className="mb-2 text-cyan-200" />
                  <div className="cv-telemetry-value">{mediaItems.length}</div>
                  <div className="cv-telemetry-label">Loaded media cards</div>
                </motion.div>

                <motion.div
                  whileHover={{ y: -3 }}
                  className="cv-telemetry-card"
                >
                  <RadioTower size={15} className="mb-2 text-emerald-300" />
                  <div className="cv-telemetry-value">
                    {startupPluginsReady ? "Live" : "Syncing"}
                  </div>
                  <div className="cv-telemetry-label">Service fabric</div>
                </motion.div>
              </div>
            </div>
          </section>

          <div
            ref={mainScrollRef}
            className="cv-workspace-scroll"
            onWheel={handleWheel}
          >
            <AnimatePresence mode="wait">
              <motion.section
                key={activeTab}
                initial={TAB_MOTION.initial}
                animate={TAB_MOTION.animate}
                exit={TAB_MOTION.exit}
                transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
                className="cv-workspace-panel"
                style={{ transformPerspective: 1200 }}
              >
                {CurrentTabComponent ? (
                  <CurrentTabComponent />
                ) : (
                  <div>Tab not found</div>
                )}
              </motion.section>
            </AnimatePresence>
          </div>
        </main>
      </motion.div>
    </div>
  );
}
