// CinaVault 3.0 — Settings Tab (platform defaults + persistent settings)
import React, { useState, useCallback } from "react";
import { motion } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "../../store/appStore";
import { THEME_PRESETS, applyTheme } from "../../themes";
import { BUILD_INFO } from "../../buildInfo";
import {
  Settings,
  Palette,
  Monitor,
  Zap,
  Film,
  Shield,
  Save,
  RotateCcw,
  Eye,
  Sparkles,
  Volume2,
  Subtitles,
  SkipForward,
  Layers,
  CheckCircle2,
  Info,
  Download,
  RefreshCw,
  HardDrive,
} from "lucide-react";

export default function SettingsTab() {
  const {
    settings,
    setSetting,
    currentTheme,
    setTheme,
    featureSettings,
    toggleFeature,
    addStatusMessage,
    getPersistedState,
  } = useAppStore();

  const [saving, setSaving] = useState(false);
  const [activeSection, setActiveSection] = useState("appearance");

  // ── Manual save ──
  const handleSave = useCallback(async () => {
    setSaving(true);
    addStatusMessage("Saving all settings...");
    try {
      const state = getPersistedState();
      for (const [key, value] of Object.entries(state)) {
        await invoke("set_setting", { key, value });
      }
      addStatusMessage("All settings saved successfully");
    } catch {
      // Fallback to localStorage
      try {
        const state = getPersistedState();
        localStorage.setItem("cinavault_state", JSON.stringify(state));
        addStatusMessage("Settings saved to local storage");
      } catch {
        addStatusMessage("Settings save failed — will retry on exit");
      }
    }
    setSaving(false);
  }, [getPersistedState, addStatusMessage]);

  // ── Reset to platform defaults ──
  const handleReset = useCallback(() => {
    const platformDefaults: Record<string, string> = {
      theme: "vidhub_flagship",
      splash_enabled: "true",
      sidebar_collapsed: "false",
      motion_enabled: "true",
      skip_intro: "true",
      skip_outro: "true",
      auto_next: "true",
      auto_subtitles: "true",
      chapter_thumbs_enabled: "true",
      smart_collections: "true",
      poster_sync: "true",
      unified_library: "true",
      watchlist_enabled: "true",
      hw_transcoding: "true",
      quality_control: "auto",
      default_player: "system",
      particle_effects: "true",
      ai_visualizer: "true",
      glassmorphism: "true",
      starfield_header: "true",
      window_opacity: "100",
    };
    for (const [k, v] of Object.entries(platformDefaults)) {
      setSetting(k, v);
    }
    setTheme("vidhub_flagship");
    applyTheme("vidhub_flagship");
    addStatusMessage(
      "Settings reset to CinaVault 3.0 defaults - all features enabled",
    );
  }, [setSetting, setTheme, addStatusMessage]);

  // Toggle helper
  const isOn = (key: string) => settings[key] === "true";
  const toggle = (key: string) => setSetting(key, isOn(key) ? "false" : "true");

  const SECTIONS = [
    { id: "appearance", label: "Appearance", icon: Palette },
    { id: "playback", label: "Playback", icon: Film },
    { id: "library", label: "Library", icon: Layers },
    { id: "effects", label: "Visual Effects", icon: Sparkles },
    { id: "performance", label: "Performance", icon: Zap },
    { id: "about", label: "About", icon: Info },
  ];

  return (
    <div className="flex gap-5 h-full">
      {/* ── Section Nav ── */}
      <div className="w-48 shrink-0 space-y-1">
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            onClick={() => setActiveSection(s.id)}
            className={`w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl text-xs font-medium transition-all ${
              activeSection === s.id
                ? "bg-[var(--cv-accent)]/15 text-[var(--cv-accent)]"
                : "hover:bg-white/5 text-[var(--cv-subtext)]"
            }`}
          >
            <s.icon size={14} />
            {s.label}
          </button>
        ))}

        {/* Save / Reset */}
        <div className="pt-3 space-y-2">
          <button
            onClick={handleSave}
            disabled={saving}
            className="w-full cv-btn text-xs py-2.5 flex items-center justify-center gap-1.5 disabled:opacity-50"
          >
            {saving ? (
              <RefreshCw size={12} className="animate-spin" />
            ) : (
              <Save size={12} />
            )}
            {saving ? "Saving..." : "Save All Settings"}
          </button>
          <button
            onClick={handleReset}
            className="w-full cv-btn text-xs py-2.5 flex items-center justify-center gap-1.5 bg-white/5"
          >
            <RotateCcw size={12} /> Reset to 3.0 Defaults
          </button>
        </div>
      </div>

      {/* ── Content ── */}
      <div className="flex-1 min-w-0">
        <motion.div
          key={activeSection}
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          className="space-y-4"
        >
          {/* ═══ Appearance ═══ */}
          {activeSection === "appearance" && (
            <>
              <SectionHeader
                title="Themes & Skins"
                desc="Choose a visual theme or Kodi-inspired CinaVault skin - 3.0 includes all presets by default"
              />
              <div className="grid grid-cols-2 xl:grid-cols-3 gap-3">
                {THEME_PRESETS.map((theme) => (
                  <button
                    key={theme.id}
                    onClick={() => {
                      setTheme(theme.id);
                      applyTheme(theme.id);
                      setSetting("theme", theme.id);
                    }}
                    aria-label={`Select ${theme.name} skin`}
                    className={`p-3 rounded-xl border text-left transition-all ${
                      currentTheme === theme.id
                        ? "border-[var(--cv-accent)] bg-[var(--cv-accent)]/10"
                        : "border-white/5 bg-white/3 hover:bg-white/5"
                    }`}
                  >
                    <div className="flex items-center justify-between gap-2 mb-2">
                      <div className="flex gap-1">
                        {Object.values(theme.colors)
                          .slice(0, 5)
                          .map((c, i) => (
                            <div
                              key={i}
                              className="w-4 h-4 rounded-full"
                              style={{ background: c }}
                            />
                          ))}
                      </div>
                      {theme.origin === "Kodi" && (
                        <span
                          className="text-[9px] uppercase tracking-wide rounded-full px-2 py-0.5 border border-[var(--cv-accent)]/30 bg-[var(--cv-accent)]/10"
                          style={{ color: "var(--cv-accent)" }}
                        >
                          Kodi Skin
                        </span>
                      )}
                    </div>
                    <div
                      className="text-xs font-medium"
                      style={{
                        color:
                          currentTheme === theme.id
                            ? "var(--cv-accent)"
                            : "var(--cv-text)",
                      }}
                    >
                      {theme.name}
                    </div>
                    {theme.description && (
                      <div
                        className="mt-1 text-[10px] leading-snug"
                        style={{ color: "var(--cv-subtext)" }}
                      >
                        {theme.description}
                      </div>
                    )}
                  </button>
                ))}
              </div>

              <SectionHeader title="Window" desc="Window appearance settings" />
              <SettingRow
                label="Window Opacity"
                desc="Background transparency level"
              >
                <div className="flex items-center gap-2">
                  <input
                    type="range"
                    min="60"
                    max="100"
                    value={settings.window_opacity || "100"}
                    onChange={(e) =>
                      setSetting("window_opacity", e.target.value)
                    }
                    className="w-32 accent-[var(--cv-accent)]"
                  />
                  <span
                    className="text-xs w-8 text-right"
                    style={{ color: "var(--cv-text)" }}
                  >
                    {settings.window_opacity || 100}%
                  </span>
                </div>
              </SettingRow>
              <ToggleRow
                label="Show Splash Screen"
                desc="Animated splash on startup"
                checked={isOn("splash_enabled")}
                onChange={() => toggle("splash_enabled")}
              />
            </>
          )}

          {/* ═══ Playback ═══ */}
          {activeSection === "playback" && (
            <>
              <SectionHeader
                title="Playback Controls"
                desc="All 3.0 playback features enabled by default"
              />
              <ToggleRow
                label="Skip Intro Detection"
                desc="Automatically detect and skip intros"
                checked={isOn("skip_intro")}
                onChange={() => toggle("skip_intro")}
              />
              <ToggleRow
                label="Skip Outro / Credits"
                desc="Auto-skip end credits and outros"
                checked={isOn("skip_outro")}
                onChange={() => toggle("skip_outro")}
              />
              <ToggleRow
                label="Auto-Play Next Episode"
                desc="Seamlessly play the next episode in a series"
                checked={isOn("auto_next")}
                onChange={() => toggle("auto_next")}
              />
              <ToggleRow
                label="Auto-Download Subtitles"
                desc="Fetch subtitles automatically for all media"
                checked={isOn("auto_subtitles")}
                onChange={() => toggle("auto_subtitles")}
              />
              <ToggleRow
                label="Chapter Thumbnails"
                desc="Generate preview thumbnails for video chapters"
                checked={isOn("chapter_thumbs_enabled")}
                onChange={() => toggle("chapter_thumbs_enabled")}
              />

              <SectionHeader
                title="Default Player"
                desc="Which player to use for media playback"
              />
              <SettingRow
                label="Player"
                desc="System default or built-in Vidstack player"
              >
                <select
                  value={settings.default_player || "system"}
                  onChange={(e) => setSetting("default_player", e.target.value)}
                  className="cv-input text-xs min-w-[140px]"
                >
                  <option value="system">System Default</option>
                  <option value="vidstack">Vidstack (Built-in)</option>
                  <option value="mpv">MPV</option>
                  <option value="vlc">VLC</option>
                </select>
              </SettingRow>
            </>
          )}

          {/* ═══ Library ═══ */}
          {activeSection === "library" && (
            <>
              <SectionHeader
                title="Library Features"
                desc="Smart library management - all 3.0 features active"
              />
              <ToggleRow
                label="Smart Collections"
                desc="Auto-generate collections based on genres, years, and actors"
                checked={isOn("smart_collections")}
                onChange={() => toggle("smart_collections")}
              />
              <ToggleRow
                label="Poster Sync"
                desc="Keep poster artwork synced across all connected servers"
                checked={isOn("poster_sync")}
                onChange={() => toggle("poster_sync")}
              />
              <ToggleRow
                label="Unified Library"
                desc="Merge multiple server libraries into one unified view"
                checked={isOn("unified_library")}
                onChange={() => toggle("unified_library")}
              />
              <ToggleRow
                label="Watchlist"
                desc="Track what you want to watch next"
                checked={isOn("watchlist_enabled")}
                onChange={() => toggle("watchlist_enabled")}
              />
            </>
          )}

          {/* ═══ Visual Effects ═══ */}
          {activeSection === "effects" && (
            <>
              <SectionHeader
                title="Visual Effects"
                desc="CinaVault 3.0 visual enhancements - all enabled by default"
              />
              <ToggleRow
                label="Motion Animations"
                desc="Smooth page transitions and micro-animations"
                checked={isOn("motion_enabled")}
                onChange={() => toggle("motion_enabled")}
              />
              <ToggleRow
                label="Particle Effects"
                desc="Ambient floating particle system in backgrounds"
                checked={isOn("particle_effects")}
                onChange={() => toggle("particle_effects")}
              />
              <ToggleRow
                label="AI Visualizer"
                desc="Neural network-style visual effects and animations"
                checked={isOn("ai_visualizer")}
                onChange={() => toggle("ai_visualizer")}
              />
              <ToggleRow
                label="Glassmorphism"
                desc="Frosted glass UI panels with blur effects"
                checked={isOn("glassmorphism")}
                onChange={() => toggle("glassmorphism")}
              />
              <ToggleRow
                label="Starfield Header"
                desc="Animated starfield with parallax in the header bar"
                checked={isOn("starfield_header")}
                onChange={() => toggle("starfield_header")}
              />
            </>
          )}

          {/* ═══ Performance ═══ */}
          {activeSection === "performance" && (
            <>
              <SectionHeader
                title="Performance"
                desc="Hardware acceleration and transcoding settings"
              />
              <ToggleRow
                label="Hardware Transcoding"
                desc="Use GPU acceleration for video transcoding"
                checked={isOn("hw_transcoding")}
                onChange={() => toggle("hw_transcoding")}
              />
              <SettingRow
                label="Quality Control"
                desc="Automatic bitrate selection or manual override"
              >
                <select
                  value={settings.quality_control || "auto"}
                  onChange={(e) =>
                    setSetting("quality_control", e.target.value)
                  }
                  className="cv-input text-xs min-w-[140px]"
                >
                  <option value="auto">Auto (Adaptive)</option>
                  <option value="max">Maximum Quality</option>
                  <option value="balanced">Balanced</option>
                  <option value="low">Low Bandwidth</option>
                </select>
              </SettingRow>

              <SectionHeader
                title="Auto-Save"
                desc="Settings persistence behavior"
              />
              <div className="cv-card p-3 flex items-center gap-3 bg-green-500/5 border border-green-500/10">
                <CheckCircle2 size={16} className="text-green-400 shrink-0" />
                <div>
                  <div
                    className="text-xs font-medium"
                    style={{ color: "var(--cv-text)" }}
                  >
                    Auto-Save Active
                  </div>
                  <div
                    className="text-[10px]"
                    style={{ color: "var(--cv-subtext)" }}
                  >
                    All settings, feature toggles, metadata provider selections,
                    and task schedules are automatically saved on exit and
                    restored on next startup. Auto-save runs every 60 seconds.
                  </div>
                </div>
              </div>
            </>
          )}

          {/* ═══ About ═══ */}
          {activeSection === "about" && (
            <>
              <div className="cv-card p-3">
                <img
                  src="/branding/cinavault-premium-banner.png"
                  alt="CinaVault 3.0 Media Server brand"
                  className="w-full rounded-lg border border-white/10"
                />
              </div>
              <div className="cv-card p-6 text-center">
                <div className="text-3xl mb-2">🎬</div>
                <h2
                  className="text-xl font-bold mb-1"
                  style={{ color: "var(--cv-text)" }}
                >
                  {BUILD_INFO.name}
                </h2>
                <p
                  className="text-sm font-medium mb-0.5"
                  style={{ color: "var(--cv-accent)" }}
                >
                  {BUILD_INFO.edition}
                </p>
                <p className="text-xs" style={{ color: "var(--cv-subtext)" }}>
                  {BUILD_INFO.displayName} · {BUILD_INFO.version} · Tauri v2 +
                  React 18
                </p>
                <div className="mt-4 grid grid-cols-3 gap-3 text-center">
                  <div className="p-3 rounded-xl bg-white/3">
                    <div
                      className="text-lg font-bold"
                      style={{ color: "var(--cv-accent)" }}
                    >
                      150+
                    </div>
                    <div
                      className="text-[10px]"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Plugins Available
                    </div>
                  </div>
                  <div className="p-3 rounded-xl bg-white/3">
                    <div
                      className="text-lg font-bold"
                      style={{ color: "var(--cv-accent)" }}
                    >
                      30+
                    </div>
                    <div
                      className="text-[10px]"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Metadata Providers
                    </div>
                  </div>
                  <div className="p-3 rounded-xl bg-white/3">
                    <div
                      className="text-lg font-bold"
                      style={{ color: "var(--cv-accent)" }}
                    >
                      {THEME_PRESETS.length}
                    </div>
                    <div
                      className="text-[10px]"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Theme / Skin Presets
                    </div>
                  </div>
                </div>
              </div>
              <div
                className="cv-card p-4 text-xs space-y-1"
                style={{ color: "var(--cv-subtext)" }}
              >
                <p>
                  <strong>Engine:</strong> Tauri v2 + Rust + React 18 + Vite 6
                </p>
                <p>
                  <strong>Player:</strong> Vidstack + MPV + VLC + System Default
                </p>
                <p>
                  <strong>Server:</strong> MS-C / MS-B compatible API server
                </p>
                <p>
                  <strong>Plugins:</strong> MS-C, MS-B, MS-A, and CinaVault
                  native
                </p>
                <p>
                  <strong>Cloud:</strong> OneDrive, Google Drive, Dropbox
                </p>
                <p>
                  <strong>Security:</strong> ClamAV, VPN, encrypted settings
                </p>
                <p>
                  <strong>AI:</strong> HuggingFace Transformers, local inference
                </p>
              </div>
            </>
          )}
        </motion.div>
      </div>
    </div>
  );
}

