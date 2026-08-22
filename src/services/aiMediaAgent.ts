export type MediaAgentTask =
  | "identify-media"
  | "retrieve-posters"
  | "enrich-metadata"
  | "normalize-filename"
  | "detect-duplicates"
  | "quarantine-duplicates";

export type MediaAgentResult = {
  enabled: true;
  handledBy: "CinaVault AI Media Agent";
  task: MediaAgentTask;
  safeMode: true;
  message: string;
};

export const AI_MEDIA_AGENT_ENABLED = true;

export function normalizeMediaFilename(input: string): string {
  return input
    .replace(/\.[^.]+$/, "")
    .replace(/[._-]+/g, " ")
    .replace(
      /\b(1080p|720p|2160p|4k|bluray|webrip|x264|x265|h264|h265|aac|dts)\b/gi,
      "",
    )
    .replace(/\s+/g, " ")
    .trim();
}

export function buildMediaAgentTask(task: MediaAgentTask): MediaAgentResult {
  return {
    enabled: true,
    handledBy: "CinaVault AI Media Agent",
    task,
    safeMode: true,
    message:
      "AI Media Agent is permanently enabled for media identification, poster retrieval, metadata enrichment, filename normalization, and duplicate quarantine.",
  };
}

export function shouldQuarantineDuplicate(
  hashA?: string,
  hashB?: string,
): boolean {
  return Boolean(hashA && hashB && hashA === hashB);
}
