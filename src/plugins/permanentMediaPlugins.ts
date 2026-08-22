// CinaVault Premium — Permanent Media Plugins (Build 155)
export interface PermanentPlugin {
  id: string;
  name: string;
  version: string;
  description: string;
  category: "transcoding" | "download" | "analysis" | "playback" | "metadata";
  executable: string;
  windowsExecutable: string;
  checkArgs: string[];
  permanent: true;
  enabled: true;
  installed: true;
  startup: true;
  required: true;
  autoInstall: boolean;
  capabilities: string[];
}

export const permanentMediaPlugins: PermanentPlugin[] = [
  {
    id: "ffmpeg",
    name: "FFmpeg",
    version: "6.x",
    description:
      "Industry-standard multimedia framework. Powers all transcoding, thumbnail generation, chapter extraction, and format conversion.",
    category: "transcoding",
    executable: "ffmpeg",
    windowsExecutable: "ffmpeg.exe",
    checkArgs: ["-version"],
    permanent: true,
    enabled: true,
    installed: true,
    startup: true,
    required: true,
    autoInstall: true,
    capabilities: [
      "transcode",
      "thumbnail",
      "chapter_extract",
      "format_probe",
      "subtitle_extract",
      "audio_extract",
      "hls_stream",
    ],
  },
  {
    id: "ffprobe",
    name: "FFprobe",
    version: "6.x",
    description:
      "Media analysis companion to FFmpeg. Extracts codec, resolution, duration, and embedded metadata from all media files.",
    category: "analysis",
    executable: "ffprobe",
    windowsExecutable: "ffprobe.exe",
    checkArgs: ["-version"],
    permanent: true,
    enabled: true,
    installed: true,
    startup: true,
    required: true,
    autoInstall: true,
    capabilities: [
      "media_probe",
      "codec_detect",
      "resolution_detect",
      "duration_detect",
      "embedded_title_extract",
      "stream_info",
    ],
  },
  {
    id: "yt-dlp",
    name: "yt-dlp",
    version: "latest",
    description:
      "Premier video downloader supporting 1,000+ sites. Powers all download operations including YouTube, Vimeo, and streaming platforms.",
    category: "download",
    executable: "yt-dlp",
    windowsExecutable: "yt-dlp.exe",
    checkArgs: ["--version"],
    permanent: true,
    enabled: true,
    installed: true,
    startup: true,
    required: true,
    autoInstall: true,
    capabilities: [
      "video_download",
      "audio_extract",
      "playlist_download",
      "subtitle_download",
      "thumbnail_download",
      "format_selection",
    ],
  },
  {
    id: "mediainfo",
    name: "MediaInfo",
    version: "latest",
    description:
      "Detailed technical and tag information about video and audio files. Used for deep media analysis and library quality checks.",
    category: "analysis",
    executable: "mediainfo",
    windowsExecutable: "MediaInfo.exe",
    checkArgs: ["--Version"],
    permanent: true,
    enabled: true,
    installed: true,
    startup: true,
    required: true,
    autoInstall: true,
    capabilities: [
      "deep_media_analysis",
      "tag_read",
      "hdr_detect",
      "dolby_detect",
      "chapter_read",
      "track_info",
    ],
  },
  {
    id: "mkvtoolnix",
    name: "MKVToolNix",
    version: "latest",
    description:
      "Tools for creating, editing, and inspecting Matroska (MKV) files. Used for subtitle and chapter manipulation.",
    category: "transcoding",
    executable: "mkvmerge",
    windowsExecutable: "mkvmerge.exe",
    checkArgs: ["--version"],
    permanent: true,
    enabled: true,
    installed: true,
    startup: true,
    required: true,
    autoInstall: true,
    capabilities: [
      "mkv_merge",
      "subtitle_mux",
      "chapter_write",
      "attachment_add",
      "track_remove",
    ],
  },
];

export function arePermanentMediaPluginsReady(): boolean {
  return (
    permanentMediaPlugins.length > 0 &&
    permanentMediaPlugins.every((p) => p.enabled)
  );
}
export function getStartupMediaPlugins(): PermanentPlugin[] {
  return permanentMediaPlugins.filter((p) => p.autoInstall);
}
export function getPluginById(id: string): PermanentPlugin | undefined {
  return permanentMediaPlugins.find((p) => p.id === id);
}
export function getPluginsByCategory(
  category: PermanentPlugin["category"],
): PermanentPlugin[] {
  return permanentMediaPlugins.filter((p) => p.category === category);
}
export function hasCapability(capability: string): boolean {
  return permanentMediaPlugins.some((p) => p.capabilities.includes(capability));
}
