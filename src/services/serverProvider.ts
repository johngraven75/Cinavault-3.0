import {
  getCinaVaultServerSettings,
  shouldKeepCurrentAppFeatures,
  shouldUseCinaVaultServerFirst,
} from "../server/cinavaultServer";

export function getPreferredMediaServer() {
  return {
    primary: "cinavault-server",
    fallback: "jellyfin-compatible",
    settings: getCinaVaultServerSettings(),
    keepExistingFeatures: shouldKeepCurrentAppFeatures(),
    useCinaVaultServerFirst: shouldUseCinaVaultServerFirst(),
  };
}
