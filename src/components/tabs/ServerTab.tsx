// CinaVault Premium — Server Tab (MS-C/MS-B Management + Public Access Settings)
import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { useAppStore } from "../../store/appStore";
import {
  CheckCircle,
  Clipboard,
  ExternalLink,
  FileText,
  Globe,
  Import,
  LayoutDashboard,
  Library,
  Link2,
  ListTodo,
  Monitor,
  Play,
  Puzzle,
  RefreshCw,
  Server,
  Settings,
  Shield,
  Smartphone,
  Square,
  Users,
  Wifi,
  XCircle,
} from "lucide-react";

const ADMIN_PAGES = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "libraries", label: "Libraries", icon: Library },
  { id: "users", label: "Users", icon: Users },
  { id: "plugins", label: "Plugins", icon: Puzzle },
  { id: "tasks", label: "Tasks", icon: ListTodo },
  { id: "logs", label: "Logs", icon: FileText },
  { id: "sessions", label: "Sessions API", icon: Monitor },
  { id: "devices", label: "Devices API", icon: Smartphone },
];

type PublicIpResponse = {
  ip?: string;
};

function normalizeBaseUrl(url: string): string {
  const trimmed = url.trim();
  if (!trimmed) return "http://localhost:8096";
  if (/^https?:\/\//i.test(trimmed)) return trimmed.replace(/\/$/, "");
  return `http://${trimmed}`.replace(/\/$/, "");
}

function buildPublicUrl(
  publicIp: string,
  port: string,
  secure: boolean,
): string {
  const cleanIp = publicIp.trim();
  const cleanPort = port.trim() || "8096";
  if (!cleanIp) return "";
  const scheme = secure ? "https" : "http";
  return `${scheme}://${cleanIp}:${cleanPort}`;
}

export default function ServerTab() {
  const {
    serverRunning,
    serverType,
    serverUrl,
    setServerStatus,
    addStatusMessage,
  } = useAppStore();
  const [serverInfo, setServerInfo] = useState<any>(null);
  const [embyCompat, setEmbyCompat] = useState<any>(null);
  const [customUrl, setCustomUrl] = useState(
    serverUrl || "http://localhost:8096",
  );
  const [apiKey, setApiKey] = useState("");
  const [checking, setChecking] = useState(false);
  const [publicIp, setPublicIp] = useState("");
  const [publicPort, setPublicPort] = useState("8096");
  const [useHttpsPublic, setUseHttpsPublic] = useState(false);
  const [publicLookupBusy, setPublicLookupBusy] = useState(false);

  const normalizedCustomUrl = useMemo(
    () => normalizeBaseUrl(customUrl),
    [customUrl],
  );
  const publicAccessUrl = useMemo(
    () => buildPublicUrl(publicIp, publicPort, useHttpsPublic),
    [publicIp, publicPort, useHttpsPublic],
  );

  useEffect(() => {
    void checkServer();
  }, []);

  const openLink = (url: string) => {
    const target = normalizeBaseUrl(url);
    window.open(target, "_blank", "noopener,noreferrer");
  };

  const copyText = async (text: string, label: string) => {
    if (!text.trim()) {
      addStatusMessage(`${label} is empty`);
      return;
    }

    try {
      await navigator.clipboard.writeText(text);
      addStatusMessage(`${label} copied`);
    } catch {
      addStatusMessage(`Could not copy ${label}`);
    }
  };

  const checkServer = async () => {
    setChecking(true);
    try {
      const status = await invoke<any>("get_server_status", {
        serverType,
        baseUrl: normalizedCustomUrl,
      });
      setServerStatus(Boolean(status.running), serverType, normalizedCustomUrl);
      setServerInfo(status.running ? status : null);
      if (status.running) {
        addStatusMessage(
          `Server detected: ${status.server_name || serverType} v${status.version || "unknown"}`,
        );
      }
    } catch (error) {
      setServerStatus(false, serverType, normalizedCustomUrl);
      setServerInfo(null);
      addStatusMessage(`Server check failed: ${error}`);
    } finally {
      setChecking(false);
    }
  };

  const startServer = async () => {
    addStatusMessage(`Starting ${serverType} server...`);
    try {
      const result = await invoke<any>("start_server", { serverType });
      addStatusMessage(
        `Server ${result.status || "started"}${result.path ? `: ${result.path}` : ""}`,
      );
      window.setTimeout(() => void checkServer(), 3000);
    } catch (error) {
      addStatusMessage(`Start failed: ${error}`);
    }
  };

  const stopServer = async () => {
    try {
      await invoke("stop_server", { serverType });
      addStatusMessage(`${serverType} server stopped`);
      setServerStatus(false, serverType, normalizedCustomUrl);
      setServerInfo(null);
    } catch (error) {
      addStatusMessage(`Stop failed: ${error}`);
    }
  };

  const openAdmin = async (page: string) => {
    try {
      await invoke("open_admin_page", { baseUrl: normalizedCustomUrl, page });
    } catch (error) {
      addStatusMessage(`Failed to open admin page: ${error}`);
    }
  };

  const importLibraries = async () => {
    if (!apiKey.trim()) {
      addStatusMessage("API key required for import");
      return;
    }

    try {
      const result = await invoke<any>("import_libraries", {
        baseUrl: normalizedCustomUrl,
        apiKey: apiKey.trim(),
      });
      addStatusMessage(
        `Imported ${result.sources_imported || 0} library sources from ${result.libraries_found || 0} libraries`,
      );
    } catch (error) {
      addStatusMessage(`Import failed: ${error}`);
    }
  };

  const checkEmbyCompat = async () => {
    try {
      const result = await invoke<any>("check_emby_compat", {
        baseUrl: normalizedCustomUrl,
      });
      setEmbyCompat(result);
      addStatusMessage(
        result.compatible
          ? `Compatible: ${result.product} v${result.version}`
          : "Compatibility check failed",
      );
    } catch (error) {
      addStatusMessage(`Compatibility check failed: ${error}`);
    }
  };

  const lookupPublicIp = async () => {
    setPublicLookupBusy(true);
    try {
      const response = await fetch("https://api.ipify.org?format=json", {
        cache: "no-store",
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = (await response.json()) as PublicIpResponse;
      if (!data.ip) throw new Error("No public IP returned");
      setPublicIp(data.ip);
      addStatusMessage(`Public IP detected: ${data.ip}`);
    } catch (error) {
      addStatusMessage(`Public IP lookup failed: ${error}`);
    } finally {
      setPublicLookupBusy(false);
    }
  };

  const applyPublicUrlAsServer = () => {
    if (!publicAccessUrl) {
      addStatusMessage("Public URL is not ready yet");
      return;
    }
    setCustomUrl(publicAccessUrl);
    setServerStatus(serverRunning, serverType, publicAccessUrl);
    addStatusMessage("Public URL applied as the active server URL");
  };

  return (
    <div className="space-y-5">
      <div className="glass-panel p-5">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-bold flex items-center gap-2">
            <Server size={16} className="text-cv-accent" /> Server Management
          </h3>
          <div className="flex items-center gap-3">
            <select
              value={serverType}
              onChange={(event) =>
                setServerStatus(
                  serverRunning,
                  event.target.value,
                  normalizedCustomUrl,
                )
              }
              className="cv-select text-xs py-1.5"
            >
              <option value="jellyfin">MS-C</option>
              <option value="emby">MS-B</option>
            </select>
            <button
              onClick={checkServer}
              className="cv-btn cv-btn-secondary text-xs py-1.5"
            >
              <RefreshCw size={12} className={checking ? "animate-spin" : ""} />{" "}
              Check
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 xl:grid-cols-3 gap-4">
          <div className="glass-panel-2 p-4 rounded-lg">
            <div className="flex items-center gap-3 mb-3">
              <div
                className={`w-12 h-12 rounded-xl flex items-center justify-center ${serverRunning ? "bg-green-500/20" : "bg-cv-danger/20"}`}
              >
                <Server
                  size={24}
                  className={
                    serverRunning ? "text-green-500" : "text-cv-danger"
                  }
                />
              </div>
              <div>
                <div className="text-sm font-bold">
                  {serverRunning ? "Running" : "Stopped"}
                </div>
                <div className="text-[10px] text-cv-subtext capitalize">
                  {serverType} Server
                </div>
              </div>
            </div>
            {serverInfo && (
              <div className="space-y-1 text-xs text-cv-subtext">
                <div>
                  Name:{" "}
                  <span className="text-cv-text">
                    {serverInfo.server_name || "Unknown"}
                  </span>
                </div>
                <div>
                  Version:{" "}
                  <span className="text-cv-text">
                    {serverInfo.version || "Unknown"}
                  </span>
                </div>
              </div>
            )}
            <div className="flex gap-2 mt-4">
              {!serverRunning ? (
                <button
                  onClick={startServer}
                  className="cv-btn cv-btn-primary text-xs flex-1"
                >
                  <Play size={12} /> Start Server
                </button>
              ) : (
                <button
                  onClick={stopServer}
                  className="cv-btn cv-btn-danger text-xs flex-1"
                >
                  <Square size={12} /> Stop Server
                </button>
              )}
            </div>
          </div>

          <div className="glass-panel-2 p-4 rounded-lg">
            <label className="section-label">Server URL</label>
            <input
              value={customUrl}
              onChange={(event) => setCustomUrl(event.target.value)}
              className="cv-input mb-2"
            />
            <label className="section-label">API Key for Library Imports</label>
            <div className="flex gap-2 mb-2">
              <input
                type="password"
                value={apiKey}
                onChange={(event) => setApiKey(event.target.value)}
                className="cv-input flex-1"
                placeholder="Enter API key"
              />
              <button
                onClick={() => openLink(normalizedCustomUrl)}
                className="cv-btn cv-btn-secondary text-xs shrink-0"
              >
                <ExternalLink size={12} /> Key
              </button>
            </div>
            <div className="text-[10px] text-cv-subtext mb-3">
              {serverType === "emby"
                ? "MS-B: Dashboard > Advanced > Access Keys."
                : "MS-C: Dashboard > Advanced > Access Keys."}
            </div>
            <div className="grid grid-cols-2 gap-2">
              <button
                onClick={importLibraries}
                className="cv-btn cv-btn-secondary text-xs"
              >
                <Import size={12} /> Import
              </button>
              <button
                onClick={() => copyText(normalizedCustomUrl, "Server URL")}
                className="cv-btn cv-btn-secondary text-xs"
              >
                <Clipboard size={12} /> Copy
              </button>
            </div>
          </div>

          <div className="glass-panel-2 p-4 rounded-lg">
            <label className="section-label">MS-B SDK Compatibility</label>
            <button
              onClick={checkEmbyCompat}
              className="cv-btn cv-btn-secondary text-xs mb-3 w-full"
            >
              <CheckCircle size={12} /> Check Compatibility
            </button>
            {embyCompat && (
              <div className="space-y-1 text-xs">
                <div className="flex items-center gap-2">
                  {embyCompat.compatible ? (
                    <CheckCircle size={12} className="text-green-500" />
                  ) : (
                    <XCircle size={12} className="text-cv-danger" />
                  )}
                  <span>
                    {embyCompat.compatible ? "Compatible" : "Not Compatible"}
                  </span>
                </div>
                {embyCompat.product && (
                  <div className="text-cv-subtext">
                    Product: {embyCompat.product}
                  </div>
                )}
                {embyCompat.version && (
                  <div className="text-cv-subtext">
                    Version: {embyCompat.version}
                  </div>
                )}
                <div className="text-cv-subtext">
                  MS-B API: {embyCompat.emby_api ? "Yes" : "No"}
                </div>
                <div className="text-cv-subtext">
                  MS-C API: {embyCompat.jellyfin_api ? "Yes" : "No"}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="glass-panel p-5">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-bold flex items-center gap-2">
            <Globe size={16} className="text-cv-accent" /> Public Access
            Settings
          </h3>
          <button
            onClick={lookupPublicIp}
            className="cv-btn cv-btn-secondary text-xs py-1.5"
          >
            <Wifi
              size={12}
              className={publicLookupBusy ? "animate-pulse" : ""}
            />{" "}
            Lookup Public IP
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
          <div>
            <label className="section-label">Public IP / Hostname</label>
            <input
              value={publicIp}
              onChange={(event) => setPublicIp(event.target.value)}
              className="cv-input"
              placeholder="Auto lookup or enter domain"
            />
          </div>
          <div>
            <label className="section-label">Public Port</label>
            <input
              value={publicPort}
              onChange={(event) => setPublicPort(event.target.value)}
              className="cv-input"
              placeholder="8096"
            />
          </div>
          <div>
            <label className="section-label">Connection</label>
            <button
              onClick={() => setUseHttpsPublic((value) => !value)}
              className={`cv-btn ${useHttpsPublic ? "cv-btn-primary" : "cv-btn-secondary"} text-xs w-full`}
            >
              <Shield size={12} /> {useHttpsPublic ? "HTTPS" : "HTTP"}
            </button>
          </div>
          <div>
            <label className="section-label">Generated URL</label>
            <input
              value={publicAccessUrl || "Not configured"}
              readOnly
              className="cv-input"
            />
          </div>
        </div>

        <div className="flex flex-wrap gap-2 mt-4">
          <button
            onClick={applyPublicUrlAsServer}
            className="cv-btn cv-btn-primary text-xs"
          >
            <Settings size={12} /> Use as Server URL
          </button>
          <button
            onClick={() => copyText(publicAccessUrl, "Public URL")}
            className="cv-btn cv-btn-secondary text-xs"
          >
            <Clipboard size={12} /> Copy Public URL
          </button>
          <button
            onClick={() => publicAccessUrl && openLink(publicAccessUrl)}
            className="cv-btn cv-btn-secondary text-xs"
          >
            <ExternalLink size={12} /> Open Public URL
          </button>
          <button
            onClick={() => copyText(`${publicPort}/tcp`, "Port rule")}
            className="cv-btn cv-btn-secondary text-xs"
          >
            <Link2 size={12} /> Copy Port Rule
          </button>
        </div>

        <div className="mt-3 text-[11px] text-cv-subtext leading-relaxed">
          Public access usually requires router port forwarding, firewall
          allowance, and a stable IP or DNS hostname. Keep HTTPS enabled when
          your reverse proxy or certificate setup supports it.
        </div>
      </div>

      <div className="glass-panel p-5">
        <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
          <LayoutDashboard size={16} className="text-cv-accent" /> Admin Console
        </h3>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
          {ADMIN_PAGES.map((page) => (
            <motion.button
              key={page.id}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              onClick={() => openAdmin(page.id)}
              className="glass-panel-2 p-4 rounded-lg flex flex-col items-center gap-2 hover:bg-white/5 transition-colors"
            >
              <page.icon size={24} className="text-cv-accent" />
              <span className="text-xs font-semibold">{page.label}</span>
              <ExternalLink size={10} className="text-cv-subtext" />
            </motion.button>
          ))}
        </div>
      </div>
    </div>
  );
}
