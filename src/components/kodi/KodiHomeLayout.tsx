// CinaVault Premium — Kodi Home Layout (Build 155)
// Full Kodi-style home screen: hero banner, horizontal shelves, poster wall, detail panel
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type JSX,
} from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import {
  CheckCircle,
  ChevronRight,
  Film,
  Grid3X3,
  Heart,
  Info,
  List,
  Play,
  RefreshCw,
  Search,
  Sparkles,
  Star,
  Clock,
  type LucideIcon,
} from "lucide-react";
import { useAppStore, type MediaItem } from "../../store/appStore";
import "../../styles/kodi-skin.css";

// ─── helpers ──────────────────────────────────────────────────────────────

function resolveImg(path?: string | null): string | undefined {
  if (!path) return undefined;
  if (/^(https?:|data:|asset:)/i.test(path)) return path;
  try {
    return convertFileSrc(path);
  } catch {
    return path;
  }
}

function KodiPosterImage({
  path,
  alt,
  imageClassName,
  fallbackClassName,
  fallbackSize,
}: {
  path?: string | null;
  alt: string;
  imageClassName: string;
  fallbackClassName: string;
  fallbackSize?: number;
}): JSX.Element {
  const directSrc = resolveImg(path);
  const [src, setSrc] = useState<string | undefined>(directSrc);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    setFailed(false);
    if (!path || /^(https?:|data:|asset:)/i.test(path)) {
      setSrc(directSrc);
      return () => { active = false; };
    }
    setSrc(undefined);
    void invoke<string>("get_poster_data_url", { path })
      .then((value) => { if (active) setSrc(value); })
      .catch(() => { if (active) setFailed(true); });
    return () => { active = false; };
  }, [path, directSrc]);

  if (!src || failed) {
    return (
      <div className={fallbackClassName} data-poster-fallback="true">
        {fallbackSize ? <Film size={fallbackSize} /> : null}
      </div>
    );
  }
  return (
    <img
      src={src}
      alt={alt}
      className={imageClassName}
      loading="lazy"
      decoding="async"
      onError={() => setFailed(true)}
      data-poster-source={path?.startsWith("http") ? "remote" : "sidecar"}
    />
  );
}

function sortRecent(items: MediaItem[]): MediaItem[] {
  return [...items].sort((a, b) => {
    const at = Date.parse(a.date_added ?? "") || 0;
    const bt = Date.parse(b.date_added ?? "") || 0;
    return bt - at;
  });
}

function sortRating(items: MediaItem[]): MediaItem[] {
  return [...items].sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0));
}

// ─── Hero Banner ──────────────────────────────────────────────────────────

interface HeroProps {
  items: MediaItem[];
  onPlay: (item: MediaItem) => void;
  onSelect: (item: MediaItem) => void;
}

