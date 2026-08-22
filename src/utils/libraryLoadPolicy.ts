export const LIBRARY_PAGE_SIZE = 240;

export interface LibraryPageRequest {
  [key: string]: unknown;
  mediaType?: string;
  limit: number;
  offset: number;
}

interface BuildLibraryPageRequestOptions {
  mediaType?: string;
  pageSize?: number;
  offset?: number;
}

interface MediaIdentity {
  id?: number | null;
  file_path?: string | null;
}

export function buildLibraryPageRequest({
  mediaType,
  pageSize = LIBRARY_PAGE_SIZE,
  offset = 0,
}: BuildLibraryPageRequestOptions): LibraryPageRequest {
  const request: LibraryPageRequest = {
    limit: pageSize,
    offset: Math.max(0, offset),
  };

  if (mediaType && mediaType !== "all") {
    request.mediaType = mediaType;
  }

  return request;
}

export function hasMoreLibraryPages<T>(
  page: T[],
  pageSize = LIBRARY_PAGE_SIZE,
): boolean {
  return page.length >= pageSize;
}

export function shouldAutoLoadNextLibraryPage<T>(
  page: T[],
  pageSize = LIBRARY_PAGE_SIZE,
): boolean {
  return hasMoreLibraryPages(page, pageSize);
}

export function mergeLibraryPage<T extends MediaIdentity>(
  current: T[],
  next: T[],
): T[] {
  const seen = new Set<string>();
  const merged: T[] = [];

  for (const item of [...current, ...next]) {
    const key =
      item.id != null ? `id:${item.id}` : `path:${item.file_path ?? ""}`;
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(item);
  }

  return merged;
}
