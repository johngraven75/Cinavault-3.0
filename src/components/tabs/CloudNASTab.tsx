// CinaVault Premium — Cloud & NAS Tab (Build 155)
// OneDrive + Google Drive + Dropbox + Synology QuickConnect + WD My Cloud
import React, { useState, useCallback, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore, CloudServiceStatus } from "../../store/appStore";
import {
  Cloud,
  HardDrive,
  FolderOpen,
  RefreshCw,
  Plus,
  Trash2,
  CheckCircle2,
  XCircle,
  Wifi,
  WifiOff,
  Link2,
  LogIn,
  LogOut,
  Server,
  Database,
  ChevronDown,
  ChevronUp,
  Shield,
  AlertTriangle,
} from "lucide-react";

// ── OAuth endpoints ──
const ONEDRIVE_AUTH_URL =
  "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const ONEDRIVE_CLIENT_ID = "cinavault-onedrive-client";
const ONEDRIVE_SCOPES = "Files.ReadWrite.All offline_access";
const ONEDRIVE_REDIRECT = "http://localhost:19284/auth/callback";
const GDRIVE_AUTH_URL = "https://accounts.google.com/o/oauth2/v2/auth";
const GDRIVE_CLIENT_ID = "cinavault-gdrive-client";
const GDRIVE_SCOPES = "https://www.googleapis.com/auth/drive.readonly";
const GDRIVE_REDIRECT = "http://localhost:19284/auth/callback";
const DROPBOX_AUTH_URL = "https://www.dropbox.com/oauth2/authorize";

type CloudId = "onedrive" | "gdrive" | "dropbox";

interface NasLibrary {
  id: string;
  name: string;
  path: string;
  share_name: string;
  media_type: string;
  item_count: number;
  size_bytes: number;
}

interface NasConnectionResult {
  success: boolean;
  device_name: string;
  device_model: string;
  firmware: string;
  host_resolved: string;
  libraries: NasLibrary[];
  error?: string;
}

interface NasProfile {
  id: string;
  name: string;
  protocol: string;
  host: string;
  port: number;
  path: string;
  username: string;
  status: "connected" | "disconnected" | "error";
}

const STATUS_COLORS: Record<
  CloudServiceStatus,
  { bg: string; text: string; label: string }
> = {
  connected: {
    bg: "rgba(34,197,94,0.15)",
    text: "#22c55e",
    label: "Connected",
  },
  disconnected: {
    bg: "rgba(156,163,175,0.15)",
    text: "#9ca3af",
    label: "Disconnected",
  },
  connecting: {
    bg: "rgba(251,191,36,0.15)",
    text: "#fbbf24",
    label: "Connecting...",
  },
  error: { bg: "rgba(239,68,68,0.15)", text: "#ef4444", label: "Error" },
};