// ── Reusable Components ──

function SectionHeader({ title, desc }: { title: string; desc: string }) {
  return (
    <div className="pt-2 pb-1">
      <h3 className="text-sm font-bold" style={{ color: "var(--cv-text)" }}>
        {title}
      </h3>
      <p className="text-[10px]" style={{ color: "var(--cv-subtext)" }}>
        {desc}
      </p>
    </div>
  );
}

function SettingRow({
  label,
  desc,
  children,
}: {
  label: string;
  desc: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between p-3 rounded-xl border border-white/5 bg-white/3">
      <div>
        <div
          className="text-xs font-medium"
          style={{ color: "var(--cv-text)" }}
        >
          {label}
        </div>
        <div className="text-[10px]" style={{ color: "var(--cv-subtext)" }}>
          {desc}
        </div>
      </div>
      {children}
    </div>
  );
}

function ToggleRow({
  label,
  desc,
  checked,
  onChange,
}: {
  label: string;
  desc: string;
  checked: boolean;
  onChange: () => void;
}) {
  return (
    <button
      onClick={onChange}
      className={`w-full flex items-center justify-between p-3 rounded-xl border transition-all ${
        checked
          ? "border-[var(--cv-accent)]/30 bg-[var(--cv-accent)]/5"
          : "border-white/5 bg-white/3"
      } hover:bg-white/5`}
    >
      <div className="text-left">
        <div
          className="text-xs font-medium"
          style={{ color: "var(--cv-text)" }}
        >
          {label}
        </div>
        <div className="text-[10px]" style={{ color: "var(--cv-subtext)" }}>
          {desc}
        </div>
      </div>
      <div
        className={`w-10 h-5 rounded-full flex items-center transition-all ${
          checked
            ? "bg-[var(--cv-accent)] justify-end"
            : "bg-white/10 justify-start"
        }`}
      >
        <div className="w-4 h-4 rounded-full bg-white mx-0.5 shadow-sm" />
      </div>
    </button>
  );
}
