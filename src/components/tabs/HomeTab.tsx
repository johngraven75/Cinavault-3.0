// CinaVault Premium — v2 Build 1.04 authoritative real-library HUD
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { JSX } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { useAppStore, type MediaItem } from "../../store/appStore";
import KodiHomeLayout from "../kodi/KodiHomeLayout";
import {
  filterItemsByTitleInitial,
  TITLE_LETTERS,
  type TitleInitialFilter,
} from "../../utils/libraryAlphabetFilter";
import {
  buildLibraryPageRequest,
  hasMoreLibraryPages,
  LIBRARY_PAGE_SIZE,
  mergeLibraryPage,
  shouldAutoLoadNextLibraryPage,
} from "../../utils/libraryLoadPolicy";
import {
  canPlayMediaItem,
  isLibraryDisplayableMediaItem,
} from "../../utils/mediaPlaybackSafety";
import MeteorShower from "../effects/MeteorShower";
import {
  Activity,
  ChevronDown,
  CheckCircle,
  Clock,
  Database,
  Film,
  Grid3X3,
  Heart,
  List,
  Play,
  RefreshCw,
  Search,
  Sparkles,
  Star,
  X,
  Zap,
  type LucideIcon,
} from "lucide-react";

type Shelf = "recent" | "verified" | "unverified" | "favorites";

interface ShelfOption {
  id: Shelf;
  label: string;
  icon: LucideIcon;
}

interface LibraryCount {
  total: number;
  mediaType?: string | null;
  capped: boolean;
}

interface MetadataCheckResult {
  message?: string;
  updated_item?: Partial<MediaItem> & { id?: number };
}

const SHELF_OPTIONS: ShelfOption[] = [
  { id: "recent", label: "Trending Now", icon: Clock },
  { id: "verified", label: "Verified Signal", icon: CheckCircle },
  { id: "unverified", label: "Needs Metadata", icon: Sparkles },
  { id: "favorites", label: "My Vault", icon: Heart },
];

function resolveMediaImageSrc(path?: string | null): string | undefined {
  if (!path) return undefined;
  if (/^(https?:|data:|asset:)/i.test(path)) return path;
  try {
    return convertFileSrc(path);
  } catch {
    return path;
  }
}

function MediaPosterImage({
  path,
  alt,
  className,
  fallbackClassName,
  fallbackSize = 32,
}: {
  path?: string | null;
  alt: string;
  className?: string;
  fallbackClassName: string;
  fallbackSize?: number;
}): JSX.Element {
  const directSrc = resolveMediaImageSrc(path);
  const [imageSrc, setImageSrc] = useState<string | undefined>(directSrc);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let active = true;
    setFailed(false);
    if (!path || /^(https?:|data:|asset:)/i.test(path)) {
      setImageSrc(directSrc);
      return () => { active = false; };
    }
    setImageSrc(undefined);
    void invoke<string>("get_poster_data_url", { path })
      .then((value) => { if (active) setImageSrc(value); })
      .catch(() => { if (active) setFailed(true); });
    return () => { active = false; };
  }, [path, directSrc]);

  if (!imageSrc || failed) {
    return (
      <div className={fallbackClassName} data-poster-fallback="true">
        <Film size={fallbackSize} className="text-cv-subtext/30" />
      </div>
    );
  }

  return (
    <img
      src={imageSrc}
      alt={alt}
      className={className}
      loading="lazy"
      decoding="async"
      onError={() => setFailed(true)}
      data-poster-source={path?.startsWith("http") ? "remote" : "cached-local"}
    />
  );
}

function formatRuntime(minutes?: number): string {
  if (!minutes) return "N/A";
  const hours = Math.floor(minutes / 60);
  const mins = Math.round(minutes % 60);
  return hours > 0 ? `${hours}h ${mins}m` : `${mins}m`;
}

function calculateWatchtimeHours(items: MediaItem[]): number {
  const minutes = items.reduce((total, item) => total + (item.duration || 0), 0);
  return Math.max(0, Math.round(minutes / 60));
}

function sortRecent(items: MediaItem[]): MediaItem[] {
  return [...items].sort((a, b) => {
    const aTime = Date.parse(a.date_added || "") || 0;
    const bTime = Date.parse(b.date_added || "") || 0;
    return bTime - aTime;
  });
}

