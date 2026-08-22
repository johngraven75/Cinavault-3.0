// CinaVault Premium — Build 170 Remote Access Management
import React, { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { useAppStore } from "../../store/appStore";
import {
  AlertTriangle,
  CheckCircle,
  Cloud,
  Copy,
  Globe,
  KeyRound,
  Link2,
  Lock,
  LogIn,
  PlugZap,
  Power,
  RefreshCw,
  RotateCw,
  Router,
  ShieldCheck,
  SlidersHorizontal,
  UserPlus,
  Wifi,
} from "lucide-react";

type SecureMode = "required" | "preferred" | "disabled";

type RemoteAccessUser = {
  id: number;
  email: string;
  display_name?: string | null;
  access_key_preview: string;
  enabled: boolean;
  permissions: string[];
  created_at: string;
  updated_at: string;
  last_login?: string | null;
};

type RemoteProvision = RemoteAccessUser & {
  access_key: string;
};

type RemotePrincipal = {
  id: number;
  email: string;
  display_name?: string | null;
  auth_method: "password" | "access_key";
  session_token: string;
  expires_at: string;
  permissions: string[];
};

type RemoteKeyRotation = {
  email: string;
  access_key: string;
  access_key_preview: string;
};

type RemoteConnectivityStatus = {
  running: boolean;
  automatic: boolean;
  port: number;
  directAvailable: boolean;
  directMethod?: string | null;
  directUrl?: string | null;
  publicIp?: string | null;
  relayActive: boolean;
  relayMode?: "named" | "quick" | string | null;
  relayUrl?: string | null;
  preferredUrl?: string | null;
  lastError?: string | null;
};

const emptyConnectivity: RemoteConnectivityStatus = {
  running: false,
  automatic: true,
  port: 32400,
  directAvailable: false,
  directMethod: null,
  directUrl: null,
  publicIp: null,
  relayActive: false,
  relayMode: null,
  relayUrl: null,
  preferredUrl: null,
  lastError: null,
};

const secureOptions: { value: SecureMode; label: string; desc: string }[] = [
  {
    value: "required",
    label: "Required",
    desc: "Require the encrypted HTTPS cloud relay for remote clients.",
  },
  {
    value: "preferred",
    label: "Preferred",
    desc: "Use automatic direct access first and encrypted relay fallback.",
  },
  {
    value: "disabled",
    label: "Disabled",
    desc: "Permit direct HTTP access when router mapping succeeds.",
  },
];

export function isRemoteAccessConfigurationValid(
  remoteEnabled: boolean,
  publicPort: string,
  manualPort: boolean,
) {
  if (!remoteEnabled) return false;
  if (!publicPort || Number.isNaN(Number(publicPort))) return false;
  if (manualPort && (Number(publicPort) < 1 || Number(publicPort) > 65535)) {
    return false;
  }
  return true;
}

export default function RemoteAccessTab() {
  const { settings, setSetting, serverUrl, addStatusMessage } = useAppStore();
  const [testing, setTesting] = useState(false);
  const [lastTestAt, setLastTestAt] = useState("");
  const [connectivity, setConnectivity] =
    useState<RemoteConnectivityStatus>(emptyConnectivity);
  const [accounts, setAccounts] = useState<RemoteAccessUser[]>([]);
  const [accountForm, setAccountForm] = useState({
    displayName: "",
    email: "",
    password: "",
  });
  const [passwordLogin, setPasswordLogin] = useState({
    email: "",
    password: "",
  });
  const [keyLogin, setKeyLogin] = useState("");
  const [latestKey, setLatestKey] = useState<
    RemoteKeyRotation | RemoteProvision | null
  >(null);
  const [lastPrincipal, setLastPrincipal] = useState<RemotePrincipal | null>(
    null,
  );
  const [busy, setBusy] = useState<string | null>(null);

  const remoteEnabled = settings.remote_access_enabled !== "false";
  const manualPort = settings.remote_manually_specify_port === "true";
  const secureMode = (settings.remote_secure_connections ||
    "preferred") as SecureMode;
  const preferredRelay = settings.remote_preferred_relay === "true";
  const fallback = settings.remote_allow_fallback !== "false";
  const upnp = settings.remote_enable_upnp !== "false";
  const natPmp = settings.remote_enable_natpmp !== "false";
  const publicPort = settings.remote_public_port || "32400";
  const uploadLimit = settings.remote_upload_limit_mbps || "20";
  const allowedNetworks = settings.remote_allowed_networks || "";

  const configured = useMemo(
    () =>
      isRemoteAccessConfigurationValid(remoteEnabled, publicPort, manualPort),
    [remoteEnabled, publicPort, manualPort],
  );
  const remotelyReachable =
    remoteEnabled && (connectivity.directAvailable || connectivity.relayActive);
  const effectivePreferRelay = preferredRelay || secureMode === "required";

  const loadAccounts = async () => {
    try {
      const rows = await invoke<RemoteAccessUser[]>("list_remote_access_users");
      setAccounts(rows);
    } catch (error) {
      addStatusMessage(`Remote accounts unavailable: ${error}`);
    }
  };

  const loadConnectivity = async () => {
    try {
      const status = await invoke<RemoteConnectivityStatus>(
        "get_remote_connectivity_status",
      );
      setConnectivity(status);
    } catch (error) {
      addStatusMessage(`Remote connectivity status unavailable: ${error}`);
    }
  };

  useEffect(() => {
    void loadAccounts();
    void loadConnectivity();
    const timer = window.setInterval(() => void loadConnectivity(), 5000);
    return () => window.clearInterval(timer);
  }, []);

  const runConnectionTest = async () => {
    setTesting(true);
    try {
      if (!remoteEnabled) {
        const stopped = await invoke<RemoteConnectivityStatus>(
          "stop_remote_connectivity",
        );
        setConnectivity(stopped);
        addStatusMessage("Automatic remote access stopped");
        return;
      }

      const port = Number(publicPort || 32400);
      const status = await invoke<RemoteConnectivityStatus>(
        "start_remote_connectivity",
        {
          port,
          preferRelay: effectivePreferRelay,
          allowRelay: fallback,
          enableUpnp: upnp,
          enableNatPmp: natPmp,
        },
      );
      setConnectivity(status);
      setLastTestAt(new Date().toLocaleTimeString());
      if (status.preferredUrl) {
        addStatusMessage(`Remote access ready: ${status.preferredUrl}`);
      } else {
        addStatusMessage(
          `Remote access unavailable: ${status.lastError || "No reachable route"}`,
        );
      }
    } catch (error) {
      addStatusMessage(`Remote access test failed: ${error}`);
    } finally {
      setTesting(false);
    }
  };

  const copyRemoteUrl = async () => {
    if (!connectivity.preferredUrl) return;
    await navigator.clipboard?.writeText(connectivity.preferredUrl);
    addStatusMessage("Remote server URL copied");
  };

  const createAccount = async () => {
    setBusy("create");
    try {
      const provision = await invoke<RemoteProvision>(
        "create_remote_access_user",
        {
          email: accountForm.email,
          password: accountForm.password,
          displayName: accountForm.displayName || null,
        },
      );
      setLatestKey(provision);
      setAccountForm({ displayName: "", email: "", password: "" });
      addStatusMessage(`Remote account ready: ${provision.email}`);
      await loadAccounts();
    } catch (error) {
      addStatusMessage(`Remote account setup failed: ${error}`);
    } finally {
      setBusy(null);
    }
  };

  const loginWithPassword = async () => {
    setBusy("password");
    try {
      const principal = await invoke<RemotePrincipal | null>(
        "authenticate_remote_password",
        passwordLogin,
      );
      setLastPrincipal(principal);
      addStatusMessage(
        principal
          ? `Password access accepted for ${principal.email}`
          : "Password access denied",
      );
      await loadAccounts();
    } catch (error) {
      addStatusMessage(`Password access failed: ${error}`);
    } finally {
      setBusy(null);
    }
  };

  const loginWithKey = async () => {
    setBusy("key");
    try {
      const principal = await invoke<RemotePrincipal | null>(
        "authenticate_remote_access_key",
        { accessKey: keyLogin },
      );
      setLastPrincipal(principal);
      addStatusMessage(
        principal
          ? `Access key accepted for ${principal.email}`
          : "Access key denied",
      );
      await loadAccounts();
    } catch (error) {
      addStatusMessage(`Access key check failed: ${error}`);
    } finally {
      setBusy(null);
    }
  };

  const rotateKey = async (email: string) => {
    setBusy(`rotate:${email}`);
    try {
      const rotated = await invoke<RemoteKeyRotation | null>(
        "rotate_remote_access_key",
        { email },
      );
      if (rotated) {
        setLatestKey(rotated);
        addStatusMessage(`Access key rotated for ${email}`);
      }
      await loadAccounts();
    } catch (error) {
      addStatusMessage(`Key rotation failed: ${error}`);
    } finally {
      setBusy(null);
    }
  };

  const toggleAccount = async (account: RemoteAccessUser) => {
    setBusy(`toggle:${account.email}`);
    try {
      await invoke("set_remote_access_user_enabled", {
        email: account.email,
        enabled: !account.enabled,
      });
      addStatusMessage(
        `${account.email} ${account.enabled ? "disabled" : "enabled"}`,
      );
      await loadAccounts();
    } catch (error) {
      addStatusMessage(`Remote account update failed: ${error}`);
    } finally {
      setBusy(null);
    }
  };

  const copyLatestKey = async () => {
    if (!latestKey?.access_key) return;
    await navigator.clipboard?.writeText(latestKey.access_key);
    addStatusMessage("Remote access key copied");
  };

  return (
    <div className="space-y-5">
      <div className="glass-panel p-5">
        <div className="flex items-center justify-between gap-3 mb-4">
          <div>
            <h3 className="text-sm font-bold flex items-center gap-2">
              <Router size={16} className="text-cv-accent" /> Build 170 Remote
              Connectivity
            </h3>
            <p className="mt-1 text-[11px] text-cv-subtext">
              Automatic UPnP/NAT-PMP direct access with encrypted outbound cloud
              relay fallback.
            </p>
          </div>
          <button
            onClick={runConnectionTest}
            disabled={testing || !configured}
            className="cv-btn cv-btn-secondary text-xs"
          >
            <RefreshCw size={12} className={testing ? "animate-spin" : ""} />
            {testing ? "Negotiating..." : "Refresh Automatic Access"}
          </button>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-4 gap-4">
          <div className="glass-panel-2 p-4 rounded-lg">
            <div className="text-[11px] text-cv-subtext mb-2">Status</div>
            <div className="flex items-center gap-2 mb-2">
              {remotelyReachable ? (
                <CheckCircle size={14} className="text-green-400" />
              ) : (
                <AlertTriangle size={14} className="text-amber-400" />
              )}
              <span className="text-sm font-semibold">
                {remotelyReachable ? "Remote Ready" : "Not Reachable"}
              </span>
            </div>
            <div className="text-xs text-cv-subtext">
              Local server: {serverUrl}
            </div>
            <div className="text-xs text-cv-subtext">
              Last refresh: {lastTestAt || "Startup automation"}
            </div>
            <div className="text-xs text-cv-subtext">
              Remote users: {accounts.length}
            </div>
          </div>

          <div className="glass-panel-2 p-4 rounded-lg">
            <div className="text-[11px] text-cv-subtext mb-2 flex items-center gap-1">
              <Wifi size={11} /> Direct Route
            </div>
            <div className="text-sm font-semibold">
              {connectivity.directAvailable
                ? connectivity.directMethod || "Mapped"
                : "Unavailable"}
            </div>
            <div className="text-xs text-cv-subtext break-all mt-1">
              {connectivity.directUrl || "Router mapping has not succeeded."}
            </div>
            <div className="text-xs text-cv-subtext mt-1">
              Public IP: {connectivity.publicIp || "Not detected"}
            </div>
          </div>

          <div className="glass-panel-2 p-4 rounded-lg">
            <div className="text-[11px] text-cv-subtext mb-2 flex items-center gap-1">
              <Cloud size={11} /> Cloud Relay
            </div>
            <div className="text-sm font-semibold">
              {connectivity.relayActive
                ? connectivity.relayMode === "named"
                  ? "Named Production Tunnel"
                  : "Automatic Relay"
                : "Standby"}
            </div>
            <div className="text-xs text-cv-subtext break-all mt-1">
              {connectivity.relayUrl ||
                "Starts automatically when direct access is unavailable."}
            </div>
          </div>

          <div className="glass-panel-2 p-4 rounded-lg">
            <div className="text-[11px] text-cv-subtext mb-2 flex items-center gap-1">
              <Link2 size={11} /> Client URL
            </div>
            <div className="text-xs text-cv-text break-all min-h-[38px]">
              {connectivity.preferredUrl || "No public route available"}
            </div>
            <button
              onClick={copyRemoteUrl}
              disabled={!connectivity.preferredUrl}
              className="cv-btn cv-btn-secondary text-xs mt-2"
            >
              <Copy size={12} /> Copy URL
            </button>
          </div>
        </div>

        {connectivity.lastError && (
          <div className="mt-4 rounded-lg border border-amber-400/30 bg-amber-400/10 p-3 text-xs text-amber-200">
            {connectivity.lastError}
          </div>
        )}
      </div>

      <div className="glass-panel p-5">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="glass-panel-2 p-4 rounded-lg">
            <div className="text-[11px] text-cv-subtext mb-2">Reachability</div>
            <label className="flex items-center justify-between text-xs py-1">
              <span>Enable Remote Access</span>
              <input
                type="checkbox"
                checked={remoteEnabled}
                onChange={(event) =>
                  setSetting(
                    "remote_access_enabled",
                    String(event.target.checked),
                  )
                }
              />
            </label>
            <label className="flex items-center justify-between text-xs py-1">
              <span>Manually Specify Public Port</span>
              <input
                type="checkbox"
                checked={manualPort}
                onChange={(event) =>
                  setSetting(
                    "remote_manually_specify_port",
                    String(event.target.checked),
                  )
                }
              />
            </label>
            <label className="text-xs block mt-2">Public Port</label>
            <input
              className="cv-input mt-1"
              value={publicPort}
              onChange={(event) =>
                setSetting(
                  "remote_public_port",
                  event.target.value.replace(/[^\d]/g, ""),
                )
              }
              placeholder="32400"
            />
          </div>

          <div className="glass-panel-2 p-4 rounded-lg">
            <div className="text-[11px] text-cv-subtext mb-2">
              Automatic NAT Traversal
            </div>
            <label className="flex items-center justify-between text-xs py-1">
              <span className="flex items-center gap-1">
                <PlugZap size={12} /> Enable UPnP
              </span>
              <input
                type="checkbox"
                checked={upnp}
                onChange={(event) =>
                  setSetting("remote_enable_upnp", String(event.target.checked))
                }
              />
            </label>
            <label className="flex items-center justify-between text-xs py-1">
              <span className="flex items-center gap-1">
                <PlugZap size={12} /> Enable NAT-PMP
              </span>
              <input
                type="checkbox"
                checked={natPmp}
                onChange={(event) =>
                  setSetting(
                    "remote_enable_natpmp",
                    String(event.target.checked),
                  )
                }
              />
            </label>
            <p className="mt-2 text-[10px] text-cv-subtext">
              Port mappings use renewable leases and are refreshed automatically.
            </p>
          </div>

          <div className="glass-panel-2 p-4 rounded-lg">
            <div className="text-[11px] text-cv-subtext mb-2">
              Relay Policy
            </div>
            <label className="flex items-center justify-between text-xs py-1">
              <span>Prefer Encrypted Relay</span>
              <input
                type="checkbox"
                checked={preferredRelay}
                onChange={(event) =>
                  setSetting(
                    "remote_preferred_relay",
                    String(event.target.checked),
                  )
                }
              />
            </label>
            <label className="flex items-center justify-between text-xs py-1">
              <span>Automatic Relay Fallback</span>
              <input
                type="checkbox"
                checked={fallback}
                onChange={(event) =>
                  setSetting(
                    "remote_allow_fallback",
                    String(event.target.checked),
                  )
                }
              />
            </label>
            <p className="mt-2 text-[10px] text-cv-subtext">
              Named tunnels use configured deployment credentials. Otherwise the
              bundled relay client can create a zero-configuration fallback URL.
            </p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-[1.1fr_0.9fr] gap-5">
        <div className="glass-panel p-5">
          <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
            <UserPlus size={16} className="text-cv-accent" /> Remote Account
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div>
              <label className="section-label">Display Name</label>
              <input
                className="cv-input"
                value={accountForm.displayName}
                onChange={(event) =>
                  setAccountForm({
                    ...accountForm,
                    displayName: event.target.value,
                  })
                }
              />
            </div>
            <div>
              <label className="section-label">Email</label>
              <input
                className="cv-input"
                value={accountForm.email}
                onChange={(event) =>
                  setAccountForm({ ...accountForm, email: event.target.value })
                }
              />
            </div>
            <div>
              <label className="section-label">Password</label>
              <input
                type="password"
                className="cv-input"
                value={accountForm.password}
                onChange={(event) =>
                  setAccountForm({ ...accountForm, password: event.target.value })
                }
              />
            </div>
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            <button
              onClick={createAccount}
              disabled={busy === "create"}
              className="cv-btn cv-btn-primary text-xs"
            >
              <UserPlus size={12} />
              {busy === "create" ? "Saving..." : "Save Account"}
            </button>
            {latestKey?.access_key && (
              <button
                onClick={copyLatestKey}
                className="cv-btn cv-btn-secondary text-xs"
              >
                <Copy size={12} /> Copy New Access Key
              </button>
            )}
          </div>
          {latestKey?.access_key && (
            <div className="mt-4 rounded-lg border border-cv-accent/30 bg-cv-accent/10 p-3">
              <div className="text-[11px] text-cv-subtext mb-1">
                New access key for {latestKey.email}
              </div>
              <code className="block text-xs text-cv-text break-all">
                {latestKey.access_key}
              </code>
            </div>
          )}
        </div>

        <div className="glass-panel p-5">
          <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
            <LogIn size={16} className="text-cv-accent" /> Access Check
          </h3>
          <div className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <input
                className="cv-input"
                placeholder="email"
                value={passwordLogin.email}
                onChange={(event) =>
                  setPasswordLogin({
                    ...passwordLogin,
                    email: event.target.value,
                  })
                }
              />
              <input
                type="password"
                className="cv-input"
                placeholder="password"
                value={passwordLogin.password}
                onChange={(event) =>
                  setPasswordLogin({
                    ...passwordLogin,
                    password: event.target.value,
                  })
                }
              />
            </div>
            <button
              onClick={loginWithPassword}
              disabled={busy === "password"}
              className="cv-btn cv-btn-secondary text-xs w-full justify-center"
            >
              <LogIn size={12} />
              {busy === "password" ? "Checking..." : "Check Email Password"}
            </button>
            <div className="flex gap-2">
              <input
                className="cv-input flex-1"
                placeholder="cvra_..."
                value={keyLogin}
                onChange={(event) => setKeyLogin(event.target.value)}
              />
              <button
                onClick={loginWithKey}
                disabled={busy === "key"}
                className="cv-btn cv-btn-secondary text-xs"
              >
                <KeyRound size={12} /> Key
              </button>
            </div>
            <div className="text-xs text-cv-subtext">
              Last accepted:{" "}
              {lastPrincipal
                ? `${lastPrincipal.email} via ${lastPrincipal.auth_method}`
                : "None"}
            </div>
          </div>
        </div>
      </div>

      <div className="glass-panel p-5">
        <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
          <ShieldCheck size={16} className="text-cv-accent" /> Authorized Remote
          Users
        </h3>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
          {accounts.map((account) => (
            <div
              key={account.id}
              className="rounded-lg border border-white/10 bg-white/[0.02] p-3"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="text-sm font-semibold truncate">
                    {account.display_name || account.email}
                  </div>
                  <div className="text-xs text-cv-subtext truncate">
                    {account.email}
                  </div>
                  <div className="text-[11px] text-cv-subtext">
                    Key: ...{account.access_key_preview}
                  </div>
                  <div className="text-[11px] text-cv-subtext">
                    Last login: {account.last_login || "Never"}
                  </div>
                </div>
                <div
                  className={`text-[11px] px-2 py-1 rounded ${
                    account.enabled
                      ? "bg-green-500/15 text-green-300"
                      : "bg-cv-danger/15 text-cv-danger"
                  }`}
                >
                  {account.enabled ? "Enabled" : "Disabled"}
                </div>
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                <button
                  onClick={() => rotateKey(account.email)}
                  disabled={busy === `rotate:${account.email}`}
                  className="cv-btn cv-btn-secondary text-xs"
                >
                  <RotateCw size={12} /> Rotate Key
                </button>
                <button
                  onClick={() => toggleAccount(account)}
                  disabled={busy === `toggle:${account.email}`}
                  className="cv-btn cv-btn-secondary text-xs"
                >
                  <Power size={12} /> {account.enabled ? "Disable" : "Enable"}
                </button>
              </div>
            </div>
          ))}
          {accounts.length === 0 && (
            <div className="rounded-lg border border-white/10 bg-white/[0.02] p-4 text-xs text-cv-subtext">
              No remote users saved.
            </div>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
        <div className="glass-panel p-5">
          <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
            <ShieldCheck size={16} className="text-cv-accent" /> Secure
            Connections
          </h3>
          <div className="space-y-2">
            {secureOptions.map((option) => (
              <button
                key={option.value}
                onClick={() =>
                  setSetting("remote_secure_connections", option.value)
                }
                className={`w-full text-left rounded-lg p-3 border transition ${
                  secureMode === option.value
                    ? "border-cv-accent/40 bg-cv-accent/10"
                    : "border-white/10 bg-white/[0.02]"
                }`}
              >
                <div className="text-xs font-semibold flex items-center gap-2">
                  <Lock size={12} />
                  {option.label}
                </div>
                <div className="text-[11px] text-cv-subtext mt-1">
                  {option.desc}
                </div>
              </button>
            ))}
          </div>
        </div>

        <div className="glass-panel p-5">
          <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
            <SlidersHorizontal size={16} className="text-cv-accent" /> Streaming
            Constraints
          </h3>
          <label className="section-label">Internet Upload Limit (Mbps)</label>
          <input
            className="cv-input mb-3"
            value={uploadLimit}
            onChange={(event) =>
              setSetting(
                "remote_upload_limit_mbps",
                event.target.value.replace(/[^\d]/g, ""),
              )
            }
            placeholder="20"
          />
          <label className="section-label">
            Allowed Networks (CIDR, comma-separated)
          </label>
          <textarea
            className="cv-input min-h-[100px]"
            value={allowedNetworks}
            onChange={(event) =>
              setSetting("remote_allowed_networks", event.target.value)
            }
            placeholder="192.168.1.0/24,10.0.0.0/8"
          />
          <div className="text-[10px] text-cv-subtext mt-2 flex items-center gap-1">
            <Globe size={10} />
            Account authentication remains mandatory through direct and relayed
            connections.
          </div>
        </div>
      </div>

      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        className="glass-panel p-4 text-xs text-cv-subtext"
      >
        Remote profile saved:{" "}
        <span className="text-cv-text">
          {remoteEnabled ? "Enabled" : "Disabled"} / {secureMode} secure / port{" "}
          {publicPort || "n/a"} /{" "}
          {connectivity.relayActive
            ? `${connectivity.relayMode || "cloud"} relay active`
            : connectivity.directAvailable
              ? `${connectivity.directMethod || "direct"} active`
              : "route pending"}
        </span>
      </motion.div>
    </div>
  );
}
