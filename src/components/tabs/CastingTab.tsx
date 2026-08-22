import { useEffect, useMemo, useState } from "react";
import {
  Airplay,
  Cast,
  Loader2,
  Pause,
  Play,
  RefreshCw,
  Router,
  Tv,
  Volume2,
  Wifi,
  X,
} from "lucide-react";
import { useAppStore } from "../../store/appStore";
import {
  type CastingDevice,
  type CastingDeviceType,
  type CastingSession,
  connectCastingDevice,
  disconnectCastingDevice,
  discoverCastingDevices,
  getCastingSession,
  startCasting,
  updateCastingPlayback,
} from "../../services/castingService";

const DEVICE_META: Record<CastingDeviceType, { label: string; icon: typeof Cast }> = {
  chromecast: { label: "Chromecast", icon: Cast },
  airplay: { label: "AirPlay", icon: Airplay },
  smartview: { label: "Smart View", icon: Tv },
  dlna: { label: "DLNA", icon: Router },
};

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function CastingTab() {
  const { selectedMedia, addStatusMessage } = useAppStore();
  const [devices, setDevices] = useState<CastingDevice[]>([]);
  const [scanning, setScanning] = useState(false);
  const [connectingId, setConnectingId] = useState<string | null>(null);
  const [activeDevice, setActiveDevice] = useState<CastingDevice | null>(null);
  const [session, setSession] = useState<CastingSession | null>(() => getCastingSession());
  const [mediaUrl, setMediaUrl] = useState(selectedMedia?.file_path ?? "");
  const [message, setMessage] = useState("Ready to discover nearby devices");

  useEffect(() => {
    setMediaUrl(selectedMedia?.file_path ?? "");
  }, [selectedMedia?.id, selectedMedia?.file_path]);

  const counts = useMemo(
    () =>
      devices.reduce<Record<CastingDeviceType, number>>(
        (result, device) => ({ ...result, [device.type]: result[device.type] + 1 }),
        { chromecast: 0, airplay: 0, smartview: 0, dlna: 0 },
      ),
    [devices],
  );

  const scan = async (): Promise<void> => {
    setScanning(true);
    setMessage("Scanning the local network…");
    try {
      const discovered = await discoverCastingDevices();
      setDevices(discovered);
      setActiveDevice(discovered.find((device) => device.connected) ?? null);
      setMessage(
        discovered.length > 0
          ? `${discovered.length} compatible device${discovered.length === 1 ? "" : "s"} found`
          : "No compatible devices found. Confirm all devices use the same network.",
      );
    } catch (error) {
      const detail = errorMessage(error);
      setMessage(`Device discovery failed: ${detail}`);
      addStatusMessage(`Casting discovery failed: ${detail}`);
    } finally {
      setScanning(false);
    }
  };

  useEffect(() => {
    void scan();
  }, []);

  const connect = async (device: CastingDevice): Promise<void> => {
    setConnectingId(device.id);
    setMessage(`Connecting to ${device.name}…`);
    try {
      const connected = await connectCastingDevice(device);
      setActiveDevice(connected);
      setDevices((current) =>
        current.map((item) => ({
          ...item,
          connected: item.id === connected.id,
          state: item.id === connected.id ? "connected" : "available",
        })),
      );
      setMessage(`${connected.name} connected`);
      addStatusMessage(`${connected.name} connected for casting`);
    } catch (error) {
      const detail = errorMessage(error);
      setMessage(`Unable to connect to ${device.name}: ${detail}`);
      addStatusMessage(`Casting connection failed: ${detail}`);
    } finally {
      setConnectingId(null);
    }
  };

  const disconnect = async (): Promise<void> => {
    if (!activeDevice) return;
    const current = activeDevice;
    try {
      await disconnectCastingDevice(current);
      setActiveDevice(null);
      setSession(null);
      setDevices((items) =>
        items.map((item) =>
          item.id === current.id
            ? { ...item, connected: false, state: "available" }
            : item,
        ),
      );
      setMessage(`${current.name} disconnected`);
      addStatusMessage(`${current.name} disconnected`);
    } catch (error) {
      const detail = errorMessage(error);
      setMessage(`Disconnect failed: ${detail}`);
      addStatusMessage(`Casting disconnect failed: ${detail}`);
    }
  };

  const togglePlayback = async (): Promise<void> => {
    if (!activeDevice) return;
    try {
      if (!session) {
        const source = mediaUrl.trim();
        if (!source) {
          setMessage("Select library media or enter a reachable media URL.");
          return;
        }
        const next: CastingSession = {
          device: activeDevice,
          mediaUrl: source,
          title: selectedMedia?.title || "CinaVault playback session",
          contentType: "video/mp4",
          paused: false,
          volume: 0.8,
          currentTime: 0,
        };
        const result = await startCasting(next);
        setSession(next);
        setMessage(result);
        addStatusMessage(result);
        return;
      }

      const updated = await updateCastingPlayback({ paused: !session.paused });
      if (updated) setSession(updated);
    } catch (error) {
      const detail = errorMessage(error);
      setMessage(`Playback failed: ${detail}`);
      addStatusMessage(`Casting playback failed: ${detail}`);
    }
  };

  const changeVolume = async (volume: number): Promise<void> => {
    try {
      const updated = await updateCastingPlayback({ volume });
      if (updated) setSession(updated);
    } catch (error) {
      setMessage(`Volume update failed: ${errorMessage(error)}`);
    }
  };

  return (
    <section className="space-y-5" data-testid="cinavault-casting-tab">
      <div className="grid gap-3 md:grid-cols-4">
        {(Object.keys(DEVICE_META) as CastingDeviceType[]).map((type) => {
          const meta = DEVICE_META[type];
          const Icon = meta.icon;
          return (
            <div key={type} className="rounded-2xl border border-white/10 bg-black/20 p-4">
              <div className="flex items-center justify-between">
                <Icon size={18} className="text-cyan-100" />
                <span className="text-2xl font-black">{counts[type]}</span>
              </div>
              <div className="mt-2 text-xs font-bold uppercase tracking-[0.18em] text-cv-subtext">
                {meta.label}
              </div>
            </div>
          );
        })}
      </div>

      <div className="rounded-[26px] border border-white/10 bg-black/25 p-5">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 text-lg font-black">
              <Wifi size={18} className="text-cyan-100" /> Available Devices
            </div>
            <p className="mt-1 text-sm text-cv-subtext">{message}</p>
          </div>
          <button
            type="button"
            onClick={() => void scan()}
            disabled={scanning}
            className="flex h-11 items-center gap-2 rounded-2xl border border-white/10 bg-white/[0.06] px-4 font-bold hover:bg-white/[0.10] disabled:opacity-50"
          >
            {scanning ? <Loader2 size={16} className="animate-spin" /> : <RefreshCw size={16} />}
            {scanning ? "Scanning" : "Scan Devices"}
          </button>
        </div>

        <div className="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {devices.map((device) => {
            const meta = DEVICE_META[device.type];
            const Icon = meta.icon;
            const active = activeDevice?.id === device.id;
            const busy = connectingId === device.id;
            return (
              <article
                key={device.id}
                className={`rounded-[22px] border p-5 ${
                  active
                    ? "border-cyan-200/55 bg-cyan-200/[0.10] shadow-[0_0_30px_rgba(0,234,255,0.14)]"
                    : "border-white/10 bg-white/[0.035]"
                }`}
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="grid h-12 w-12 place-items-center rounded-2xl border border-white/10 bg-black/30">
                    <Icon size={22} className="text-cyan-100" />
                  </div>
                  <span className="rounded-full bg-white/[0.06] px-3 py-1 text-[10px] font-bold uppercase tracking-[0.16em] text-cv-subtext">
                    {active ? "Connected" : meta.label}
                  </span>
                </div>
                <h3 className="mt-4 truncate text-lg font-black">{device.name}</h3>
                <p className="mt-1 truncate text-xs text-cv-subtext">
                  {device.model || device.address || "Network playback device"}
                </p>
                <button
                  type="button"
                  disabled={active || busy}
                  onClick={() => void connect(device)}
                  className="mt-5 flex h-11 w-full items-center justify-center gap-2 rounded-2xl border border-cyan-200/25 bg-cyan-200/[0.08] font-bold text-cyan-100 hover:bg-cyan-200/[0.14] disabled:opacity-50"
                >
                  {busy ? <Loader2 size={16} className="animate-spin" /> : <Cast size={16} />}
                  {active ? "Connected" : "Connect"}
                </button>
              </article>
            );
          })}
        </div>
      </div>

      <div className="rounded-[26px] border border-white/10 bg-[linear-gradient(120deg,rgba(0,234,255,0.10),rgba(255,255,255,0.03))] p-5">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <div className="text-xs font-bold uppercase tracking-[0.2em] text-cyan-100">Current Device</div>
            <h3 className="mt-1 text-xl font-black">{activeDevice?.name || "No device connected"}</h3>
          </div>
          {activeDevice && (
            <button
              type="button"
              onClick={() => void disconnect()}
              className="flex h-10 items-center gap-2 rounded-xl border border-rose-300/20 bg-rose-300/[0.08] px-4 text-sm font-bold text-rose-100"
            >
              <X size={15} /> Disconnect
            </button>
          )}
        </div>

        <label className="mt-5 block text-xs font-bold uppercase tracking-[0.18em] text-cv-subtext">
          Media source
          <input
            value={mediaUrl}
            onChange={(event) => setMediaUrl(event.target.value)}
            placeholder="Select library media or enter a reachable URL"
            className="mt-2 h-12 w-full rounded-2xl border border-white/10 bg-black/30 px-4 text-sm text-cv-text outline-none focus:border-cyan-200/50"
          />
        </label>

        <div className="mt-5 grid gap-4 lg:grid-cols-[auto_1fr_240px] lg:items-center">
          <button
            type="button"
            disabled={!activeDevice || (!session && !mediaUrl.trim())}
            onClick={() => void togglePlayback()}
            className="grid h-14 w-14 place-items-center rounded-full border border-cyan-100/30 bg-cyan-100/[0.12] disabled:opacity-35"
            aria-label={session?.paused ? "Resume casting" : "Start or pause casting"}
          >
            {session?.paused ? <Play size={22} /> : <Pause size={22} />}
          </button>
          <div>
            <div className="h-2 overflow-hidden rounded-full bg-white/10">
              <div className="h-full w-[28%] rounded-full bg-cyan-100" />
            </div>
            <div className="mt-2 flex justify-between text-xs text-cv-subtext">
              <span>{session?.title || selectedMedia?.title || "Select media to begin casting"}</span>
              <span>{activeDevice ? "Ready" : "Offline"}</span>
            </div>
          </div>
          <label className="flex items-center gap-3 text-sm text-cv-subtext">
            <Volume2 size={18} />
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={session?.volume ?? 0.8}
              disabled={!session}
              onChange={(event) => void changeVolume(Number(event.target.value))}
              className="w-full"
            />
          </label>
        </div>
      </div>
    </section>
  );
}
