export type MediaRowItem = {
  path?: string;
  filePath?: string;
  name?: string;
  title?: string;
  type?: string;
  mediaType?: string;
  mimeType?: string;
  isPoster?: boolean;
  isBackdrop?: boolean;
  isThumbnail?: boolean;
};

const IMAGE_EXTENSIONS = /\.(jpg|jpeg|png|webp|gif|bmp|tiff|avif)$/i;
const VIDEO_EXTENSIONS =
  /\.(mp4|mkv|avi|mov|wmv|m4v|webm|ts|m2ts|mpg|mpeg|flv)$/i;
const SIDECAR_NAMES =
  /(^|[\\\/\s._-])(poster|cover|folder|fanart|backdrop|banner|thumb|thumbnail|logo|clearlogo|clearart|disc|landscape|screenshot|chapter|scene)([\\\/\s._-]|$)/i;

export function getMediaRowItemPath(item: MediaRowItem): string {
  return item.path || item.filePath || item.name || item.title || "";
}

export function isSidecarArtworkImage(item: MediaRowItem): boolean {
  const path = getMediaRowItemPath(item);
  const mime = item.mimeType || "";
  const kind = item.mediaType || item.type || "";

  return Boolean(
    item.isPoster ||
    item.isBackdrop ||
    item.isThumbnail ||
    IMAGE_EXTENSIONS.test(path) ||
    SIDECAR_NAMES.test(path) ||
    /image|photo|picture|poster|artwork|backdrop|thumbnail/i.test(mime) ||
    /image|photo|picture|poster|artwork|backdrop|thumbnail/i.test(kind),
  );
}

export function isPlayableMediaItem(item: MediaRowItem): boolean {
  const path = getMediaRowItemPath(item);
  const mime = item.mimeType || "";
  const kind = item.mediaType || item.type || "";

  return Boolean(
    VIDEO_EXTENSIONS.test(path) ||
    /video/i.test(mime) ||
    (/movie|episode|video/i.test(kind) && !isSidecarArtworkImage(item)),
  );
}

export function cleanMediaRowItems<T extends MediaRowItem>(items: T[]): T[] {
  return items.filter(
    (item) => isPlayableMediaItem(item) && !isSidecarArtworkImage(item),
  );
}

export function isActualPlayableMedia(item: MediaRowItem): boolean {
  return isPlayableMediaItem(item);
}
