// CinaVault Premium — Advanced Tab (MS-B SDK Feature Matrix + Media Requests)
import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { useAppStore } from "../../store/appStore";
import {
  Sliders,
  Zap,
  Users,
  MessageSquare,
  ExternalLink,
  ChevronDown,
  ChevronRight,
} from "lucide-react";

interface FeatureCategory {
  name: string;
  features: { key: string; label: string; description?: string }[];
}

const FEATURE_MATRIX: FeatureCategory[] = [
  {
    name: "Playback & Experience",
    features: [
      {
        key: "cinema_mode",
        label: "Cinema Mode",
        description: "Dim UI during playback",
      },
      { key: "skip_intro", label: "Skip Intro Detection" },
      { key: "skip_credits", label: "Skip Credits Detection" },
      { key: "next_up", label: "Next Up Notification" },
      { key: "auto_resume", label: "Auto Resume Playback" },
      { key: "crossfade", label: "Audio Crossfade" },
      { key: "gapless", label: "Gapless Playback" },
    ],
  },
  {
    name: "UI & Customization",
    features: [
      { key: "glass_effects", label: "Glassmorphism Effects" },
      { key: "particle_bg", label: "Animated Particle Background" },
      { key: "custom_css", label: "Custom CSS Injection" },
      { key: "compact_mode", label: "Compact View Mode" },
      { key: "poster_hover", label: "Poster Hover Preview" },
      { key: "shelf_carousel", label: "Shelf Carousel Mode" },
    ],
  },
  {
    name: "Library & Metadata",
    features: [
      { key: "auto_metadata", label: "Auto Metadata Fetch" },
      { key: "smart_match", label: "Smart Title Matching" },
      { key: "nfo_import", label: "NFO File Import" },
      { key: "subtitle_fetch", label: "Auto Subtitle Download" },
      { key: "poster_sync", label: "Cloud Poster Sync" },
      { key: "chapter_thumbs", label: "Chapter Thumbnails" },
      { key: "collection_auto", label: "Auto Collections" },
    ],
  },
  {
    name: "User & Server Management",
    features: [
      { key: "user_profiles", label: "Multiple User Profiles" },
      { key: "parental_ctrl", label: "Parental Controls" },
      { key: "activity_log", label: "Activity Logging" },
      { key: "remote_access", label: "Remote Access" },
      { key: "api_keys", label: "API Key Management" },
      { key: "webhook", label: "Webhook Notifications" },
    ],
  },
  {
    name: "Performance & Connectivity",
    features: [
      { key: "hw_transcode", label: "Hardware Transcoding" },
      { key: "stream_buffer", label: "Stream Buffering Control" },
      { key: "bandwidth_limit", label: "Bandwidth Limiter" },
      { key: "cdn_cache", label: "CDN / Cache Layer" },
      { key: "direct_play", label: "Force Direct Play" },
      { key: "gpu_accel", label: "GPU Acceleration" },
    ],
  },
  {
    name: "Library & Discovery",
    features: [
      { key: "trending", label: "Trending Content" },
      { key: "recommendations", label: "AI Recommendations" },
      { key: "similar_titles", label: "Similar Titles" },
      { key: "genre_radio", label: "Genre Radio Stations" },
      { key: "watchlist", label: "Watchlist Management" },
      { key: "continue_watching", label: "Continue Watching" },
      { key: "new_releases", label: "New Releases Alerts" },
    ],
  },
];