function KodiHero({ items, onPlay, onSelect }: HeroProps): JSX.Element {
  const [heroIdx, setHeroIdx] = useState(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const heroItems = useMemo(() => items.slice(0, 8), [items]);
  const hero = heroItems[heroIdx];

  const advance = useCallback(() => {
    setHeroIdx((i) => (i + 1) % Math.max(heroItems.length, 1));
  }, [heroItems.length]);

  useEffect(() => {
    if (heroItems.length <= 1) return;
    timerRef.current = setTimeout(advance, 7000);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [heroIdx, heroItems.length, advance]);

  if (!hero) {
    return (
      <div className="kodi-hero">
        <div className="kodi-hero-backdrop-fallback" />
      </div>
    );
  }

  return (
    <div className="kodi-hero">
      <KodiPosterImage
        key={hero.id}
        path={hero.backdrop_path ?? hero.poster_path}
        alt={hero.title}
        imageClassName="kodi-hero-backdrop"
        fallbackClassName="kodi-hero-backdrop-fallback"
      />
      <div className="kodi-hero-gradient" />

      <AnimatePresence mode="wait">
        <motion.div
          key={hero.id}
          className="kodi-hero-content"
          initial={{ opacity: 0, y: 14 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -10 }}
          transition={{ duration: 0.4 }}
        >
          <div className="kodi-hero-badge">
            <Star size={10} /> Featured
          </div>
          <h2 className="kodi-hero-title">{hero.title}</h2>
          <div className="kodi-hero-meta">
            {hero.year && <span>{hero.year}</span>}
            {hero.year && hero.genre && <span className="kodi-hero-meta-dot" />}
            {hero.genre && <span>{hero.genre}</span>}
            {hero.rating && (
              <>
                <span className="kodi-hero-meta-dot" />
                <span>★ {hero.rating.toFixed(1)}</span>
              </>
            )}
          </div>
          {hero.overview && (
            <p className="kodi-hero-overview">{hero.overview}</p>
          )}
          <div className="kodi-hero-actions">
            <button
              type="button"
              className="kodi-btn-play"
              onClick={() => onPlay(hero)}
            >
              <Play size={14} fill="currentColor" /> Play
            </button>
            <button
              type="button"
              className="kodi-btn-info"
              onClick={() => onSelect(hero)}
            >
              <Info size={14} /> More Info
            </button>
          </div>
        </motion.div>
      </AnimatePresence>

      {heroItems.length > 1 && (
        <div className="kodi-hero-dots">
          {heroItems.map((_, i) => (
            <button
              key={i}
              type="button"
              className={`kodi-hero-dot${i === heroIdx ? " active" : ""}`}
              onClick={() => setHeroIdx(i)}
              aria-label={`Hero item ${i + 1}`}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Poster Card ──────────────────────────────────────────────────────────

type MetadataCheckResult = {
  message?: string;
  updated_item?: MediaItem;
};

interface CardProps {
  item: MediaItem;
  onSelect: (item: MediaItem) => void;
  onPlay: (item: MediaItem) => void;
}

function KodiCard({ item, onSelect, onPlay }: CardProps): JSX.Element {
  const hasNfo = Boolean(item.nfo_path);
  const hasPoster = Boolean(
    item.poster_path && !item.poster_path.startsWith("http"),
  );

  return (
    <motion.div
      className="kodi-card"
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2 }}
      onClick={() => onSelect(item)}
      tabIndex={0}
      onKeyDown={(e) => e.key === "Enter" && onSelect(item)}
    >
      {/* Poster */}
      <KodiPosterImage
        path={item.poster_path ?? item.backdrop_path}
        alt={item.title}
        imageClassName="kodi-poster"
        fallbackClassName="kodi-poster-placeholder"
        fallbackSize={28}
      />
      <div className="kodi-poster-overlay" />

      {/* Badges */}
      <div className="kodi-card-badges">
        {item.verified && (
          <span className="kodi-badge kodi-badge-verified">
            <CheckCircle size={11} />
          </span>
        )}
        {item.favorite && (
          <span className="kodi-badge kodi-badge-favorite">
            <Heart size={11} />
          </span>
        )}
      </div>
      {item.resolution && (
        <span className="kodi-badge-resolution">{item.resolution}</span>
      )}

      {/* NFO / Poster sidecar badges */}
      {(hasNfo || hasPoster) && (
        <div
          style={{
            position: "absolute",
            bottom: 8,
            right: 8,
            display: "flex",
            gap: 4,
          }}
        >
          {hasNfo && <span className="kodi-nfo-badge">NFO</span>}
          {hasPoster && <span className="kodi-poster-badge">ART</span>}
        </div>
      )}

      {/* Card info */}
      <div className="kodi-card-info">
        <div className="kodi-card-title">{item.title}</div>
        <div className="kodi-card-meta">
          {item.year && <span>{item.year}</span>}
          {item.rating && (
            <span style={{ color: "var(--cv-gold)" }}>
              ★ {item.rating.toFixed(1)}
            </span>
          )}
        </div>
        {item.genre && <div className="kodi-card-genre">{item.genre}</div>}
      </div>

      {/* Hover quick actions */}
      <div className="kodi-card-hover-actions">
        <button
          type="button"
          className="kodi-quick-play"
          onClick={(e) => {
            e.stopPropagation();
            onPlay(item);
          }}
        >
          <Play size={12} fill="currentColor" /> Play
        </button>
        <button
          type="button"
          className="kodi-quick-meta"
          onClick={(e) => {
            e.stopPropagation();
            onSelect(item);
          }}
        >
          <Info size={12} /> Details
        </button>
      </div>
    </motion.div>
  );
}

// ─── Shelf ────────────────────────────────────────────────────────────────

interface ShelfProps {
  title: string;
  icon: LucideIcon;
  items: MediaItem[];
  onSelect: (item: MediaItem) => void;
  onPlay: (item: MediaItem) => void;
  onSeeAll?: () => void;
}

function KodiShelf({
  title,
  icon: Icon,
  items,
  onSelect,
  onPlay,
  onSeeAll,
}: ShelfProps): JSX.Element | null {
  if (items.length === 0) return null;
  return (
    <section className="kodi-section">
      <div className="kodi-section-header">
        <div className="kodi-section-title">
          <span className="kodi-section-title-accent" />
          <Icon size={14} />
          {title}
          <span className="kodi-section-count">{items.length}</span>
        </div>
        {onSeeAll && (
          <button type="button" className="kodi-see-all" onClick={onSeeAll}>
            See All <ChevronRight size={12} style={{ display: "inline" }} />
          </button>
        )}
      </div>
      <div className="kodi-shelf">
        {items.map((item) => (
          <KodiCard
            key={item.id}
            item={item}
            onSelect={onSelect}
            onPlay={onPlay}
          />
        ))}
      </div>
    </section>
  );
}

// ─── Detail Panel ─────────────────────────────────────────────────────────

interface DetailPanelProps {
  item: MediaItem;
  onPlay: (item: MediaItem) => void;
  onClose: () => void;
  onCheckMetadata: (item: MediaItem) => void;
  checkingId: number | null;
}

function KodiDetailPanel({
  item,
  onPlay,
  onClose,
  onCheckMetadata,
  checkingId,
}: DetailPanelProps): JSX.Element {
  const imgSrc = resolveImg(item.poster_path);
  const hasNfo = Boolean(item.nfo_path);
  const hasPoster = Boolean(
    item.poster_path && !item.poster_path.startsWith("http"),
  );

  return (
    <motion.aside
      className="kodi-detail-panel"
      initial={{ opacity: 0, x: 32 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: 32 }}
      transition={{ duration: 0.25 }}
    >
      <KodiPosterImage
        path={item.poster_path ?? item.backdrop_path}
        alt={item.title}
        imageClassName="kodi-detail-poster"
        fallbackClassName="kodi-detail-poster kodi-poster-placeholder"
        fallbackSize={40}
      />

      <div className="kodi-detail-title">{item.title}</div>

      <div className="kodi-detail-meta">
        {item.year && <span className="kodi-detail-chip">{item.year}</span>}
        {item.media_type && (
          <span className="kodi-detail-chip accent">{item.media_type}</span>
        )}
        {item.resolution && (
          <span className="kodi-detail-chip">{item.resolution}</span>
        )}
        {item.rating && (
          <span
            className="kodi-detail-chip"
            style={{ color: "var(--cv-gold)" }}
          >
            ★ {item.rating.toFixed(1)}
          </span>
        )}
        {hasNfo && <span className="kodi-nfo-badge">NFO</span>}
        {hasPoster && <span className="kodi-poster-badge">ART</span>}
      </div>

      {item.overview && <p className="kodi-detail-overview">{item.overview}</p>}

      <div>
        {item.genre && (
          <div className="kodi-detail-row">
            <span className="kodi-detail-row-label">Genre</span>
            <span className="kodi-detail-row-value">{item.genre}</span>
          </div>
        )}
        {item.tmdb_id && (
          <div className="kodi-detail-row">
            <span className="kodi-detail-row-label">TMDB</span>
            <span className="kodi-detail-row-value">{item.tmdb_id}</span>
          </div>
        )}
        {item.imdb_id && (
          <div className="kodi-detail-row">
            <span className="kodi-detail-row-label">IMDb</span>
            <span className="kodi-detail-row-value">{item.imdb_id}</span>
          </div>
        )}
        {item.date_added && (
          <div className="kodi-detail-row">
            <span className="kodi-detail-row-label">Added</span>
            <span className="kodi-detail-row-value">
              {new Date(item.date_added).toLocaleDateString()}
            </span>
          </div>
        )}
        {item.file_path && (
          <div className="kodi-detail-row">
            <span className="kodi-detail-row-label">File</span>
            <span
              className="kodi-detail-row-value"
              style={{ fontSize: 9, wordBreak: "break-all" }}
            >
              {item.file_path.split(/[\\/]/).pop()}
            </span>
          </div>
        )}
      </div>

      <div className="kodi-detail-actions">
        <button
          type="button"
          className="kodi-detail-btn primary"
          onClick={() => onPlay(item)}
        >
          <Play size={14} fill="currentColor" /> Play Now
        </button>
        <button
          type="button"
          className="kodi-detail-btn secondary"
          onClick={() => onCheckMetadata(item)}
          disabled={checkingId === item.id}
        >
          {checkingId === item.id ? (
            <RefreshCw size={13} className="animate-spin" />
          ) : (
            <Sparkles size={13} />
          )}
          {checkingId === item.id ? "Fetching…" : "Refresh Metadata"}
        </button>
        <button
          type="button"
          className="kodi-detail-btn secondary"
          onClick={onClose}
        >
          Close
        </button>
      </div>
    </motion.aside>
  );
}

// ─── Main KodiHomeLayout ──────────────────────────────────────────────────

type ViewMode = "shelves" | "grid";
type FilterType = "all" | "movie" | "episode" | "adult" | "video";

export default function KodiHomeLayout(): JSX.Element {
  const {
    mediaItems,
    setMediaItems,
    selectedMedia,
    setSelectedMedia,
    addStatusMessage,
  } = useAppStore();

  const [loading, setLoading] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>("shelves");
  const [typeFilter, setTypeFilter] = useState<FilterType>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [checkingId, setCheckingId] = useState<number | null>(null);

  // Load library
  useEffect(() => {
    setLoading(true);
    invoke<MediaItem[]>("get_media_items", {
      mediaType: "all",
      limit: 500,
      offset: 0,
    })
      .then((items) => setMediaItems(items))
      .catch((err) => addStatusMessage(`Library load error: ${err}`))
      .finally(() => setLoading(false));
  }, [setMediaItems, addStatusMessage]);

  const handlePlay = useCallback(
    (item: MediaItem) => {
      invoke("play_media", { filePath: item.file_path }).catch((err) =>
        addStatusMessage(`Playback error: ${err}`),
      );
    },
    [addStatusMessage],
  );

  const handleCheckMetadata = useCallback(
    async (item: MediaItem) => {
      setCheckingId(item.id ?? null);
      try {
        const result = await invoke<MetadataCheckResult>(
          "check_media_item_metadata",
          { id: item.id },
        );
        const updated = result.updated_item;
        if (!updated) {
          throw new Error(
            result.message || "Metadata provider returned no updated media item",
          );
        }
        setMediaItems(
          mediaItems.map((media) =>
            media.id === updated.id ? { ...media, ...updated } : media,
          ),
        );
        if (selectedMedia?.id === updated.id) {
          setSelectedMedia({ ...selectedMedia, ...updated });
        }
        addStatusMessage(
          result.message || `Metadata updated: ${updated.title}`,
        );
      } catch (err) {
        addStatusMessage(`Metadata error: ${err}`);
      } finally {
        setCheckingId(null);
      }
    },
    [
      mediaItems,
      selectedMedia,
      setMediaItems,
      setSelectedMedia,
      addStatusMessage,
    ],
  );

  // Filtered items
  const filteredItems = useMemo(() => {
    let items = mediaItems;
    if (typeFilter !== "all") {
      items = items.filter((m) => m.media_type?.toLowerCase() === typeFilter);
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      items = items.filter(
        (m) =>
          m.title?.toLowerCase().includes(q) ||
          m.genre?.toLowerCase().includes(q) ||
          m.overview?.toLowerCase().includes(q),
      );
    }
    return items;
  }, [mediaItems, typeFilter, searchQuery]);

  const recentItems = useMemo(
    () => sortRecent(filteredItems).slice(0, 20),
    [filteredItems],
  );
  const topRatedItems = useMemo(
    () =>
      sortRating(filteredItems.filter((m) => (m.rating ?? 0) >= 7)).slice(
        0,
        20,
      ),
    [filteredItems],
  );
  const needsMetadata = useMemo(
    () =>
      filteredItems
        .filter((m) => !m.verified && !m.overview && !m.poster_path)
        .slice(0, 20),
    [filteredItems],
  );
  const favorites = useMemo(
    () => filteredItems.filter((m) => m.favorite).slice(0, 20),
    [filteredItems],
  );

  const TYPE_FILTERS: { id: FilterType; label: string }[] = [
    { id: "all", label: "All" },
    { id: "movie", label: "Movies" },
    { id: "episode", label: "TV" },
    { id: "adult", label: "Adult" },
    { id: "video", label: "Videos" },
  ];

  return (
    <div
      className="kodi-home"
      style={{ display: "flex", flexDirection: "column", gap: 24 }}
    >
      {/* Hero */}
      {recentItems.length > 0 && (
        <KodiHero
          items={recentItems}
          onPlay={handlePlay}
          onSelect={setSelectedMedia}
        />
      )}

      {/* Controls row */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "0 24px",
          flexWrap: "wrap",
        }}
      >
        {/* Filter chips */}
        <div className="kodi-filter-bar" style={{ flex: 1, padding: 0 }}>
          {TYPE_FILTERS.map((f) => (
            <button
              key={f.id}
              type="button"
              className={`kodi-filter-chip${typeFilter === f.id ? " active" : ""}`}
              onClick={() => setTypeFilter(f.id)}
            >
              {f.label}
            </button>
          ))}
        </div>

        {/* View toggle */}
        <div className="kodi-view-toggle">
          <button
            type="button"
            className={`kodi-view-btn${viewMode === "shelves" ? " active" : ""}`}
            onClick={() => setViewMode("shelves")}
          >
            <List size={13} /> Shelves
          </button>
          <button
            type="button"
            className={`kodi-view-btn${viewMode === "grid" ? " active" : ""}`}
            onClick={() => setViewMode("grid")}
          >
            <Grid3X3 size={13} /> Grid
          </button>
        </div>
      </div>

      {/* Search */}
      <div className="kodi-search-bar">
        <Search
          size={14}
          style={{ color: "var(--cv-subtext)", flexShrink: 0 }}
        />
        <input
          className="kodi-search-input"
          placeholder="Search library…"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
        {searchQuery && (
          <button
            type="button"
            style={{
              background: "none",
              border: "none",
              color: "var(--cv-subtext)",
              cursor: "pointer",
              padding: 0,
            }}
            onClick={() => setSearchQuery("")}
          >
            ✕
          </button>
        )}
      </div>

      {/* Loading state */}
      {loading && (
        <div
          style={{
            display: "flex",
            justifyContent: "center",
            padding: "20px 0",
          }}
        >
          <RefreshCw
            size={20}
            className="animate-spin"
            style={{ color: "var(--cv-accent)" }}
          />
        </div>
      )}

      {/* Main content */}
      <div style={{ display: "flex", gap: 0, flex: 1 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          {viewMode === "shelves" ? (
            <>
              <KodiShelf
                title="Trending Now"
                icon={Clock}
                items={recentItems}
                onSelect={setSelectedMedia}
                onPlay={handlePlay}
              />
              {topRatedItems.length > 0 && (
                <div style={{ marginTop: 28 }}>
                  <KodiShelf
                    title="Top Rated"
                    icon={Star}
                    items={topRatedItems}
                    onSelect={setSelectedMedia}
                    onPlay={handlePlay}
                  />
                </div>
              )}
              {favorites.length > 0 && (
                <div style={{ marginTop: 28 }}>
                  <KodiShelf
                    title="My Vault"
                    icon={Heart}
                    items={favorites}
                    onSelect={setSelectedMedia}
                    onPlay={handlePlay}
                  />
                </div>
              )}
              {needsMetadata.length > 0 && (
                <div style={{ marginTop: 28 }}>
                  <KodiShelf
                    title="Needs Metadata"
                    icon={Sparkles}
                    items={needsMetadata}
                    onSelect={setSelectedMedia}
                    onPlay={handlePlay}
                  />
                </div>
              )}
            </>
          ) : (
            <div className="kodi-poster-wall">
              <AnimatePresence>
                {filteredItems.map((item) => (
                  <KodiCard
                    key={item.id}
                    item={item}
                    onSelect={setSelectedMedia}
                    onPlay={handlePlay}
                  />
                ))}
              </AnimatePresence>
              {filteredItems.length === 0 && !loading && (
                <div
                  style={{
                    gridColumn: "1/-1",
                    textAlign: "center",
                    padding: "40px 0",
                    color: "var(--cv-subtext)",
                    fontSize: 13,
                  }}
                >
                  No items match your filter.
                </div>
              )}
            </div>
          )}
        </div>

        {/* Detail panel */}
        <AnimatePresence>
          {selectedMedia && (
            <KodiDetailPanel
              item={selectedMedia}
              onPlay={handlePlay}
              onClose={() => setSelectedMedia(null)}
              onCheckMetadata={handleCheckMetadata}
              checkingId={checkingId}
            />
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