export default function HomeTab(): JSX.Element {
  const {
    mediaItems,
    setMediaItems,
    selectedMedia,
    setSelectedMedia,
    libraryView,
    setLibraryView,
    searchQuery,
    addStatusMessage,
    currentTheme,
  } = useAppStore();

  const [activeShelf, setActiveShelf] = useState<Shelf>("recent");
  const [typeFilter, setTypeFilter] = useState("all");
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [autoLoadingLibrary, setAutoLoadingLibrary] = useState(false);
  const [libraryOffset, setLibraryOffset] = useState(0);
  const [libraryHasMore, setLibraryHasMore] = useState(false);
  const [authoritativeCount, setAuthoritativeCount] = useState<number | null>(null);
  const [titleInitialFilter, setTitleInitialFilter] =
    useState<TitleInitialFilter>("all");
  const [metadataCheckId, setMetadataCheckId] = useState<number | null>(null);
  const [libraryLoadError, setLibraryLoadError] = useState<string | null>(null);
  const libraryLoadGenerationRef = useRef(0);

  const requestMediaPage = useCallback(
    (offset: number) =>
      invoke<MediaItem[]>(
        "get_media_items",
        buildLibraryPageRequest({ mediaType: typeFilter, offset }),
      ),
    [typeFilter],
  );

  const requestAuthoritativeCount = useCallback(async (): Promise<number> => {
    const result = await invoke<LibraryCount>("get_library_count", {
      mediaType: typeFilter === "all" ? null : typeFilter,
    });
    if (result.capped) {
      throw new Error("Library count contract unexpectedly reported a capped total");
    }
    const total = Number(result.total);
    if (!Number.isSafeInteger(total) || total < 0) {
      throw new Error("Library count contract returned an invalid total");
    }
    return total;
  }, [typeFilter]);

  const refreshAuthoritativeCount = useCallback(async () => {
    try {
      setAuthoritativeCount(await requestAuthoritativeCount());
    } catch (error) {
      setAuthoritativeCount(null);
      addStatusMessage(`Authoritative library count unavailable: ${String(error)}`);
    }
  }, [addStatusMessage, requestAuthoritativeCount]);

  const applyUpdatedMediaItem = useCallback(
    (updated: Partial<MediaItem> & { id?: number }) => {
      if (!updated.id) return;
      setMediaItems((current) =>
        current.map((item) =>
          item.id === updated.id ? { ...item, ...updated } : item,
        ),
      );
      if (selectedMedia?.id === updated.id) {
        setSelectedMedia({ ...selectedMedia, ...updated });
      }
    },
    [selectedMedia, setMediaItems, setSelectedMedia],
  );

  const loadMedia = useCallback(async () => {
    const generation = libraryLoadGenerationRef.current + 1;
    libraryLoadGenerationRef.current = generation;
    setLoading(true);
    setAutoLoadingLibrary(false);
    setLibraryLoadError(null);

    try {
      const [items, exactCount] = await Promise.all([
        requestMediaPage(0),
        requestAuthoritativeCount(),
      ]);
      if (generation !== libraryLoadGenerationRef.current) return;

      const hasMore = hasMoreLibraryPages(items);
      setMediaItems(items);
      setLibraryOffset(items.length);
      setLibraryHasMore(hasMore);
      setAuthoritativeCount(exactCount);
      setAutoLoadingLibrary(shouldAutoLoadNextLibraryPage(items));
      addStatusMessage(
        hasMore
          ? `HUD opened ${items.length} records; authoritative inventory is ${exactCount.toLocaleString()} and the full library is compiling`
          : `HUD loaded all ${exactCount.toLocaleString()} vault records`,
      );
    } catch (error) {
      if (generation !== libraryLoadGenerationRef.current) return;
      setMediaItems([]);
      setSelectedMedia(null);
      setLibraryOffset(0);
      setLibraryHasMore(false);
      setAutoLoadingLibrary(false);
      setAuthoritativeCount(null);
      const message = error instanceof Error ? error.message : String(error);
      setLibraryLoadError(message);
      addStatusMessage(`Library bridge unavailable: ${message}`);
    } finally {
      if (generation === libraryLoadGenerationRef.current) setLoading(false);
    }
  }, [
    addStatusMessage,
    requestAuthoritativeCount,
    requestMediaPage,
    setMediaItems,
    setSelectedMedia,
  ]);

  const loadMoreMedia = useCallback(
    async (automatic = false) => {
      if (loading || loadingMore || !libraryHasMore) return;
      const generation = libraryLoadGenerationRef.current;
      setLoadingMore(true);
      try {
        const items = await requestMediaPage(libraryOffset);
        if (generation !== libraryLoadGenerationRef.current) return;
        const mergedItems = mergeLibraryPage(mediaItems, items);
        const hasMore = hasMoreLibraryPages(items);
        setMediaItems(mergedItems);
        setLibraryOffset(libraryOffset + items.length);
        setLibraryHasMore(hasMore);
        setAutoLoadingLibrary(
          automatic && shouldAutoLoadNextLibraryPage(items),
        );
        if (!hasMore) {
          await refreshAuthoritativeCount();
          addStatusMessage(
            `HUD library compile complete: ${mergedItems.length.toLocaleString()} records loaded`,
          );
        } else if (!automatic) {
          addStatusMessage(
            `Loaded ${items.length} more records (${mergedItems.length.toLocaleString()} currently in view memory)`,
          );
        }
      } catch (error) {
        setAutoLoadingLibrary(false);
        addStatusMessage(`HUD compile failed: ${String(error)}`);
      } finally {
        setLoadingMore(false);
      }
    },
    [
      addStatusMessage,
      libraryHasMore,
      libraryOffset,
      loading,
      loadingMore,
      mediaItems,
      refreshAuthoritativeCount,
      requestMediaPage,
      setMediaItems,
    ],
  );

  useEffect(() => {
    void loadMedia();
  }, [loadMedia]);

  useEffect(() => {
    if (!autoLoadingLibrary || loading || loadingMore || !libraryHasMore) return;
    const timer = window.setTimeout(() => void loadMoreMedia(true), 0);
    return () => window.clearTimeout(timer);
  }, [autoLoadingLibrary, libraryHasMore, loading, loadingMore, loadMoreMedia]);

  useEffect(() => {
    const refresh = () => {
      void loadMedia();
    };
    window.addEventListener("cinavault:library-refresh", refresh);
    return () => window.removeEventListener("cinavault:library-refresh", refresh);
  }, [loadMedia]);

  const displayableMediaItems = useMemo(
    () => mediaItems.filter(isLibraryDisplayableMediaItem),
    [mediaItems],
  );

  const filteredItems = useMemo(() => {
    let items = displayableMediaItems;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      items = items.filter(
        (item) =>
          item.title.toLowerCase().includes(q) ||
          item.genre?.toLowerCase().includes(q) ||
          item.resolution?.toLowerCase().includes(q) ||
          item.codec?.toLowerCase().includes(q),
      );
    }
    if (typeFilter !== "all") {
      items = items.filter((item) => item.media_type === typeFilter);
    }
    if (activeShelf === "verified") items = items.filter((item) => item.verified);
    else if (activeShelf === "unverified") items = items.filter((item) => !item.verified);
    else if (activeShelf === "favorites") items = items.filter((item) => item.favorite);
    else items = sortRecent(items);
    return filterItemsByTitleInitial(items, titleInitialFilter);
  }, [activeShelf, displayableMediaItems, searchQuery, titleInitialFilter, typeFilter]);

  const heroItem = selectedMedia || filteredItems[0] || null;
  const heroImageSrc = heroItem
    ? resolveMediaImageSrc(heroItem.backdrop_path || heroItem.poster_path)
    : undefined;
  const watchtimeHours = calculateWatchtimeHours(displayableMediaItems);
  const verifiedCount = filteredItems.filter((item) => item.verified).length;
  const movieCount = filteredItems.filter((item) => item.media_type === "movie").length;
  const inventoryLabel = authoritativeCount === null ? "Unavailable" : authoritativeCount.toLocaleString();

  const handlePlay = async (item: MediaItem) => {
    if (!canPlayMediaItem(item)) {
      addStatusMessage(`Quick Play skipped: ${item.title} is not playable`);
      return;
    }
    try {
      await invoke("play_media", { filePath: item.file_path });
      addStatusMessage(`Quick Play engaged: ${item.title}`);
    } catch (error) {
      addStatusMessage(`Quick Play failed: ${String(error)}`);
    }
  };

  const handleVerify = async (item: MediaItem) => {
    if (!item.id) return;
    try {
      await invoke("verify_media_item", { id: item.id });
      addStatusMessage(`Verification pulse complete: ${item.title}`);
      await loadMedia();
    } catch (error) {
      addStatusMessage(`Verification pulse failed: ${String(error)}`);
    }
  };

  const handleCheckMetadata = async (item: MediaItem) => {
    if (!item.id) {
      addStatusMessage(`Metadata scan skipped: ${item.title} has no vault id yet`);
      return;
    }
    setMetadataCheckId(item.id);
    try {
      const result = await invoke<MetadataCheckResult>("check_media_item_metadata", {
        id: item.id,
      });
      if (result.updated_item) applyUpdatedMediaItem(result.updated_item);
      addStatusMessage(result.message || `Metadata scan complete: ${item.title}`);
    } catch (error) {
      addStatusMessage(`Metadata scan failed for ${item.title}: ${String(error)}`);
    } finally {
      setMetadataCheckId(null);
    }
  };

  if (currentTheme?.startsWith("kodi_")) return <KodiHomeLayout />;

  return (
    <div className="cyber-home space-y-5">
      <section className="cyber-hero">
        <MeteorShower meteorCount={34} />
        {heroImageSrc && (
          <div
            className="absolute inset-0 z-0 opacity-30"
            style={{
              backgroundImage: `url(${heroImageSrc})`,
              backgroundSize: "cover",
              backgroundPosition: "center",
            }}
          />
        )}
        <div className="absolute inset-0 z-[1] bg-[linear-gradient(90deg,rgba(5,5,10,0.95),rgba(5,5,10,0.52)_45%,rgba(5,5,10,0.82)),radial-gradient(circle_at_80%_22%,rgba(189,0,255,0.24),transparent_36%)]" />
        <div className="relative z-10 grid min-h-[310px] gap-5 p-5 lg:grid-cols-[minmax(0,1fr)_320px]">
          <div className="flex min-w-0 flex-col justify-end">
            <div className="cyber-eyebrow mb-2 flex items-center gap-2">
              <Zap size={14} /> {heroItem ? "Trending Now / Holographic Carousel" : libraryLoadError ? "Library Bridge Offline" : "Vault Empty / Awaiting Scan"}
            </div>
            {heroItem ? (
              <>
                <h2 className="cyber-title max-w-4xl text-4xl font-black tracking-tight lg:text-6xl">{heroItem.title}</h2>
                <div className="mt-3 flex flex-wrap items-center gap-2">
                  {heroItem.year && <span className="cyber-chip">{heroItem.year}</span>}
                  <span className="cyber-chip">{heroItem.media_type || "media"}</span>
                  {heroItem.resolution && <span className="cyber-chip">{heroItem.resolution}</span>}
                  {heroItem.rating && <span className="cyber-chip is-hot"><Star size={12} /> {heroItem.rating}</span>}
                </div>
                {heroItem.overview && <p className="mt-4 max-w-3xl text-sm leading-7 text-cv-subtext">{heroItem.overview}</p>}
              </>
            ) : (
              <>
                <h2 className="cyber-title max-w-4xl text-4xl font-black tracking-tight lg:text-6xl">{libraryLoadError ? "Library Unavailable" : "No Media Found"}</h2>
                <p className="mt-4 max-w-3xl text-sm leading-7 text-cv-subtext">{libraryLoadError ? "The backend media bridge did not respond. No demo media is injected." : "Add media sources and scan to populate the holographic vault."}</p>
              </>
            )}
            <div className="mt-5 flex flex-wrap gap-2">
              {heroItem ? (
                <>
                  <button type="button" onClick={() => void handlePlay(heroItem)} className="cyber-button"><Play size={15} /> Quick Play</button>
                  <button type="button" onClick={() => setSelectedMedia(heroItem)} className="cyber-button"><Sparkles size={15} /> Open Terminal Panel</button>
                  <button type="button" onClick={() => void handleCheckMetadata(heroItem)} className="cyber-button is-amber"><Search size={15} /> Check Metadata</button>
                </>
              ) : (
                <button type="button" onClick={() => void loadMedia()} className="cyber-button"><RefreshCw size={15} className={loading ? "animate-spin" : ""} /> Refresh Library</button>
              )}
            </div>
          </div>
          <div className="cyber-terminal-panel hidden bg-black/35 p-4 lg:block">
            <div className="cyber-eyebrow mb-3 flex items-center gap-2"><Activity size={13} /> User Terminal Quick-Stats</div>
            <TerminalLine label="Watchtime" value={`${watchtimeHours}h`} />
            <TerminalLine label="Vault Inventory" value={inventoryLabel} />
            <TerminalLine label="Loaded Records" value={displayableMediaItems.length.toLocaleString()} />
            <TerminalLine label="Visible Records" value={filteredItems.length.toLocaleString()} />
            <TerminalLine label="Verified Signal" value={`${verifiedCount} locked`} />
            <TerminalLine label="Count Policy" value={authoritativeCount === null ? "Unavailable" : "Uncapped DB total"} />
          </div>
        </div>
      </section>

      <section className="grid gap-3 md:grid-cols-3">
        <StatCard icon={Clock} label="Watchtime" value={`${watchtimeHours}h`} detail="Loaded library runtime index" />
        <StatCard icon={Database} label="Vault Inventory" value={inventoryLabel} detail={authoritativeCount === null ? "Authoritative database count unavailable" : "Exact uncapped indexed-media count"} />
        <StatCard icon={Activity} label="System Status" value={libraryLoadError ? "Offline" : autoLoadingLibrary ? "Compiling" : "Nominal"} detail={`${verifiedCount} visible verified / ${movieCount} visible movies`} />
      </section>

      <section className="cyber-control-core">
        <div className="relative z-10 flex flex-wrap items-center justify-between gap-3">
          <div className="flex flex-wrap gap-2">
            {SHELF_OPTIONS.map((shelf) => {
              const Icon = shelf.icon;
              return (
                <button key={shelf.id} type="button" onClick={() => setActiveShelf(shelf.id)} className={`cyber-button text-xs ${activeShelf === shelf.id ? "is-amber" : ""}`}>
                  <Icon size={13} /> {shelf.label}
                </button>
              );
            })}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <select value={typeFilter} onChange={(event) => setTypeFilter(event.target.value)} className="cyber-select">
              <option value="all">All Types</option>
              <option value="movie">Movies</option>
              <option value="episode">Episodes</option>
              <option value="video">Videos</option>
              <option value="tvshow">TV Shows</option>
              <option value="music">Music</option>
            </select>
            <div className="flex overflow-hidden border border-cyan-300/20 bg-black/40">
              <button type="button" onClick={() => setLibraryView("card")} className={`cyber-button h-10 w-10 px-0 ${libraryView === "card" ? "is-amber" : ""}`} title="Card view"><Grid3X3 size={14} /></button>
              <button type="button" onClick={() => setLibraryView("list")} className={`cyber-button h-10 w-10 px-0 ${libraryView === "list" ? "is-amber" : ""}`} title="List view"><List size={14} /></button>
            </div>
            <button type="button" onClick={() => void loadMedia()} className="cyber-button text-xs"><RefreshCw size={13} className={loading ? "animate-spin" : ""} /> Refresh</button>
          </div>
        </div>
        <div className="relative z-10 mt-4 alphabet-filter" role="tablist" aria-label="Filter library by title initial" tabIndex={0}>
          {(["all", ...TITLE_LETTERS, "#"] as TitleInitialFilter[]).map((letter) => (
            <button key={letter} type="button" role="tab" aria-selected={titleInitialFilter === letter} onClick={() => setTitleInitialFilter(letter)} className={`alphabet-filter-button ${titleInitialFilter === letter ? "active" : ""}`} title={letter === "all" ? "Show all titles" : `Show ${letter} titles`}>
              {letter === "all" ? "All" : letter}
            </button>
          ))}
        </div>
      </section>

      <section className={`grid gap-4 ${selectedMedia ? "xl:grid-cols-[minmax(0,1fr)_360px]" : ""}`}>
        <div>
          {loading ? (
            <div className="cyber-grid">{Array.from({ length: 12 }).map((_, index) => <div key={index} className="cyber-card shimmer h-56" />)}</div>
          ) : filteredItems.length === 0 ? (
            <div className="cyber-panel rounded-[18px] p-12 text-center">
              <Film size={48} className="mx-auto mb-4 text-cv-subtext/40" />
              <h3 className="text-lg font-black uppercase tracking-[0.12em] text-cv-text">{libraryLoadError ? "Library Bridge Offline" : "No Media Found"}</h3>
              <p className="mt-2 text-sm text-cv-subtext">{libraryLoadError ? "No demo records are loaded when the backend is unavailable." : "Add media sources and scan to populate the holographic vault."}</p>
            </div>
          ) : libraryView === "card" ? (
            <div className="cyber-grid">
              {filteredItems.map((item, index) => (
                <MediaCard key={`${item.id || item.title}-${index}`} item={item} checking={metadataCheckId === item.id} onSelect={() => setSelectedMedia(item)} onPlay={() => void handlePlay(item)} onCheckMetadata={() => void handleCheckMetadata(item)} />
              ))}
            </div>
          ) : (
            <div className="cyber-table">
              <div className="cyber-table-row with-metadata-action cyber-stat-label bg-cyan-300/[0.06]"><span>Title</span><span>Type</span><span>Year</span><span>Rating</span><span>Status</span><span>Metadata</span></div>
              {filteredItems.map((item, index) => {
                const checking = metadataCheckId === item.id;
                return (
                  <div key={`${item.id || item.title}-row-${index}`} role="button" tabIndex={0} onClick={() => setSelectedMedia(item)} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); setSelectedMedia(item); } }} className="cyber-table-row with-metadata-action w-full text-left text-sm">
                    <span className="truncate font-semibold">{item.title}</span>
                    <span className="text-xs capitalize text-cv-subtext">{item.media_type}</span>
                    <span className="text-xs text-cv-subtext">{item.year || "—"}</span>
                    <span className="flex items-center gap-1 text-xs">{item.rating ? <><Star size={11} className="text-[var(--cyber-amber)]" />{item.rating}</> : "—"}</span>
                    <span>{item.verified ? <CheckCircle size={15} className="text-cyan-200" /> : <Clock size={15} className="text-cv-subtext/50" />}</span>
                    <button type="button" onClick={(event) => { event.stopPropagation(); void handleCheckMetadata(item); }} disabled={checking} className="cyber-button library-row-metadata-action disabled:opacity-60" title={`Check metadata for ${item.title}`}>
                      {checking ? <RefreshCw size={12} className="animate-spin" /> : <Search size={12} />} {checking ? "Checking..." : "Check Metadata"}
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {selectedMedia && (
          <motion.aside key={selectedMedia.id || selectedMedia.title} initial={{ opacity: 0, x: 28 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: 28 }} transition={{ duration: 0.24 }} className="cyber-terminal-panel bg-[#05050a]/90 p-4">
            <div className="mb-4 flex items-start justify-between gap-3">
              <div><div className="cyber-eyebrow flex items-center gap-2"><Sparkles size={13} /> Terminal Panel</div><h3 className="mt-1 text-xl font-black leading-tight text-cv-text">{selectedMedia.title}</h3></div>
              <button type="button" onClick={() => setSelectedMedia(null)} className="cyber-button h-10 w-10 px-0" title="Close terminal panel"><X size={15} /></button>
            </div>
            <MediaPosterImage path={selectedMedia.poster_path || selectedMedia.backdrop_path} alt={selectedMedia.title} className="mb-4 max-h-72 w-full rounded border border-cyan-300/25 object-cover opacity-90" fallbackClassName="mb-4 flex h-56 w-full items-center justify-center rounded border border-cyan-300/25 bg-black/40" fallbackSize={46} />
            <div className="space-y-1">
              <TerminalLine label="Type" value={selectedMedia.media_type || "Unknown"} />
              <TerminalLine label="Year" value={selectedMedia.year ? `${selectedMedia.year}` : "N/A"} />
              <TerminalLine label="Genre" value={selectedMedia.genre || "Unclassified"} />
              <TerminalLine label="Runtime" value={formatRuntime(selectedMedia.duration)} />
              <TerminalLine label="Verified" value={selectedMedia.verified ? "Locked" : "Pending"} />
              <TerminalLine label="Favorite" value={selectedMedia.favorite ? "Vaulted" : "Not Set"} />
            </div>
            {selectedMedia.overview && <p className="mt-4 rounded border border-cyan-300/10 bg-black/30 p-3 text-xs leading-6 text-cv-subtext">{selectedMedia.overview}</p>}
            <div className="mt-4 grid gap-2">
              <button type="button" onClick={() => void handlePlay(selectedMedia)} className="cyber-button"><Play size={14} /> Quick Play</button>
              <button type="button" onClick={() => void handleVerify(selectedMedia)} className="cyber-button"><CheckCircle size={14} /> Verify Signal</button>
              <button type="button" onClick={() => void handleCheckMetadata(selectedMedia)} disabled={metadataCheckId === selectedMedia.id} className="cyber-button is-amber disabled:opacity-60">{metadataCheckId === selectedMedia.id ? <RefreshCw size={14} className="animate-spin" /> : <Sparkles size={14} />} Check Metadata</button>
            </div>
          </motion.aside>
        )}
      </section>

      {autoLoadingLibrary && !loading && (
        <div className="flex justify-center"><div className="cyber-button pointer-events-none"><RefreshCw size={14} className="animate-spin" /> Compiling full library ({displayableMediaItems.length.toLocaleString()} loaded / {inventoryLabel} total)</div></div>
      )}
      {libraryHasMore && !loading && !autoLoadingLibrary && (
        <div className="flex justify-center"><button type="button" onClick={() => void loadMoreMedia()} disabled={loadingMore} className="cyber-button">{loadingMore ? <RefreshCw size={14} className="animate-spin" /> : <ChevronDown size={14} />} {loadingMore ? "Compiling" : `Load Next ${LIBRARY_PAGE_SIZE}`}</button></div>
      )}
    </div>
  );
}

function StatCard({ icon: Icon, label, value, detail }: { icon: LucideIcon; label: string; value: string; detail: string }): JSX.Element {
  return (
    <div className="cyber-stat"><div className="relative z-10 flex items-start justify-between gap-3"><div><div className="cyber-stat-label">{label}</div><div className="cyber-stat-value mt-2">{value}</div><div className="mt-2 text-xs text-cv-subtext">{detail}</div></div><div className="grid h-11 w-11 place-items-center border border-cyan-300/25 bg-cyan-300/10 text-cyan-200 shadow-[0_0_18px_rgba(0,245,255,0.16)]"><Icon size={18} /></div></div></div>
  );
}

function TerminalLine({ label, value }: { label: string; value: string }): JSX.Element {
  return <div className="terminal-line"><span>{label}</span><strong>{value}</strong></div>;
}

function MediaCard({ item, checking, onSelect, onPlay, onCheckMetadata }: { item: MediaItem; checking: boolean; onSelect: () => void; onPlay: () => void; onCheckMetadata: () => void }): JSX.Element {
  return (
    <motion.div className="cyber-card group" initial={{ opacity: 0, y: 14 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2 }} onClick={onSelect}>
      <CardVisual item={item} />
      <div className="relative z-10 p-3"><h4 className="truncate text-sm font-black text-cv-text">{item.title}</h4><div className="mt-1 flex flex-wrap items-center gap-2 text-[10px] uppercase tracking-[0.08em] text-cv-subtext">{item.year && <span>{item.year}</span>}<span>{item.media_type}</span>{item.resolution && <span className="text-cyan-200">{item.resolution}</span>}</div>{item.genre && <div className="mt-2 truncate text-[11px] text-cv-subtext/80">{item.genre}</div>}</div>
      <div className="cyber-card-actions">
        <button type="button" onClick={(event) => { event.stopPropagation(); onPlay(); }} className="cyber-button flex-1 text-[10px]"><span className="cyber-bracket">[▶]</span> Play</button>
        <button type="button" onClick={(event) => { event.stopPropagation(); onCheckMetadata(); }} disabled={checking} className="cyber-button flex-1 text-[10px] disabled:opacity-60">{checking ? <RefreshCw size={12} className="animate-spin" /> : <Search size={12} />}<span className="metadata-action-label">{checking ? "Checking..." : "Check Metadata"}</span></button>
      </div>
    </motion.div>
  );
}

function CardVisual({ item }: { item: MediaItem }): JSX.Element {
  return (
    <div className="cyber-poster aspect-[2/3]">
      <MediaPosterImage path={item.poster_path || item.backdrop_path} alt={item.title} fallbackClassName="flex h-full w-full items-center justify-center" />
      <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent" />
      <div className="absolute right-2 top-2 flex flex-col gap-1">
        {item.verified && <span className="grid h-6 w-6 place-items-center border border-cyan-200/40 bg-cyan-300/20 text-cyan-100"><CheckCircle size={12} /></span>}
        {item.favorite && <span className="grid h-6 w-6 place-items-center border border-[var(--cyber-amber)]/50 bg-[var(--cyber-amber)]/20 text-[var(--cyber-amber)]"><Heart size={12} /></span>}
      </div>
      {item.resolution && <span className="cyber-chip absolute bottom-2 left-2 py-1 text-[9px]">{item.resolution}</span>}
    </div>
  );
}