function formatBytes(bytes: number): string {
  if (bytes === 0) return "—";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

export default function CloudNASTab() {
  const { cloudServices, setCloudService, addStatusMessage } = useAppStore();

  // ── Generic NAS (SMB/NFS/etc.) ──
  const [nasProfiles, setNasProfiles] = useState<NasProfile[]>([]);
  const [showAddNas, setShowAddNas] = useState(false);
  const [nasForm, setNasForm] = useState({
    name: "",
    protocol: "smb",
    host: "",
    port: 445,
    path: "",
    username: "",
    password: "",
  });

  // ── Synology QuickConnect ──
  const [synoConnected, setSynoConnected] = useState(false);
  const [synoInfo, setSynoInfo] = useState<NasConnectionResult | null>(null);
  const [synoConnecting, setSynoConnecting] = useState(false);
  const [showSynoForm, setShowSynoForm] = useState(false);
  const [synoForm, setSynoForm] = useState({
    quickconnect_id: "",
    username: "admin",
    password: "",
    use_https: true,
    port: "",
  });
  const [synoError, setSynoError] = useState<string | null>(null);
  const [synoLibrariesExpanded, setSynoLibrariesExpanded] = useState(true);

  // ── WD My Cloud ──
  const [wdConnected, setWdConnected] = useState(false);
  const [wdInfo, setWdInfo] = useState<NasConnectionResult | null>(null);
  const [wdConnecting, setWdConnecting] = useState(false);
  const [showWdForm, setShowWdForm] = useState(false);
  const [wdForm, setWdForm] = useState({
    host: "",
    username: "admin",
    password: "",
    use_https: false,
    port: "",
  });
  const [wdError, setWdError] = useState<string | null>(null);
  const [wdLibrariesExpanded, setWdLibrariesExpanded] = useState(true);

  // ── Load persisted NAS status on mount ──
  useEffect(() => {
    invoke<{ connected: boolean; data?: any }>("synology_get_status")
      .then((res) => {
        if (res.connected && res.data) {
          setSynoConnected(true);
          setSynoInfo({
            success: true,
            device_name: res.data.device_name || "Synology NAS",
            device_model: res.data.device_model || "",
            firmware: res.data.firmware || "",
            host_resolved: res.data.host || "",
            libraries: res.data.libraries || [],
          });
        }
      })
      .catch(() => {});

    invoke<{ connected: boolean; data?: any }>("wd_mycloud_get_status")
      .then((res) => {
        if (res.connected && res.data) {
          setWdConnected(true);
          setWdInfo({
            success: true,
            device_name: res.data.device_name || "WD My Cloud",
            device_model: res.data.device_model || "",
            firmware: res.data.firmware || "",
            host_resolved: res.data.host || "",
            libraries: res.data.libraries || [],
          });
        }
      })
      .catch(() => {});
  }, []);

  // ════════════════════════════════════════════════════════════
  //  Cloud OAuth handlers
  // ════════════════════════════════════════════════════════════
  const backendProvider = (id: CloudId): string =>
    id === "gdrive" ? "googledrive" : id;
  const cloudName = (id: CloudId): string =>
    id === "gdrive"
      ? "Google Drive"
      : id === "onedrive"
        ? "OneDrive"
        : "Dropbox";
  const errorText = (error: unknown): string =>
    error instanceof Error ? error.message : String(error);

  const authenticateCloud = useCallback(
    async (id: CloudId, authUrl: string) => {
      const name = cloudName(id);
      setCloudService(id, { status: "connecting" });
      addStatusMessage(`${name}: Starting authentication...`);
      try {
        const result = await invoke<{
          success: boolean;
          account?: string;
          error?: string;
        }>("cloud_auth_start", {
          provider: backendProvider(id),
          authUrl,
        });
        if (!result.success) {
          throw new Error(result.error || "Authentication failed");
        }
        setCloudService(id, {
          status: "connected",
          account: result.account || `${name} Account`,
          lastSync: new Date().toISOString(),
        });
        addStatusMessage(`${name}: Connected as ${result.account || "user"}`);
      } catch (error) {
        setCloudService(id, { status: "error" });
        addStatusMessage(`${name}: Connection failed — ${errorText(error)}`);
      }
    },
    [setCloudService, addStatusMessage],
  );

  const connectOneDrive = useCallback(async () => {
    const params = new URLSearchParams({
      client_id: ONEDRIVE_CLIENT_ID,
      response_type: "code",
      redirect_uri: ONEDRIVE_REDIRECT,
      scope: ONEDRIVE_SCOPES,
      response_mode: "query",
    });
    await authenticateCloud("onedrive", `${ONEDRIVE_AUTH_URL}?${params}`);
  }, [authenticateCloud]);

  const connectGDrive = useCallback(async () => {
    const params = new URLSearchParams({
      client_id: GDRIVE_CLIENT_ID,
      response_type: "code",
      redirect_uri: GDRIVE_REDIRECT,
      scope: GDRIVE_SCOPES,
      access_type: "offline",
      prompt: "consent",
    });
    await authenticateCloud("gdrive", `${GDRIVE_AUTH_URL}?${params}`);
  }, [authenticateCloud]);

  const connectDropbox = useCallback(async () => {
    await authenticateCloud("dropbox", DROPBOX_AUTH_URL);
  }, [authenticateCloud]);

  const disconnect = useCallback(
    async (id: CloudId) => {
      const name = cloudName(id);
      try {
        await invoke("cloud_disconnect", { provider: backendProvider(id) });
        setCloudService(id, {
          status: "disconnected",
          account: undefined,
          lastSync: undefined,
        });
        addStatusMessage(`${name}: Disconnected`);
      } catch (error) {
        setCloudService(id, { status: "error" });
        addStatusMessage(`${name}: Disconnect failed — ${errorText(error)}`);
      }
    },
    [setCloudService, addStatusMessage],
  );

  const syncCloud = useCallback(
    async (id: CloudId) => {
      const name = cloudName(id);
      addStatusMessage(`${name}: Syncing...`);
      try {
        const result = await invoke<{ files_synced?: number }>("cloud_sync", {
          provider: backendProvider(id),
          path: "",
        });
        setCloudService(id, { lastSync: new Date().toISOString() });
        addStatusMessage(
          `${name}: Sync complete${typeof result?.files_synced === "number" ? ` (${result.files_synced} files)` : ""}`,
        );
      } catch (error) {
        setCloudService(id, { status: "error" });
        addStatusMessage(`${name}: Sync failed — ${errorText(error)}`);
      }
    },
    [setCloudService, addStatusMessage],
  );

  const browseCloud = useCallback(
    async (id: CloudId) => {
      const name = cloudName(id);
      addStatusMessage(`${name}: Browsing media library...`);
      try {
        const result = await invoke<unknown[]>("cloud_browse", {
          provider: backendProvider(id),
          path: "",
        });
        addStatusMessage(
          `${name}: Found ${Array.isArray(result) ? result.length : 0} item(s)`,
        );
      } catch (error) {
        setCloudService(id, { status: "error" });
        addStatusMessage(`${name}: Browse failed — ${errorText(error)}`);
      }
    },
    [setCloudService, addStatusMessage],
  );

  // ════════════════════════════════════════════════════════════
  //  Synology QuickConnect
  // ════════════════════════════════════════════════════════════
  const connectSynology = async () => {
    if (!synoForm.quickconnect_id || !synoForm.username || !synoForm.password) {
      setSynoError("QuickConnect ID, username, and password are required.");
      return;
    }
    setSynoConnecting(true);
    setSynoError(null);
    addStatusMessage(`Synology: Connecting to ${synoForm.quickconnect_id}...`);
    try {
      const result = await invoke<NasConnectionResult>("synology_connect", {
        quickconnectId: synoForm.quickconnect_id.trim(),
        username: synoForm.username,
        password: synoForm.password,
        useHttps: synoForm.use_https,
        port: synoForm.port ? parseInt(synoForm.port) : null,
      });
      setSynoConnected(true);
      setSynoInfo(result);
      setShowSynoForm(false);
      addStatusMessage(
        `Synology: Connected to ${result.device_name} (${result.device_model}) — ${result.libraries.length} share(s) found`,
      );
    } catch (err: any) {
      setSynoError(err?.toString() || "Connection failed");
      addStatusMessage(`Synology: Connection failed — ${err}`);
    } finally {
      setSynoConnecting(false);
    }
  };

  const disconnectSynology = async () => {
    try {
      await invoke("synology_disconnect");
      setSynoConnected(false);
      setSynoInfo(null);
      setSynoError(null);
      addStatusMessage("Synology: Disconnected");
    } catch (error) {
      setSynoError(errorText(error));
      addStatusMessage(`Synology: Disconnect failed — ${errorText(error)}`);
    }
  };

  const addSynoLibrary = async (lib: NasLibrary) => {
    try {
      await invoke("synology_add_library", {
        shareName: lib.name,
        sharePath: lib.path,
        mediaType: lib.media_type,
      });
      addStatusMessage(`Synology: Added "${lib.name}" as a media source`);
    } catch (err: any) {
      addStatusMessage(`Synology: Failed to add "${lib.name}" — ${err}`);
    }
  };

  // ════════════════════════════════════════════════════════════
  //  WD My Cloud
  // ════════════════════════════════════════════════════════════
  const connectWdMyCloud = async () => {
    if (!wdForm.host || !wdForm.username || !wdForm.password) {
      setWdError("Host/IP, username, and password are required.");
      return;
    }
    setWdConnecting(true);
    setWdError(null);
    addStatusMessage(`WD My Cloud: Connecting to ${wdForm.host}...`);
    try {
      const result = await invoke<NasConnectionResult>("wd_mycloud_connect", {
        host: wdForm.host.trim(),
        username: wdForm.username,
        password: wdForm.password,
        useHttps: wdForm.use_https,
        port: wdForm.port ? parseInt(wdForm.port) : null,
      });
      setWdConnected(true);
      setWdInfo(result);
      setShowWdForm(false);
      addStatusMessage(
        `WD My Cloud: Connected to ${result.device_name} — ${result.libraries.length} share(s) found`,
      );
    } catch (err: any) {
      setWdError(err?.toString() || "Connection failed");
      addStatusMessage(`WD My Cloud: Connection failed — ${err}`);
    } finally {
      setWdConnecting(false);
    }
  };

  const disconnectWdMyCloud = async () => {
    try {
      await invoke("wd_mycloud_disconnect");
      setWdConnected(false);
      setWdInfo(null);
      setWdError(null);
      addStatusMessage("WD My Cloud: Disconnected");
    } catch (error) {
      setWdError(errorText(error));
      addStatusMessage(`WD My Cloud: Disconnect failed — ${errorText(error)}`);
    }
  };

  const addWdLibrary = async (lib: NasLibrary) => {
    try {
      await invoke("wd_mycloud_add_library", {
        shareName: lib.name,
        sharePath: lib.path,
        mediaType: lib.media_type,
      });
      addStatusMessage(`WD My Cloud: Added "${lib.name}" as a media source`);
    } catch (err: any) {
      addStatusMessage(`WD My Cloud: Failed to add "${lib.name}" — ${err}`);
    }
  };

  // ── Generic NAS ──
  const addNas = async () => {
    const profile: NasProfile = {
      id: `nas-${Date.now()}`,
      name: nasForm.name || `NAS ${nasProfiles.length + 1}`,
      protocol: nasForm.protocol,
      host: nasForm.host,
      port: nasForm.port,
      path: nasForm.path,
      username: nasForm.username,
      status: "disconnected",
    };
    try {
      await invoke("add_source", {
        path: `${nasForm.protocol}://${nasForm.username}@${nasForm.host}:${nasForm.port}${nasForm.path}`,
        sourceType: "nas",
        name: profile.name,
      });
      profile.status = "connected";
    } catch (error) {
      profile.status = "error";
      addStatusMessage(`NAS source could not be added — ${errorText(error)}`);
    }
    setNasProfiles((prev) => [...prev, profile]);
    setShowAddNas(false);
    setNasForm({
      name: "",
      protocol: "smb",
      host: "",
      port: 445,
      path: "",
      username: "",
      password: "",
    });
    addStatusMessage(`NAS added: ${profile.name}`);
  };

  const CLOUD_SERVICES: {
    id: CloudId;
    name: string;
    icon: string;
    desc: string;
    connect: () => void;
  }[] = [
    {
      id: "onedrive",
      name: "Microsoft OneDrive",
      icon: "☁️",
      desc: "Connect your OneDrive for cloud media access and backup",
      connect: connectOneDrive,
    },
    {
      id: "gdrive",
      name: "Google Drive",
      icon: "📁",
      desc: "Stream and manage media from your Google Drive storage",
      connect: connectGDrive,
    },
    {
      id: "dropbox",
      name: "Dropbox",
      icon: "📦",
      desc: "Access Dropbox-stored media files and folders",
      connect: connectDropbox,
    },
  ];

  return (
    <div className="space-y-5">
      {/* ── Cloud Storage ── */}
      <div className="cv-card p-4">
        <div className="flex items-center gap-2 mb-4">
          <Cloud size={18} style={{ color: "var(--cv-accent)" }} />
          <h3
            className="text-base font-bold"
            style={{ color: "var(--cv-text)" }}
          >
            Cloud Storage
          </h3>
          <span className="text-[10px] px-2 py-0.5 rounded-full bg-white/10 text-[var(--cv-subtext)]">
            {
              Object.values(cloudServices).filter(
                (s) => s.status === "connected",
              ).length
            }{" "}
            connected
          </span>
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-3">
          {CLOUD_SERVICES.map((svc) => {
            const state = cloudServices[svc.id];
            const statusInfo = STATUS_COLORS[state?.status || "disconnected"];
            const isConnected = state?.status === "connected";
            const isConnecting = state?.status === "connecting";
            return (
              <motion.div
                key={svc.id}
                className="p-4 rounded-xl border border-white/5 bg-white/3 hover:bg-white/5 transition-all"
                whileHover={{ scale: 1.01 }}
              >
                <div className="flex items-center gap-3 mb-3">
                  <div className="w-10 h-10 rounded-xl flex items-center justify-center text-xl bg-white/5">
                    {svc.icon}
                  </div>
                  <div className="flex-1">
                    <div
                      className="text-sm font-semibold"
                      style={{ color: "var(--cv-text)" }}
                    >
                      {svc.name}
                    </div>
                    <div className="flex items-center gap-1.5 mt-0.5">
                      <span
                        className="w-2 h-2 rounded-full"
                        style={{ background: statusInfo.text }}
                      />
                      <span
                        className="text-[10px] font-medium"
                        style={{ color: statusInfo.text }}
                      >
                        {statusInfo.label}
                      </span>
                    </div>
                  </div>
                </div>
                <p
                  className="text-[11px] mb-3"
                  style={{ color: "var(--cv-subtext)" }}
                >
                  {svc.desc}
                </p>
                {isConnected && state?.account && (
                  <div
                    className="text-[10px] mb-2 px-2 py-1.5 rounded-lg bg-white/5"
                    style={{ color: "var(--cv-subtext)" }}
                  >
                    <span className="font-medium">Account:</span>{" "}
                    {state.account}
                    {state.lastSync && (
                      <span className="ml-2">
                        · Synced: {new Date(state.lastSync).toLocaleString()}
                      </span>
                    )}
                  </div>
                )}
                <div className="flex items-center gap-2 mt-2">
                  {!isConnected ? (
                    <button
                      onClick={svc.connect}
                      disabled={isConnecting}
                      className="cv-btn text-xs py-2 flex-1 flex items-center justify-center gap-1.5 disabled:opacity-50"
                    >
                      {isConnecting ? (
                        <RefreshCw size={12} className="animate-spin" />
                      ) : (
                        <LogIn size={12} />
                      )}
                      {isConnecting ? "Connecting..." : "Connect"}
                    </button>
                  ) : (
                    <>
                      <button
                        onClick={() => browseCloud(svc.id)}
                        className="cv-btn text-[11px] py-2 flex-1 flex items-center justify-center gap-1"
                      >
                        <FolderOpen size={11} /> Browse
                      </button>
                      <button
                        onClick={() => syncCloud(svc.id)}
                        className="cv-btn text-[11px] py-2 flex items-center justify-center gap-1"
                      >
                        <RefreshCw size={11} /> Sync
                      </button>
                      <button
                        onClick={() => disconnect(svc.id)}
                        className="w-8 h-8 rounded-lg flex items-center justify-center bg-red-500/10 hover:bg-red-500/20 transition-colors"
                      >
                        <LogOut size={13} className="text-red-400" />
                      </button>
                    </>
                  )}
                </div>
              </motion.div>
            );
          })}
        </div>
      </div>

      {/* ══════════════════════════════════════════════════════
           Synology QuickConnect
      ══════════════════════════════════════════════════════ */}
      <div className="cv-card p-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Server size={18} style={{ color: "#00b4d8" }} />
            <h3
              className="text-base font-bold"
              style={{ color: "var(--cv-text)" }}
            >
              Synology NAS
            </h3>
            <span
              className="text-[10px] px-2 py-0.5 rounded-full"
              style={{
                background: synoConnected
                  ? "rgba(34,197,94,0.15)"
                  : "rgba(156,163,175,0.15)",
                color: synoConnected ? "#22c55e" : "#9ca3af",
              }}
            >
              {synoConnected ? "Connected" : "Disconnected"}
            </span>
            <span className="text-[10px] px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-400">
              QuickConnect
            </span>
          </div>
          {!synoConnected ? (
            <button
              onClick={() => setShowSynoForm(!showSynoForm)}
              className="cv-btn text-xs flex items-center gap-1"
            >
              <Link2 size={12} /> {showSynoForm ? "Cancel" : "Connect"}
            </button>
          ) : (
            <button
              onClick={disconnectSynology}
              className="cv-btn text-xs flex items-center gap-1 bg-red-500/10 hover:bg-red-500/20 text-red-400"
            >
              <LogOut size={12} /> Disconnect
            </button>
          )}
        </div>

        {/* Connection form */}
        <AnimatePresence>
          {showSynoForm && !synoConnected && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="mb-4 overflow-hidden"
            >
              <div className="p-4 rounded-xl border border-white/10 bg-white/3 space-y-3">
                <p
                  className="text-[11px]"
                  style={{ color: "var(--cv-subtext)" }}
                >
                  Enter your Synology QuickConnect ID (e.g.{" "}
                  <span className="font-mono text-blue-400">mynas</span>) or the
                  local IP address of your NAS.
                </p>
                <div className="grid grid-cols-2 gap-3">
                  <div className="col-span-2">
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      QuickConnect ID or IP Address
                    </label>
                    <input
                      type="text"
                      value={synoForm.quickconnect_id}
                      onChange={(e) =>
                        setSynoForm((p) => ({
                          ...p,
                          quickconnect_id: e.target.value,
                        }))
                      }
                      placeholder="mynas  or  192.168.1.100"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Username
                    </label>
                    <input
                      type="text"
                      value={synoForm.username}
                      onChange={(e) =>
                        setSynoForm((p) => ({ ...p, username: e.target.value }))
                      }
                      placeholder="admin"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Password
                    </label>
                    <input
                      type="password"
                      value={synoForm.password}
                      onChange={(e) =>
                        setSynoForm((p) => ({ ...p, password: e.target.value }))
                      }
                      placeholder="••••••••"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Port (optional)
                    </label>
                    <input
                      type="number"
                      value={synoForm.port}
                      onChange={(e) =>
                        setSynoForm((p) => ({ ...p, port: e.target.value }))
                      }
                      placeholder="5001 (HTTPS) / 5000 (HTTP)"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div className="flex items-center gap-2 mt-4">
                    <input
                      type="checkbox"
                      id="syno-https"
                      checked={synoForm.use_https}
                      onChange={(e) =>
                        setSynoForm((p) => ({
                          ...p,
                          use_https: e.target.checked,
                        }))
                      }
                      className="w-3.5 h-3.5 rounded"
                    />
                    <label
                      htmlFor="syno-https"
                      className="text-[11px] flex items-center gap-1"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      <Shield size={11} className="text-green-400" /> Use HTTPS
                      (recommended)
                    </label>
                  </div>
                </div>
                {synoError && (
                  <div className="flex items-center gap-2 p-2 rounded-lg bg-red-500/10 text-red-400 text-[11px]">
                    <AlertTriangle size={12} /> {synoError}
                  </div>
                )}
                <div className="flex items-center gap-2">
                  <button
                    onClick={connectSynology}
                    disabled={synoConnecting}
                    className="cv-btn text-xs flex items-center gap-1 disabled:opacity-50"
                  >
                    {synoConnecting ? (
                      <RefreshCw size={12} className="animate-spin" />
                    ) : (
                      <Link2 size={12} />
                    )}
                    {synoConnecting ? "Connecting..." : "Connect to Synology"}
                  </button>
                  <button
                    onClick={() => {
                      setShowSynoForm(false);
                      setSynoError(null);
                    }}
                    className="cv-btn text-xs bg-white/5"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Connected state */}
        {synoConnected && synoInfo && (
          <div className="space-y-3">
            {/* Device info */}
            <div className="flex items-center gap-3 p-3 rounded-xl border border-green-500/20 bg-green-500/5">
              <div className="w-10 h-10 rounded-xl bg-blue-500/10 flex items-center justify-center">
                <Server size={20} className="text-blue-400" />
              </div>
              <div className="flex-1">
                <div
                  className="text-sm font-semibold"
                  style={{ color: "var(--cv-text)" }}
                >
                  {synoInfo.device_name}
                </div>
                <div
                  className="text-[10px]"
                  style={{ color: "var(--cv-subtext)" }}
                >
                  {synoInfo.device_model}
                  {synoInfo.firmware ? ` · ${synoInfo.firmware}` : ""} ·{" "}
                  {synoInfo.host_resolved}
                </div>
              </div>
              <CheckCircle2 size={16} className="text-green-400" />
            </div>

            {/* Shared libraries */}
            <div>
              <button
                onClick={() => setSynoLibrariesExpanded(!synoLibrariesExpanded)}
                className="flex items-center gap-2 w-full text-left mb-2"
              >
                <Database size={13} style={{ color: "var(--cv-accent)" }} />
                <span
                  className="text-xs font-semibold"
                  style={{ color: "var(--cv-text)" }}
                >
                  Shared Folders ({synoInfo.libraries.length})
                </span>
                {synoLibrariesExpanded ? (
                  <ChevronUp size={13} className="ml-auto opacity-50" />
                ) : (
                  <ChevronDown size={13} className="ml-auto opacity-50" />
                )}
              </button>
              <AnimatePresence>
                {synoLibrariesExpanded && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    className="overflow-hidden"
                  >
                    <div className="space-y-1.5">
                      {synoInfo.libraries.map((lib) => (
                        <div
                          key={lib.id}
                          className="flex items-center gap-3 p-2.5 rounded-lg border border-white/5 bg-white/3 hover:bg-white/5 transition-all"
                        >
                          <FolderOpen
                            size={14}
                            style={{ color: "var(--cv-accent)" }}
                          />
                          <div className="flex-1 min-w-0">
                            <div
                              className="text-xs font-medium truncate"
                              style={{ color: "var(--cv-text)" }}
                            >
                              {lib.name}
                            </div>
                            <div
                              className="text-[10px] truncate"
                              style={{ color: "var(--cv-subtext)" }}
                            >
                              {lib.path} · {lib.media_type}{" "}
                              {lib.size_bytes > 0
                                ? `· ${formatBytes(lib.size_bytes)}`
                                : ""}
                            </div>
                          </div>
                          <button
                            onClick={() => addSynoLibrary(lib)}
                            className="cv-btn text-[10px] py-1 px-2 flex items-center gap-1 whitespace-nowrap"
                          >
                            <Plus size={10} /> Add to Library
                          </button>
                        </div>
                      ))}
                      {synoInfo.libraries.length === 0 && (
                        <p
                          className="text-xs text-center py-3"
                          style={{ color: "var(--cv-subtext)" }}
                        >
                          No shared folders found.
                        </p>
                      )}
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>
        )}

        {!synoConnected && !showSynoForm && (
          <div className="text-center py-6">
            <Server size={32} className="mx-auto mb-2 opacity-20" />
            <p className="text-xs" style={{ color: "var(--cv-subtext)" }}>
              Connect your Synology NAS via QuickConnect ID or local IP to
              browse and add shared folders as CinaVault libraries.
            </p>
          </div>
        )}
      </div>

      {/* ══════════════════════════════════════════════════════
           WD My Cloud
      ══════════════════════════════════════════════════════ */}
      <div className="cv-card p-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <HardDrive size={18} style={{ color: "#f59e0b" }} />
            <h3
              className="text-base font-bold"
              style={{ color: "var(--cv-text)" }}
            >
              WD My Cloud
            </h3>
            <span
              className="text-[10px] px-2 py-0.5 rounded-full"
              style={{
                background: wdConnected
                  ? "rgba(34,197,94,0.15)"
                  : "rgba(156,163,175,0.15)",
                color: wdConnected ? "#22c55e" : "#9ca3af",
              }}
            >
              {wdConnected ? "Connected" : "Disconnected"}
            </span>
            <span className="text-[10px] px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-400">
              WD My Cloud Home
            </span>
          </div>
          {!wdConnected ? (
            <button
              onClick={() => setShowWdForm(!showWdForm)}
              className="cv-btn text-xs flex items-center gap-1"
            >
              <Link2 size={12} /> {showWdForm ? "Cancel" : "Connect"}
            </button>
          ) : (
            <button
              onClick={disconnectWdMyCloud}
              className="cv-btn text-xs flex items-center gap-1 bg-red-500/10 hover:bg-red-500/20 text-red-400"
            >
              <LogOut size={12} /> Disconnect
            </button>
          )}
        </div>

        {/* Connection form */}
        <AnimatePresence>
          {showWdForm && !wdConnected && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="mb-4 overflow-hidden"
            >
              <div className="p-4 rounded-xl border border-white/10 bg-white/3 space-y-3">
                <p
                  className="text-[11px]"
                  style={{ color: "var(--cv-subtext)" }}
                >
                  Enter the local IP address or hostname of your WD My Cloud
                  device (e.g.{" "}
                  <span className="font-mono text-amber-400">192.168.1.50</span>{" "}
                  or <span className="font-mono text-amber-400">wdmycloud</span>
                  ).
                </p>
                <div className="grid grid-cols-2 gap-3">
                  <div className="col-span-2">
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Host / IP Address
                    </label>
                    <input
                      type="text"
                      value={wdForm.host}
                      onChange={(e) =>
                        setWdForm((p) => ({ ...p, host: e.target.value }))
                      }
                      placeholder="192.168.1.50  or  wdmycloud"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Username
                    </label>
                    <input
                      type="text"
                      value={wdForm.username}
                      onChange={(e) =>
                        setWdForm((p) => ({ ...p, username: e.target.value }))
                      }
                      placeholder="admin"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Password
                    </label>
                    <input
                      type="password"
                      value={wdForm.password}
                      onChange={(e) =>
                        setWdForm((p) => ({ ...p, password: e.target.value }))
                      }
                      placeholder="••••••••"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Port (optional)
                    </label>
                    <input
                      type="number"
                      value={wdForm.port}
                      onChange={(e) =>
                        setWdForm((p) => ({ ...p, port: e.target.value }))
                      }
                      placeholder="80 (HTTP) / 443 (HTTPS)"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div className="flex items-center gap-2 mt-4">
                    <input
                      type="checkbox"
                      id="wd-https"
                      checked={wdForm.use_https}
                      onChange={(e) =>
                        setWdForm((p) => ({
                          ...p,
                          use_https: e.target.checked,
                        }))
                      }
                      className="w-3.5 h-3.5 rounded"
                    />
                    <label
                      htmlFor="wd-https"
                      className="text-[11px] flex items-center gap-1"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      <Shield size={11} className="text-green-400" /> Use HTTPS
                    </label>
                  </div>
                </div>
                {wdError && (
                  <div className="flex items-center gap-2 p-2 rounded-lg bg-red-500/10 text-red-400 text-[11px]">
                    <AlertTriangle size={12} /> {wdError}
                  </div>
                )}
                <div className="flex items-center gap-2">
                  <button
                    onClick={connectWdMyCloud}
                    disabled={wdConnecting}
                    className="cv-btn text-xs flex items-center gap-1 disabled:opacity-50"
                  >
                    {wdConnecting ? (
                      <RefreshCw size={12} className="animate-spin" />
                    ) : (
                      <Link2 size={12} />
                    )}
                    {wdConnecting ? "Connecting..." : "Connect to WD My Cloud"}
                  </button>
                  <button
                    onClick={() => {
                      setShowWdForm(false);
                      setWdError(null);
                    }}
                    className="cv-btn text-xs bg-white/5"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Connected state */}
        {wdConnected && wdInfo && (
          <div className="space-y-3">
            <div className="flex items-center gap-3 p-3 rounded-xl border border-green-500/20 bg-green-500/5">
              <div className="w-10 h-10 rounded-xl bg-amber-500/10 flex items-center justify-center">
                <HardDrive size={20} className="text-amber-400" />
              </div>
              <div className="flex-1">
                <div
                  className="text-sm font-semibold"
                  style={{ color: "var(--cv-text)" }}
                >
                  {wdInfo.device_name}
                </div>
                <div
                  className="text-[10px]"
                  style={{ color: "var(--cv-subtext)" }}
                >
                  {wdInfo.device_model}
                  {wdInfo.firmware ? ` · ${wdInfo.firmware}` : ""} ·{" "}
                  {wdInfo.host_resolved}
                </div>
              </div>
              <CheckCircle2 size={16} className="text-green-400" />
            </div>

            <div>
              <button
                onClick={() => setWdLibrariesExpanded(!wdLibrariesExpanded)}
                className="flex items-center gap-2 w-full text-left mb-2"
              >
                <Database size={13} style={{ color: "var(--cv-accent)" }} />
                <span
                  className="text-xs font-semibold"
                  style={{ color: "var(--cv-text)" }}
                >
                  Shares ({wdInfo.libraries.length})
                </span>
                {wdLibrariesExpanded ? (
                  <ChevronUp size={13} className="ml-auto opacity-50" />
                ) : (
                  <ChevronDown size={13} className="ml-auto opacity-50" />
                )}
              </button>
              <AnimatePresence>
                {wdLibrariesExpanded && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    className="overflow-hidden"
                  >
                    <div className="space-y-1.5">
                      {wdInfo.libraries.map((lib) => (
                        <div
                          key={lib.id}
                          className="flex items-center gap-3 p-2.5 rounded-lg border border-white/5 bg-white/3 hover:bg-white/5 transition-all"
                        >
                          <FolderOpen
                            size={14}
                            style={{ color: "var(--cv-accent)" }}
                          />
                          <div className="flex-1 min-w-0">
                            <div
                              className="text-xs font-medium truncate"
                              style={{ color: "var(--cv-text)" }}
                            >
                              {lib.name}
                            </div>
                            <div
                              className="text-[10px] truncate"
                              style={{ color: "var(--cv-subtext)" }}
                            >
                              {lib.path} · {lib.media_type}{" "}
                              {lib.size_bytes > 0
                                ? `· ${formatBytes(lib.size_bytes)}`
                                : ""}
                            </div>
                          </div>
                          <button
                            onClick={() => addWdLibrary(lib)}
                            className="cv-btn text-[10px] py-1 px-2 flex items-center gap-1 whitespace-nowrap"
                          >
                            <Plus size={10} /> Add to Library
                          </button>
                        </div>
                      ))}
                      {wdInfo.libraries.length === 0 && (
                        <p
                          className="text-xs text-center py-3"
                          style={{ color: "var(--cv-subtext)" }}
                        >
                          No shares found.
                        </p>
                      )}
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>
        )}

        {!wdConnected && !showWdForm && (
          <div className="text-center py-6">
            <HardDrive size={32} className="mx-auto mb-2 opacity-20" />
            <p className="text-xs" style={{ color: "var(--cv-subtext)" }}>
              Connect your WD My Cloud Home device using its local IP address or
              hostname to browse and add shares as CinaVault libraries.
            </p>
          </div>
        )}
      </div>

      {/* ── Generic NAS / Network Shares ── */}
      <div className="cv-card p-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Wifi size={18} style={{ color: "var(--cv-accent)" }} />
            <h3
              className="text-base font-bold"
              style={{ color: "var(--cv-text)" }}
            >
              Other NAS & Network Shares
            </h3>
          </div>
          <button
            onClick={() => setShowAddNas(!showAddNas)}
            className="cv-btn text-xs flex items-center gap-1"
          >
            <Plus size={12} /> Add Share
          </button>
        </div>

        <AnimatePresence>
          {showAddNas && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="mb-4 overflow-hidden"
            >
              <div className="p-4 rounded-xl border border-white/10 bg-white/3 space-y-3">
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Name
                    </label>
                    <input
                      type="text"
                      value={nasForm.name}
                      onChange={(e) =>
                        setNasForm((p) => ({ ...p, name: e.target.value }))
                      }
                      placeholder="My NAS"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Protocol
                    </label>
                    <select
                      value={nasForm.protocol}
                      onChange={(e) =>
                        setNasForm((p) => ({ ...p, protocol: e.target.value }))
                      }
                      className="cv-input text-xs w-full"
                    >
                      <option value="smb">SMB/CIFS</option>
                      <option value="nfs">NFS</option>
                      <option value="ftp">FTP</option>
                      <option value="sftp">SFTP</option>
                      <option value="webdav">WebDAV</option>
                    </select>
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Host / IP
                    </label>
                    <input
                      type="text"
                      value={nasForm.host}
                      onChange={(e) =>
                        setNasForm((p) => ({ ...p, host: e.target.value }))
                      }
                      placeholder="192.168.1.100"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Port
                    </label>
                    <input
                      type="number"
                      value={nasForm.port}
                      onChange={(e) =>
                        setNasForm((p) => ({
                          ...p,
                          port: parseInt(e.target.value) || 445,
                        }))
                      }
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Share Path
                    </label>
                    <input
                      type="text"
                      value={nasForm.path}
                      onChange={(e) =>
                        setNasForm((p) => ({ ...p, path: e.target.value }))
                      }
                      placeholder="/media/movies"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                  <div>
                    <label
                      className="text-[10px] font-medium mb-1 block"
                      style={{ color: "var(--cv-subtext)" }}
                    >
                      Username
                    </label>
                    <input
                      type="text"
                      value={nasForm.username}
                      onChange={(e) =>
                        setNasForm((p) => ({ ...p, username: e.target.value }))
                      }
                      placeholder="admin"
                      className="cv-input text-xs w-full"
                    />
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={addNas}
                    className="cv-btn text-xs flex items-center gap-1"
                  >
                    <Plus size={12} /> Add
                  </button>
                  <button
                    onClick={() => setShowAddNas(false)}
                    className="cv-btn text-xs bg-white/5"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {nasProfiles.length > 0 ? (
          <div className="space-y-2">
            {nasProfiles.map((nas) => (
              <div
                key={nas.id}
                className="flex items-center gap-3 p-3 rounded-xl border border-white/5 bg-white/3"
              >
                <HardDrive size={18} style={{ color: "var(--cv-accent)" }} />
                <div className="flex-1">
                  <div
                    className="text-sm font-medium"
                    style={{ color: "var(--cv-text)" }}
                  >
                    {nas.name}
                  </div>
                  <div
                    className="text-[10px]"
                    style={{ color: "var(--cv-subtext)" }}
                  >
                    {nas.protocol.toUpperCase()}://{nas.host}:{nas.port}
                    {nas.path}
                  </div>
                </div>
                <span
                  className="w-2 h-2 rounded-full"
                  style={{
                    background:
                      nas.status === "connected" ? "#22c55e" : "#9ca3af",
                  }}
                />
                <button
                  onClick={() =>
                    setNasProfiles((prev) =>
                      prev.filter((n) => n.id !== nas.id),
                    )
                  }
                  className="w-7 h-7 rounded-lg flex items-center justify-center bg-red-500/10 hover:bg-red-500/20"
                >
                  <Trash2 size={12} className="text-red-400" />
                </button>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-6">
            <WifiOff size={32} className="mx-auto mb-2 opacity-20" />
            <p className="text-xs" style={{ color: "var(--cv-subtext)" }}>
              No additional network shares configured. Use "Add Share" for SMB,
              NFS, FTP, SFTP, or WebDAV.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
