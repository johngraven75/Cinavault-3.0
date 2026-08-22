export type CinaVaultServerFeature =
  | "media-library"
  | "users"
  | "permissions"
  | "metadata"
  | "poster-retrieval"
  | "transcoding"
  | "remote-access"
  | "dlna"
  | "live-tv"
  | "plugins"
  | "downloads"
  | "duplicate-management"
  | "filename-normalization"
  | "ai-media-agent";

export type CinaVaultServerSettings = {
  enabled: true;
  serverName: "CinaVault Server";
  port: number;
  preserveJellyfinCompatibility: true;
  features: CinaVaultServerFeature[];
};

export const CINAVAULT_SERVER_SETTINGS: CinaVaultServerSettings = {
  enabled: true,
  serverName: "CinaVault Server",
  port: 8097,
  preserveJellyfinCompatibility: true,
  features: [
    "media-library",
    "users",
    "permissions",
    "metadata",
    "poster-retrieval",
    "transcoding",
    "remote-access",
    "dlna",
    "live-tv",
    "plugins",
    "downloads",
    "duplicate-management",
    "filename-normalization",
    "ai-media-agent",
  ],
};

export function getCinaVaultServerSettings() {
  return CINAVAULT_SERVER_SETTINGS;
}

export function shouldKeepCurrentAppFeatures() {
  return true;
}

export function shouldUseCinaVaultServerFirst() {
  return true;
}