export default function AdvancedTab() {
  const { featureSettings, toggleFeature, addStatusMessage } = useAppStore();
  const [expandedCats, setExpandedCats] = useState<Set<string>>(
    new Set(FEATURE_MATRIX.map((c) => c.name)),
  );
  const [requestQueue, setRequestQueue] = useState<any[]>([]);
  const [newRequest, setNewRequest] = useState({
    title: "",
    type: "movie",
    requester: "",
  });

  const toggleCat = (name: string) => {
    setExpandedCats((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const handleToggle = async (key: string) => {
    const enabled = !(featureSettings[key]?.enabled || false);
    try {
      await invoke("set_feature_setting", { key, enabled, config: "{}" });
      toggleFeature(key);
      addStatusMessage(`Feature ${key}: ${enabled ? "enabled" : "disabled"}`);
    } catch (error) {
      addStatusMessage(`Feature ${key}: update failed — ${String(error)}`);
    }
  };

  const addRequest = () => {
    if (!newRequest.title) return;
    setRequestQueue((prev) => [
      {
        ...newRequest,
        id: Date.now(),
        status: "pending",
        created: new Date().toLocaleString(),
      },
      ...prev,
    ]);
    setNewRequest({ title: "", type: "movie", requester: "" });
    addStatusMessage(`Request added: ${newRequest.title}`);
  };

  return (
    <div className="space-y-5">
      {/* Feature Matrix */}
      <div className="glass-panel p-5">
        <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
          <Zap size={16} className="text-cv-accent" /> MS-B SDK Feature Matrix
        </h3>
        <div className="space-y-2">
          {FEATURE_MATRIX.map((cat) => (
            <div
              key={cat.name}
              className="glass-panel-2 rounded-lg overflow-hidden"
            >
              <button
                onClick={() => toggleCat(cat.name)}
                className="w-full px-4 py-3 flex items-center justify-between hover:bg-white/[0.03] transition-colors"
              >
                <span className="text-xs font-bold uppercase tracking-wider text-cv-accent">
                  {cat.name}
                </span>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-cv-subtext">
                    {
                      cat.features.filter(
                        (f) => featureSettings[f.key]?.enabled,
                      ).length
                    }
                    /{cat.features.length}
                  </span>
                  {expandedCats.has(cat.name) ? (
                    <ChevronDown size={14} />
                  ) : (
                    <ChevronRight size={14} />
                  )}
                </div>
              </button>
              {expandedCats.has(cat.name) && (
                <motion.div
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: "auto" }}
                  className="px-4 pb-3 space-y-1"
                >
                  {cat.features.map((feature) => (
                    <div
                      key={feature.key}
                      className="flex items-center justify-between py-1.5 px-2 rounded hover:bg-white/[0.02]"
                    >
                      <div className="flex-1 min-w-0">
                        <div className="text-sm">{feature.label}</div>
                        {feature.description && (
                          <div className="text-[10px] text-cv-subtext">
                            {feature.description}
                          </div>
                        )}
                      </div>
                      <div
                        className={`cv-toggle ${featureSettings[feature.key]?.enabled ? "active" : ""}`}
                        onClick={() => handleToggle(feature.key)}
                      />
                    </div>
                  ))}
                </motion.div>
              )}
            </div>
          ))}
        </div>
      </div>

      {/* Media Requests & Automation */}
      <div className="glass-panel p-5">
        <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
          <MessageSquare size={16} className="text-cv-accent" /> Media Requests
          & Automation
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          {/* Request Queue */}
          <div>
            <label className="section-label">New Request</label>
            <div className="space-y-2 mb-3">
              <input
                value={newRequest.title}
                onChange={(e) =>
                  setNewRequest({ ...newRequest, title: e.target.value })
                }
                className="cv-input"
                placeholder="Title to request..."
              />
              <div className="flex gap-2">
                <select
                  value={newRequest.type}
                  onChange={(e) =>
                    setNewRequest({ ...newRequest, type: e.target.value })
                  }
                  className="cv-select flex-1"
                >
                  <option value="movie">Movie</option>
                  <option value="tvshow">TV Show</option>
                  <option value="music">Music</option>
                </select>
                <input
                  value={newRequest.requester}
                  onChange={(e) =>
                    setNewRequest({ ...newRequest, requester: e.target.value })
                  }
                  className="cv-input flex-1"
                  placeholder="Requester"
                />
              </div>
              <button
                onClick={addRequest}
                className="cv-btn cv-btn-primary text-xs w-full"
              >
                Add Request
              </button>
            </div>

            {requestQueue.length > 0 && (
              <div className="glass-panel-2 rounded-lg max-h-48 overflow-y-auto divide-y divide-white/5">
                {requestQueue.map((req) => (
                  <div key={req.id} className="px-3 py-2 text-xs">
                    <div className="font-semibold">{req.title}</div>
                    <div className="text-cv-subtext">
                      {req.type} - {req.status} - {req.requester || "Anonymous"}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Integrations */}
          <div>
            <label className="section-label">Integrations</label>
            <div className="space-y-2">
              {[
                {
                  name: "Overseerr",
                  desc: "Media request management",
                  url: "https://overseerr.dev",
                },
                {
                  name: "MS-C Requests",
                  desc: "MS-C request management",
                  url: "https://github.com/Fallenbagel/MS-C Requests",
                },
              ].map((int) => (
                <div
                  key={int.name}
                  className="glass-panel-2 p-3 rounded-lg flex items-center justify-between"
                >
                  <div>
                    <div className="text-sm font-semibold">{int.name}</div>
                    <div className="text-[10px] text-cv-subtext">
                      {int.desc}
                    </div>
                  </div>
                  <button
                    onClick={() => window.open?.(int.url)}
                    className="cv-btn cv-btn-secondary text-[10px] py-1 px-2"
                  >
                    <ExternalLink size={10} /> Open
                  </button>
                </div>
              ))}
            </div>

            <label className="section-label mt-4">User Groups & Sharing</label>
            <div className="glass-panel-2 p-4 rounded-lg">
              <div className="flex items-center gap-3 mb-3">
                <Users size={20} className="text-cv-accent" />
                <div>
                  <div className="text-sm font-semibold">User Groups</div>
                  <div className="text-[10px] text-cv-subtext">
                    Manage permissions per group
                  </div>
                </div>
              </div>
              <div className="space-y-1">
                {["Admin", "Family", "Friends", "Kids"].map((group) => (
                  <div
                    key={group}
                    className="flex items-center justify-between py-1.5 px-2 rounded bg-white/[0.02]"
                  >
                    <span className="text-xs">{group}</span>
                    <span className="text-[10px] text-cv-subtext">
                      Full Access
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
