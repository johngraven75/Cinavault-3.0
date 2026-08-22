import { invoke } from "@tauri-apps/api/core";

export type CastingDeviceType = "chromecast" | "airplay" | "smartview" | "dlna";
export type CastingConnectionState =
  | "available"
  | "connecting"
  | "connected"
  | "disconnecting"
  | "error";

export interface CastingDevice {
  id: string;
  name: string;
  address?: string;
  port?: number;
  type: CastingDeviceType;
  connected: boolean;
  state?: CastingConnectionState;
  model?: string;
  lastSeen?: string;
}

export interface CastingSession {
  device: CastingDevice;
  mediaUrl: string;
  title?: string;
  contentType?: string;
  currentTime?: number;
  duration?: number;
  volume?: number;
  paused?: boolean;
}

const SESSION_STORAGE_KEY = "cinavault_casting_session";
const DEVICE_STORAGE_KEY = "cinavault_casting_devices";

function isDesktopRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function assertDesktopRuntime(): void {
  if (!isDesktopRuntime()) {
    throw new Error("Casting is available in the installed desktop application.");
  }
}

function normalizeDevice(device: CastingDevice): CastingDevice {
  return {
    ...device,
    connected: Boolean(device.connected),
    state: device.state ?? (device.connected ? "connected" : "available"),
    lastSeen: device.lastSeen ?? new Date().toISOString(),
  };
}

function dedupeDevices(devices: CastingDevice[]): CastingDevice[] {
  const byId = new Map<string, CastingDevice>();
  for (const device of devices) {
    const normalized = normalizeDevice(device);
    const key =
      normalized.id ||
      `${normalized.type}:${normalized.address ?? "unknown"}:${normalized.port ?? ""}`;
    byId.set(key, normalized);
  }
  return [...byId.values()].sort((left, right) => left.name.localeCompare(right.name));
}

function readCachedDevices(): CastingDevice[] {
  try {
    const raw = localStorage.getItem(DEVICE_STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? dedupeDevices(parsed as CastingDevice[]) : [];
  } catch {
    return [];
  }
}

function cacheDevices(devices: CastingDevice[]): void {
  try {
    localStorage.setItem(DEVICE_STORAGE_KEY, JSON.stringify(devices));
  } catch {
    // Optional cache.
  }
}

export async function discoverCastingDevices(): Promise<CastingDevice[]> {
  if (!isDesktopRuntime()) return readCachedDevices();
  const devices = dedupeDevices(
    await invoke<CastingDevice[]>("discover_casting_devices"),
  );
  cacheDevices(devices);
  return devices;
}

export async function connectCastingDevice(
  device: CastingDevice,
): Promise<CastingDevice> {
  assertDesktopRuntime();
  const connected = normalizeDevice(
    await invoke<CastingDevice>("connect_casting_device", { device }),
  );
  const next = dedupeDevices([
    ...readCachedDevices().map((candidate) => ({
      ...candidate,
      connected: candidate.id === connected.id,
      state:
        candidate.id === connected.id
          ? ("connected" as const)
          : ("available" as const),
    })),
    connected,
  ]);
  cacheDevices(next);
  return connected;
}

export async function disconnectCastingDevice(
  device: CastingDevice,
): Promise<CastingDevice> {
  assertDesktopRuntime();
  const disconnected = normalizeDevice(
    await invoke<CastingDevice>("disconnect_casting_device", { device }),
  );
  cacheDevices(
    readCachedDevices().map((candidate) =>
      candidate.id === disconnected.id ? disconnected : candidate,
    ),
  );
  clearCastingSession();
  return disconnected;
}

export async function startCasting(session: CastingSession): Promise<string> {
  assertDesktopRuntime();
  const mediaUrl = session.mediaUrl.trim();
  if (!mediaUrl) throw new Error("Select media before starting playback.");

  const normalizedSession: CastingSession = {
    ...session,
    mediaUrl,
    volume: session.volume ?? 0.8,
    currentTime: session.currentTime ?? 0,
    paused: session.paused ?? false,
  };
  const result = await invoke<string>("start_casting", {
    session: normalizedSession,
  });
  saveCastingSession(normalizedSession);
  return result;
}

export async function updateCastingPlayback(
  patch: Partial<Pick<CastingSession, "currentTime" | "volume" | "paused">>,
): Promise<CastingSession | null> {
  assertDesktopRuntime();
  const current = getCastingSession();
  if (!current) return null;
  const updated = await invoke<CastingSession>("update_casting_playback", {
    patch,
  });
  saveCastingSession(updated);
  return updated;
}

export function getCastingSession(): CastingSession | null {
  try {
    const raw = localStorage.getItem(SESSION_STORAGE_KEY);
    return raw ? (JSON.parse(raw) as CastingSession) : null;
  } catch {
    return null;
  }
}

export function saveCastingSession(session: CastingSession): void {
  try {
    localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(session));
  } catch {
    // Optional persistence.
  }
}

export function clearCastingSession(): void {
  try {
    localStorage.removeItem(SESSION_STORAGE_KEY);
  } catch {
    // Optional persistence.
  }
}
