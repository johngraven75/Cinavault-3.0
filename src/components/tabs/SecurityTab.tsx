// CinaVault Premium — Security Tab (bundled WireGuard + Windows Defender)
import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { motion } from "framer-motion";
import { useAppStore } from "../../store/appStore";
import {
  Shield,
  Wifi,
  WifiOff,
  Scan,
  RefreshCw,
  Download,
  Loader,
  FileKey,
  CheckCircle,
  AlertTriangle,
} from "lucide-react";

type VpnProfile = {
  name: string;
  path: string;
  active: boolean;
};

type VpnStatus = {
  installed: boolean;
  engineBundled?: boolean;
  connected: boolean;
  activeProfile?: string | null;
  profiles: VpnProfile[];
  details?: string;
};

export default function SecurityTab() {
  const { vpnConnected, vpnLocation, setVpnStatus, addStatusMessage } = useAppStore();
  const [vpnLoading, setVpnLoading] = useState(false);
  const [avScanning, setAvScanning] = useState(false);
  const [vpnInstalled, setVpnInstalled] = useState<boolean | null>(null);
  const [profiles, setProfiles] = useState<VpnProfile[]>([]);
  const [selectedProfile, setSelectedProfile] = useState("");
  const [vpnDetails, setVpnDetails] = useState("");

  useEffect(() => {
    void checkVpnStatus();
  }, []);

  const checkVpnStatus = async () => {
    try {
      const status = await invoke<VpnStatus>("vpn_status");
      setVpnInstalled(status.installed);
      setProfiles(status.profiles ?? []);
      setVpnDetails(status.details ?? "");
      const active = status.activeProfile ?? status.profiles?.find((profile) => profile.active)?.name ?? "";
      setSelectedProfile((current) => current || active || status.profiles?.[0]?.name || "");
      setVpnStatus(status.connected, active);
    } catch (error) {
      setVpnInstalled(false);
      setVpnDetails(String(error));
    }
  };

  const importProfile = async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "WireGuard profile", extensions: ["conf"] }],
    });
    if (!selected || Array.isArray(selected)) return;

    setVpnLoading(true);
    try {
      const profile = await invoke<VpnProfile>("vpn_import_profile", {
        sourcePath: selected,
      });
      setSelectedProfile(profile.name);
      addStatusMessage(`WireGuard profile stored permanently: ${profile.name}`);
      await checkVpnStatus();
    } catch (error) {
      addStatusMessage(`WireGuard profile import failed: ${error}`);
    } finally {
      setVpnLoading(false);
    }
  };

  const connectVpn = async () => {
    if (!selectedProfile) {
      addStatusMessage("Import and select a WireGuard profile before connecting");
      return;
    }
    setVpnLoading(true);
    addStatusMessage(`Connecting WireGuard profile ${selectedProfile}...`);
    try {
      const result = await invoke<{ status: string }>("vpn_connect", {
        profile: selectedProfile,
      });
      if (result.status === "connected") {
        setVpnStatus(true, selectedProfile);
        addStatusMessage(`VPN connected: ${selectedProfile}`);
      } else {
        addStatusMessage(`VPN connection state: ${result.status}`);
      }
      await checkVpnStatus();
    } catch (error) {
      addStatusMessage(`VPN error: ${error}`);
    } finally {
      setVpnLoading(false);
    }
  };

  const disconnectVpn = async () => {
    setVpnLoading(true);
    try {
      await invoke("vpn_disconnect");
      setVpnStatus(false, "");
      addStatusMessage("VPN disconnected");
      await checkVpnStatus();
    } catch (error) {
      addStatusMessage(`VPN disconnect error: ${error}`);
    } finally {
      setVpnLoading(false);
    }
  };

  const runScan = async () => {
    setAvScanning(true);
    addStatusMessage("Starting antivirus quick scan...");
    try {
      const result = await invoke<{ status: string }>("run_antivirus_scan");
      addStatusMessage(`Scan ${result.status}`);
    } catch (error) {
      addStatusMessage(`Scan failed: ${error}`);
    } finally {
      setAvScanning(false);
    }
  };

  const updateSignatures = async () => {
    addStatusMessage("Updating antivirus signatures...");
    try {
      const result = await invoke<{ status: string }>("update_av_signatures");
      addStatusMessage(`Signatures ${result.status}`);
    } catch (error) {
      addStatusMessage(`Update failed: ${error}`);
    }
  };

  return (
    <div className="space-y-5">
      <div className="glass-panel p-5">
        <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
          <Shield size={16} className="text-cv-accent" /> VPN — Bundled WireGuard
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          <div className="glass-panel-2 p-5 rounded-lg text-center">
            <motion.div
              animate={{ scale: vpnConnected ? [1, 1.1, 1] : 1 }}
              transition={{ duration: 2, repeat: vpnConnected ? Infinity : 0 }}
              className={`w-20 h-20 rounded-full mx-auto mb-4 flex items-center justify-center ${
                vpnConnected
                  ? "bg-green-500/20 ring-2 ring-green-500/40"
                  : "bg-cv-danger/20 ring-2 ring-cv-danger/40"
              }`}
            >
              {vpnConnected ? (
                <Wifi size={36} className="text-green-500" />
              ) : (
                <WifiOff size={36} className="text-cv-danger" />
              )}
            </motion.div>
            <div className="text-lg font-bold mb-1">
              {vpnConnected ? "Connected" : "Disconnected"}
            </div>
            {vpnConnected && vpnLocation && (
              <div className="text-sm text-cv-accent">{vpnLocation}</div>
            )}
            <div className="text-[10px] text-cv-subtext mt-1 flex items-center justify-center gap-1">
              {vpnInstalled ? <CheckCircle size={11} /> : <AlertTriangle size={11} />}
              {vpnInstalled ? "Bundled engine ready" : "Bundled engine missing"}
            </div>
            {vpnDetails && <div className="text-[10px] text-cv-subtext mt-2">{vpnDetails}</div>}

            <div className="flex gap-2 mt-4 justify-center">
              {!vpnConnected ? (
                <button
                  onClick={connectVpn}
                  disabled={vpnLoading || !vpnInstalled || !selectedProfile}
                  className="cv-btn cv-btn-primary"
                >
                  {vpnLoading ? <Loader size={14} className="animate-spin" /> : <Wifi size={14} />}
                  Connect
                </button>
              ) : (
                <button onClick={disconnectVpn} disabled={vpnLoading} className="cv-btn cv-btn-danger">
                  {vpnLoading ? <Loader size={14} className="animate-spin" /> : <WifiOff size={14} />}
                  Disconnect
                </button>
              )}
            </div>
          </div>

          <div className="glass-panel-2 p-5 rounded-lg space-y-4">
            <div>
              <label className="section-label mb-2 block">Permanent profile</label>
              <select
                value={selectedProfile}
                onChange={(event) => setSelectedProfile(event.target.value)}
                className="cv-input w-full"
                disabled={vpnConnected}
              >
                <option value="">Select a stored profile</option>
                {profiles.map((profile) => (
                  <option key={profile.name} value={profile.name}>
                    {profile.name}{profile.active ? " — active" : ""}
                  </option>
                ))}
              </select>
            </div>
            <button onClick={importProfile} disabled={vpnLoading || vpnConnected} className="cv-btn cv-btn-secondary w-full">
              <FileKey size={14} /> Import .conf profile
            </button>
            <p className="text-xs text-cv-subtext leading-relaxed">
              Imported profiles are copied into CinaVault application data and retained across restarts and upgrades. Private keys are never committed to the repository or embedded in public installers.
            </p>
          </div>
        </div>
      </div>

      <div className="glass-panel p-5">
        <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
          <Scan size={16} className="text-cv-accent" /> Antivirus — Windows Defender
        </h3>
        <div className="flex flex-wrap gap-3">
          <button onClick={runScan} disabled={avScanning} className="cv-btn cv-btn-primary">
            {avScanning ? <Loader size={14} className="animate-spin" /> : <Scan size={14} />}
            {avScanning ? "Scanning..." : "Quick Scan"}
          </button>
          <button onClick={updateSignatures} className="cv-btn cv-btn-secondary">
            <RefreshCw size={14} /> Update Signatures
          </button>
          <button onClick={importProfile} disabled={vpnLoading || vpnConnected} className="cv-btn cv-btn-secondary">
            <Download size={14} /> Add VPN Profile
          </button>
        </div>
      </div>
    </div>
  );
}
