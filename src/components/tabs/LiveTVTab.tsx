// CinaVault Premium — Live TV Tab (Xtream Codes / IPTV)
import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { useAppStore } from "../../store/appStore";
import {
  Tv,
  Plus,
  Trash2,
  RefreshCw,
  Play,
  Radio,
  Wifi,
  Globe,
  List,
  Search,
  Calendar,
  ArrowLeft,
} from "lucide-react";
import { buildAddXtreamProfileArgs } from "../../utils/xtreamProfile";
import IPTVPlayer from "../IPTVPlayer";

export default function LiveTVTab() {
  const { addStatusMessage } = useAppStore();
  const [profiles, setProfiles] = useState<any[]>([]);
  const [channels, setChannels] = useState<any[]>([]);
  const [selectedProfile, setSelectedProfile] = useState<number | null>(null);
  const [channelSearch, setChannelSearch] = useState("");
  const [showAddProfile, setShowAddProfile] = useState(false);
  const [newProfile, setNewProfile] = useState({
    name: "",
    server_url: "",
    username: "",
    password: "",
  });
  const [selectedChannel, setSelectedChannel] = useState<any | null>(null);

  useEffect(() => {
    loadProfiles();
  }, []);

  const loadProfiles = async () => {
    try {
      const p = await invoke<any[]>("get_xtream_profiles");
      setProfiles(p);
    } catch {
      setProfiles(DEMO_PROFILES);
    }
  };

  const addProfile = async () => {
    try {
      const profileArgs = buildAddXtreamProfileArgs(newProfile);
      await invoke("add_xtream_profile", profileArgs);
      addStatusMessage(`IPTV profile added: ${profileArgs.name}`);
      setNewProfile({ name: "", server_url: "", username: "", password: "" });
      setShowAddProfile(false);
      loadProfiles();
    } catch (e) {
      addStatusMessage(`Failed: ${e}`);
    }
  };

  const removeProfile = async (id: number) => {
    try {
      await invoke("remove_xtream_profile", { id });
      addStatusMessage("Profile removed");
      loadProfiles();
      if (selectedProfile === id) {
        setSelectedProfile(null);
        setChannels([]);
        setSelectedChannel(null);
      }
    } catch (e) {
      addStatusMessage(`Failed: ${e}`);
    }
  };

  const syncStreams = async (id: number) => {
    addStatusMessage("Syncing streams...");
    try {
      const result = await invoke<any>("sync_xtream_streams", {
        profileId: id,
      });
      addStatusMessage(`Synced ${result.channels_synced} channels`);
      loadChannels(id);
    } catch (e) {
      addStatusMessage(`Sync failed: ${e}`);
    }
  };

  const syncEpg = async (id: number) => {
    addStatusMessage("Syncing EPG data...");
    try {
      await invoke("sync_epg", { profileId: id });
      addStatusMessage("EPG synced successfully");
    } catch (e) {
      addStatusMessage(`EPG sync failed: ${e}`);
    }
  };

  const loadChannels = async (profileId: number) => {
    setSelectedProfile(profileId);
    setSelectedChannel(null);
    try {
      const ch = await invoke<any[]>("get_live_channels", { profileId });
      setChannels(ch);
    } catch {
      setChannels(DEMO_CHANNELS);
    }
  };

  const filteredChannels = channels.filter(
    (ch) =>
      !channelSearch ||
      ch.name.toLowerCase().includes(channelSearch.toLowerCase()),
  );

  const groupedChannels = filteredChannels.reduce(
    (acc: Record<string, any[]>, ch) => {
      const group = ch.group_name || "Uncategorized";
      if (!acc[group]) acc[group] = [];
      acc[group].push(ch);
      return acc;
    },
    {},
  );

  return (
    <div className="space-y-5">
      {/* Xtream Profiles */}
      <div className="glass-panel p-5">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-bold flex items-center gap-2">
            <Radio size={16} className="text-cv-accent" /> Xtream Codes Profiles
          </h3>
          <button
            onClick={() => setShowAddProfile(!showAddProfile)}
            className="cv-btn cv-btn-primary text-xs"
          >
            <Plus size={12} /> Add Profile
          </button>
        </div>

        {showAddProfile && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            className="mb-4 glass-panel-2 p-4 rounded-lg"
          >
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="section-label">Profile Name</label>
                <input
                  value={newProfile.name}
                  onChange={(e) =>
                    setNewProfile({ ...newProfile, name: e.target.value })
                  }
                  className="cv-input"
                  placeholder="My IPTV"
                />
              </div>
              <div>
                <label className="section-label">Server URL</label>
                <input
                  value={newProfile.server_url}
                  onChange={(e) =>
                    setNewProfile({ ...newProfile, server_url: e.target.value })
                  }
                  className="cv-input"
                  placeholder="http://provider.com:8080"
                />
              </div>
              <div>
                <label className="section-label">Username</label>
                <input
                  value={newProfile.username}
                  onChange={(e) =>
                    setNewProfile({ ...newProfile, username: e.target.value })
                  }
                  className="cv-input"
                />
              </div>
              <div>
                <label className="section-label">Password</label>
                <input
                  type="password"
                  value={newProfile.password}
                  onChange={(e) =>
                    setNewProfile({ ...newProfile, password: e.target.value })
                  }
                  className="cv-input"
                />
              </div>
            </div>
            <div className="flex gap-2 mt-3">
              <button
                onClick={addProfile}
                className="cv-btn cv-btn-primary text-xs"
              >
                <Plus size={12} /> Save
              </button>
              <button
                onClick={() => setShowAddProfile(false)}
                className="cv-btn cv-btn-secondary text-xs"
              >
                Cancel
              </button>
            </div>
          </motion.div>
        )}

        {profiles.length === 0 ? (
          <div className="text-center py-6 text-cv-subtext text-sm">
            No IPTV profiles configured
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {profiles.map((p) => (
              <div
                key={p.id}
                className={`glass-panel-2 p-4 rounded-lg cursor-pointer transition-all ${selectedProfile === p.id ? "ring-1 ring-cv-accent" : "hover:bg-white/5"}`}
                onClick={() => loadChannels(p.id)}
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm font-semibold">{p.name}</span>
                  <span
                    className={`status-dot ${p.enabled ? "online" : "offline"}`}
                  />
                </div>
                <div className="text-[10px] text-cv-subtext mb-3 truncate">
                  {p.server_url}
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      syncStreams(p.id);
                    }}
                    className="cv-btn cv-btn-secondary text-[10px] py-1 px-2"
                  >
                    <RefreshCw size={10} /> Sync
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      syncEpg(p.id);
                    }}
                    className="cv-btn cv-btn-secondary text-[10px] py-1 px-2"
                  >
                    <Calendar size={10} /> EPG
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      removeProfile(p.id);
                    }}
                    className="cv-btn cv-btn-danger text-[10px] py-1 px-2"
                  >
                    <Trash2 size={10} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Channel List or Player */}
      {selectedProfile && (
        <div className="glass-panel rounded-xl overflow-hidden">
          {selectedChannel ? (
            <div className="flex flex-col h-full">
              <div className="px-5 py-3 border-b border-white/5 flex items-center justify-between">
                <button
                  onClick={() => setSelectedChannel(null)}
                  className="text-cv-accent hover:text-cv-accent/80 transition-colors"
                >
                  <ArrowLeft size={16} /> Back
                </button>
                <h3 className="text-sm font-bold truncate max-w-xs">
                  {selectedChannel.name}
                </h3>
              </div>
              <div className="flex-1">
                <IPTVPlayer streamUrl={selectedChannel.stream_url} />
              </div>
            </div>
          ) : (
            <>
              <div className="px-5 py-3 border-b border-white/5 flex items-center justify-between">
                <h3 className="text-sm font-bold">
                  {channels.length} Channels
                </h3>
                <div className="relative w-48">
                  <Search
                    size={12}
                    className="absolute left-3 top-1/2 -translate-y-1/2 text-cv-subtext"
                  />
                  <input
                    value={channelSearch}
                    onChange={(e) => setChannelSearch(e.target.value)}
                    className="cv-input pl-8 text-xs py-1.5"
                    placeholder="Search channels..."
                  />
                </div>
              </div>
              <div className="max-h-[calc(100vh-420px)] overflow-y-auto">
                {Object.entries(groupedChannels).map(([group, chs]) => (
                  <div key={group}>
                    <div className="px-5 py-2 bg-white/[0.02] text-[10px] font-bold uppercase tracking-wider text-cv-accent">
                      {group} ({chs.length})
                    </div>
                    {chs.map((ch: any, i: number) => (
                      <div
                        key={ch.id || i}
                        className="px-5 py-2 flex items-center gap-3 zebra-row cursor-pointer"
                        onClick={() => setSelectedChannel(ch)}
                      >
                        <div className="w-8 h-8 rounded bg-white/5 flex items-center justify-center shrink-0">
                          {ch.logo_url ? (
                            <img
                              src={ch.logo_url}
                              className="w-6 h-6 object-contain"
                            />
                          ) : (
                            <Tv size={14} className="text-cv-subtext" />
                          )}
                        </div>
                        <span className="text-sm flex-1 truncate">
                          {ch.name}
                        </span>
                        <button className="opacity-0 group-hover:opacity-100 cv-btn cv-btn-primary text-[10px] py-1 px-2">
                          <Play size={10} /> Play
                        </button>
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}

const DEMO_PROFILES = [
  {
    id: 1,
    name: "Premium IPTV",
    server_url: "http://iptv.example.com:8080",
    username: "user",
    password: "pass",
    enabled: true,
    last_synced: null,
  },
];
const DEMO_CHANNELS = [
  {
    id: 1,
    profile_id: 1,
    name: "CNN",
    stream_url: "",
    logo_url: null,
    group_name: "News",
    epg_id: "cnn",
  },
  {
    id: 2,
    profile_id: 1,
    name: "ESPN",
    stream_url: "",
    logo_url: null,
    group_name: "Sports",
    epg_id: "espn",
  },
  {
    id: 3,
    profile_id: 1,
    name: "HBO",
    stream_url: "",
    logo_url: null,
    group_name: "Entertainment",
    epg_id: "hbo",
  },
];
