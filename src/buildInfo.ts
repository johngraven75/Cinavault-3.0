import manifest from "../build-version.json";

export interface CinaVaultBuildInfo {
  name: string;
  version: string;
  build: string;
  displayName: string;
  releaseCycle: string;
  releaseTag: string;
  edition: string;
}

export const BUILD_INFO: CinaVaultBuildInfo = Object.freeze({
  name: manifest.productName,
  version: manifest.semanticVersion,
  build: manifest.displayBuild,
  displayName: manifest.displayName,
  releaseCycle: manifest.releaseCycle,
  releaseTag: manifest.releaseTag,
  edition: manifest.edition,
});

export const BUILD_DATASET_VALUE = manifest.releaseTag.replaceAll(".", "-");
export const WINDOW_TITLE = `${manifest.productName} · ${manifest.displayName}`;
