// CinaVault Premium — SQLite Database Layer (rusqlite) — Build 115
// Premium defaults: all features ON, full persistence support

#[cfg(test)]
use crate::library_artifacts::sidecar_poster_path_for_video;
use crate::library_artifacts::{is_generated_chapter_image_path, is_sidecar_artwork_image};
use crate::AppState;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaItem {
    pub id: Option<i64>,
    pub title: String,
    pub file_path: String,
    pub media_type: String,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub genre: Option<String>,
    pub duration: Option<i64>,
    pub file_size: Option<i64>,
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub verified: bool,
    pub watched: bool,
    pub favorite: bool,
    pub date_added: String,
    pub last_played: Option<String>,
    pub tmdb_id: Option<String>,
    pub imdb_id: Option<String>,
    pub source_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaSource {
    pub id: Option<i64>,
    pub path: String,
    pub source_type: String,
    pub name: String,
    pub enabled: bool,
    pub last_scanned: Option<String>,
    pub item_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteAccessUserProvision {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub access_key: String,
    pub access_key_preview: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteAccessUserSummary {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub access_key_preview: String,
    pub enabled: bool,
    pub permissions: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_login: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteAccessPrincipal {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub auth_method: String,
    pub session_token: String,
    pub expires_at: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteAccessKeyRotation {
    pub email: String,
    pub access_key: String,
    pub access_key_preview: String,
}

pub struct Database {
    pub conn: Connection,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AdultLibraryLabelResult {
    pub inventory_items: usize,
    pub items_labeled_adult: usize,
    pub items_already_adult: usize,
}

impl Database {
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        let db = Database { conn };
        db.initialize_tables()?;
        Ok(db)
    }

    fn initialize_tables(&self) -> SqlResult<()> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS media_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                file_path TEXT NOT NULL UNIQUE,
                media_type TEXT NOT NULL DEFAULT 'movie',
                year INTEGER,
                rating REAL,
                overview TEXT,
                poster_path TEXT,
                backdrop_path TEXT,
                genre TEXT,
                duration INTEGER,
                file_size INTEGER,
                resolution TEXT,
                codec TEXT,
                verified INTEGER DEFAULT 0,
                watched INTEGER DEFAULT 0,
                favorite INTEGER DEFAULT 0,
                date_added TEXT NOT NULL,
                last_played TEXT,
                tmdb_id TEXT,
                imdb_id TEXT,
                source_id INTEGER,
                FOREIGN KEY (source_id) REFERENCES media_sources(id)
            );

            CREATE TABLE IF NOT EXISTS media_sources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                source_type TEXT NOT NULL DEFAULT 'folder',
                name TEXT NOT NULL,
                enabled INTEGER DEFAULT 1,
                last_scanned TEXT,
                item_count INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS feature_settings (
                feature_key TEXT PRIMARY KEY,
                enabled INTEGER DEFAULT 0,
                config_json TEXT DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS xtream_profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                server_url TEXT NOT NULL,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                enabled INTEGER DEFAULT 1,
                last_synced TEXT
            );

            CREATE TABLE IF NOT EXISTS live_channels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                profile_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                stream_url TEXT NOT NULL,
                logo_url TEXT,
                group_name TEXT,
                epg_id TEXT,
                FOREIGN KEY (profile_id) REFERENCES xtream_profiles(id)
            );

            CREATE TABLE IF NOT EXISTS plugin_repos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                enabled INTEGER DEFAULT 1,
                last_synced TEXT
            );

            CREATE TABLE IF NOT EXISTS plugins (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                version TEXT,
                description TEXT,
                author TEXT,
                repo_id INTEGER,
                installed INTEGER DEFAULT 0,
                config_json TEXT DEFAULT '{}',
                FOREIGN KEY (repo_id) REFERENCES plugin_repos(id)
            );

            CREATE TABLE IF NOT EXISTS api_keys (
                provider TEXT PRIMARY KEY,
                api_key TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS download_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT,
                status TEXT DEFAULT 'pending',
                file_path TEXT,
                file_size INTEGER,
                started_at TEXT,
                completed_at TEXT,
                error TEXT
            );

            CREATE TABLE IF NOT EXISTS duplicate_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_hash TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS duplicate_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL,
                media_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                file_size INTEGER,
                FOREIGN KEY (group_id) REFERENCES duplicate_groups(id),
                FOREIGN KEY (media_id) REFERENCES media_items(id)
            );

            CREATE TABLE IF NOT EXISTS remote_access_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT NOT NULL UNIQUE COLLATE NOCASE,
                display_name TEXT,
                password_salt TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                access_key_salt TEXT NOT NULL,
                access_key_hash TEXT NOT NULL,
                access_key_preview TEXT NOT NULL,
                enabled INTEGER DEFAULT 1,
                permissions TEXT NOT NULL DEFAULT 'server:read,library:read,stream:play',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_login TEXT
            );

            CREATE TABLE IF NOT EXISTS remote_access_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                token_salt TEXT NOT NULL,
                token_hash TEXT NOT NULL UNIQUE,
                token_lookup TEXT NOT NULL UNIQUE,
                auth_method TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                revoked INTEGER DEFAULT 0,
                FOREIGN KEY (user_id) REFERENCES remote_access_users(id)
            );

            CREATE INDEX IF NOT EXISTS idx_media_title ON media_items(title);
            CREATE INDEX IF NOT EXISTS idx_media_type ON media_items(media_type);
            CREATE INDEX IF NOT EXISTS idx_media_source ON media_items(source_id);
            CREATE INDEX IF NOT EXISTS idx_media_verified ON media_items(verified);
            CREATE INDEX IF NOT EXISTS idx_media_date ON media_items(date_added);
            CREATE INDEX IF NOT EXISTS idx_remote_access_users_email ON remote_access_users(email);
            CREATE INDEX IF NOT EXISTS idx_remote_access_sessions_user ON remote_access_sessions(user_id);
        ")?;
        self.ensure_column("remote_access_sessions", "token_lookup", "TEXT")?;
        // Existing sessions cannot be indexed safely because only their salted hashes are stored.
        // Revoking them forces one reauthentication and avoids a linear token scan on every request.
        self.conn.execute(
            "UPDATE remote_access_sessions SET revoked = 1 WHERE token_lookup IS NULL",
            [],
        )?;
        self.conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_remote_access_sessions_token_lookup
             ON remote_access_sessions(token_lookup) WHERE token_lookup IS NOT NULL",
            [],
        )?;
        self.ensure_column("plugins", "plugin_key", "TEXT")?;
        self.ensure_column("plugins", "platform", "TEXT")?;
        self.ensure_column("plugins", "install_path", "TEXT")?;
        self.ensure_column("plugins", "enabled", "INTEGER DEFAULT 1")?;
        self.ensure_column("plugins", "repo_url", "TEXT")?;
        self.conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_plugins_plugin_key ON plugins(plugin_key) WHERE plugin_key IS NOT NULL",
            [],
        )?;

        // ── Premium defaults: ALL features ON ──
        let defaults = vec![
            ("theme", "vidhub_flagship"),
            ("window_width", "1400"),
            ("window_height", "900"),
            ("window_opacity", "100"),
            ("splash_enabled", "true"),
            ("sidebar_collapsed", "false"),
            ("motion_enabled", "true"),
            ("skip_intro", "true"),
            ("skip_outro", "true"),
            ("auto_next", "true"),
            ("auto_subtitles", "true"),
            ("chapter_thumbs_enabled", "true"),
            ("prefer_embedded_titles", "true"),
            ("default_player", "system"),
            ("smart_collections", "true"),
            ("poster_sync", "true"),
            ("unified_library", "true"),
            ("watchlist_enabled", "true"),
            ("hw_transcoding", "true"),
            ("quality_control", "auto"),
            ("particle_effects", "true"),
            ("ai_visualizer", "true"),
            ("glassmorphism", "true"),
            ("starfield_header", "true"),
            ("offline_mode", "false"),
            ("ai_model", "Qwen/Qwen3-4B-Instruct-2507"),
            ("hf_token", ""),
            ("synology_connection", ""),
            ("wd_mycloud_connection", ""),
            // Scheduled task defaults
            (
                "_scheduledTasks",
                r#"{"thumbnails":"on_scan","chapter_images":"on_scan","metadata_check":"daily","match_unmatch":"on_import"}"#,
            ),
        ];
        for (key, value) in defaults {
            self.conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }
        self.conn.execute(
            "UPDATE settings SET value = 'true' WHERE key = 'prefer_embedded_titles' AND value = 'false'",
            [],
        )?;
        self.conn.execute(
            "UPDATE settings SET value = ?1 WHERE key = 'ai_model' AND value = ?2",
            params![
                "Qwen/Qwen3-4B-Instruct-2507",
                "mistralai/Mistral-7B-Instruct-v0.3"
            ],
        )?;

        // ── Premium feature defaults: ALL enabled ──
        let features = vec![
            "smart_collections",
            "poster_sync",
            "unified_library",
            "watchlist",
            "skip_intro",
            "skip_outro",
            "auto_next",
            "auto_subtitles",
            "chapter_thumbs",
            "hw_transcoding",
            "motion_effects",
            "splash_screen",
            "particle_effects",
            "ai_visualizer",
            "glassmorphism",
            "starfield_header",
            "animated_sidebar",
            "emby_sdk",
            "vpn_integration",
            "ai_diagnostics",
            "duplicate_finder",
            "iptv_support",
            "plugin_system",
        ];
        for feature in features {
            self.conn.execute(
                "INSERT OR IGNORE INTO feature_settings (feature_key, enabled, config_json) VALUES (?1, 1, '{}')",
                params![feature],
            )?;
        }

        // ── Adult provider defaults: seed no-key providers so they are active immediately ──
        // Providers that work without a user-supplied API key are seeded with a sentinel value
        // so load_provider_keys() includes them in configured_adult_providers.
        let keyless_adult_providers = vec![
            // pgma: local sidecar bridge — no real key needed, sentinel enables it
            ("pgma", "pgma_local_bridge"),
            // porn_site_nuxt / IreneHub: local Nuxt server on localhost:42069
            ("porn_site_nuxt", "http://localhost:42069/"),
            // iafd: scrape-based, no API key required
            ("iafd", "iafd_scrape"),
            // phoenixadult: Jellyfin manifest provider, no key
            ("phoenixadult", "phoenixadult_manifest"),
        ];
        for (provider, default_value) in keyless_adult_providers {
            self.conn.execute(
                "INSERT OR IGNORE INTO api_keys (provider, api_key) VALUES (?1, ?2)",
                params![provider, default_value],
            )?;
        }

        // ── Plugin config_json defaults: seed all known plugins with functional configs ──
        // Each plugin gets a config_json row in the plugins table so the UI can show
        // and edit settings without requiring a separate install step.
        let plugin_configs: Vec<(&str, &str, &str)> = vec![
            // (plugin_key, name, config_json)
            // ── Adult Metadata Providers ──
            (
                "tpdb",
                "ThePornDB",
                r#"{"enabled":true,"api_key":"","base_url":"https://api.theporndb.net","search_limit":10,"include_adult":true,"auto_match":true,"poster_download":true,"nfo_write":true}"#,
            ),
            (
                "stashdb",
                "StashDB",
                r#"{"enabled":true,"api_key":"","endpoint":"https://stashdb.org/graphql","auto_match":true,"poster_download":true,"nfo_write":true}"#,
            ),
            (
                "pgma",
                "PGMA Modernized",
                r#"{"enabled":true,"mode":"local_sidecar_bridge","auto_match":true,"nfo_write":true,"poster_download":true}"#,
            ),
            (
                "porn_site_nuxt",
                "Porn Site Nuxt",
                r#"{"enabled":true,"base_url":"http://localhost:42069/","auto_match":true,"poster_download":true,"nfo_write":true}"#,
            ),
            (
                "iafd",
                "IAFD",
                r#"{"enabled":true,"base_url":"https://www.iafd.com","scrape_mode":true,"auto_match":true,"poster_download":true}"#,
            ),
            (
                "phoenixadult",
                "PhoenixAdult",
                r#"{"enabled":true,"manifest_url":"https://raw.githubusercontent.com/DirtyRacer1337/Jellyfin.Plugin.PhoenixAdult/master/manifest.json","auto_match":true,"poster_download":true}"#,
            ),
            // ── Standard Metadata Providers ──
            (
                "tmdb",
                "TMDb",
                r#"{"enabled":true,"api_key":"","base_url":"https://api.themoviedb.org/3","language":"en-US","include_adult":true,"poster_size":"w500","backdrop_size":"w1280","auto_match":true,"nfo_write":true}"#,
            ),
            (
                "omdb",
                "OMDb",
                r#"{"enabled":true,"api_key":"","base_url":"https://www.omdbapi.com","plot":"full","auto_match":true,"nfo_write":true}"#,
            ),
            (
                "tvdb",
                "TVDB",
                r#"{"enabled":true,"api_key":"","base_url":"https://api4.thetvdb.com/v4","language":"eng","auto_match":true,"nfo_write":true}"#,
            ),
            (
                "fanart",
                "Fanart.tv",
                r#"{"enabled":true,"api_key":"","base_url":"https://webservice.fanart.tv/v3","prefer_language":"en","poster_download":true,"backdrop_download":true}"#,
            ),
            (
                "trakt",
                "Trakt",
                r#"{"enabled":true,"client_id":"","client_secret":"","base_url":"https://api.trakt.tv","sync_watched":true,"sync_ratings":true,"sync_watchlist":true}"#,
            ),
            (
                "opensubtitles",
                "OpenSubtitles",
                r#"{"enabled":true,"api_key":"","base_url":"https://api.opensubtitles.com/api/v1","languages":["en"],"auto_download":true,"hearing_impaired":false}"#,
            ),
            (
                "anidb",
                "AniDB",
                r#"{"enabled":true,"client":"cinavault","clientver":1,"base_url":"https://api.anidb.net:9001/httpapi","auto_match":true,"nfo_write":true}"#,
            ),
            (
                "mal",
                "MyAnimeList",
                r#"{"enabled":true,"client_id":"","base_url":"https://api.myanimelist.net/v2","auto_match":true,"nfo_write":true}"#,
            ),
            (
                "kitsu",
                "Kitsu",
                r#"{"enabled":true,"base_url":"https://kitsu.io/api/edge","auto_match":true,"nfo_write":true}"#,
            ),
            (
                "anilist",
                "AniList",
                r#"{"enabled":true,"base_url":"https://graphql.anilist.co","auto_match":true,"nfo_write":true}"#,
            ),
            (
                "audiodb",
                "AudioDB",
                r#"{"enabled":true,"api_key":"2","base_url":"https://theaudiodb.com/api/v1/json","auto_match":true}"#,
            ),
            (
                "musicbrainz",
                "MusicBrainz",
                r#"{"enabled":true,"base_url":"https://musicbrainz.org/ws/2","format":"json","auto_match":true}"#,
            ),
            (
                "lastfm",
                "Last.fm",
                r#"{"enabled":true,"api_key":"","base_url":"https://ws.audioscrobbler.com/2.0","auto_scrobble":true}"#,
            ),
            (
                "discogs",
                "Discogs",
                r#"{"enabled":true,"token":"","base_url":"https://api.discogs.com","auto_match":true}"#,
            ),
            (
                "igdb",
                "IGDB",
                r#"{"enabled":true,"client_id":"","client_secret":"","base_url":"https://api.igdb.com/v4","auto_match":true}"#,
            ),
            (
                "tvmaze",
                "TVMaze",
                r#"{"enabled":true,"base_url":"https://api.tvmaze.com","auto_match":true,"nfo_write":true}"#,
            ),
            (
                "cinemeta",
                "Cinemeta",
                r#"{"enabled":true,"base_url":"https://v3-cinemeta.strem.io","auto_match":true}"#,
            ),
            // ── MS-C (Jellyfin) Plugins ──
            (
                "jf-open-subtitles",
                "OpenSubtitles (MS-C)",
                r#"{"enabled":true,"username":"","password":"","auto_download":true,"languages":["en"],"hearing_impaired":false,"foreign_parts_only":false}"#,
            ),
            (
                "jf-trakt",
                "Trakt (MS-C)",
                r#"{"enabled":true,"client_id":"","client_secret":"","sync_watched":true,"sync_ratings":true,"sync_watchlist":true,"sync_interval_hours":24}"#,
            ),
            (
                "jf-simkl",
                "Simkl (MS-C)",
                r#"{"enabled":true,"client_id":"","client_secret":"","auto_scrobble":true,"sync_watched":true}"#,
            ),
            (
                "jf-kodi-sync",
                "Kodi Sync Queue (MS-C)",
                r#"{"enabled":true,"retain_days":30,"auto_clean":true}"#,
            ),
            (
                "jf-webhook",
                "Webhook (MS-C)",
                r#"{"enabled":true,"endpoints":[],"notify_on_play":true,"notify_on_stop":true,"notify_on_new_item":true,"template":"discord"}"#,
            ),
            (
                "jf-playback-reporting",
                "Playback Reporting (MS-C)",
                r#"{"enabled":true,"retain_days":365,"keep_watching_items":true}"#,
            ),
            (
                "jf-session-cleaner",
                "Session Cleaner (MS-C)",
                r#"{"enabled":true,"max_session_age_days":30,"auto_clean":true,"clean_interval_hours":24}"#,
            ),
            (
                "jf-ldap",
                "LDAP Auth (MS-C)",
                r#"{"enabled":false,"server":"","port":389,"base_dn":"","bind_dn":"","bind_password":"","user_filter":"(objectClass=person)","use_ssl":false}"#,
            ),
            (
                "jf-dlna",
                "DLNA (MS-C)",
                r#"{"enabled":true,"server_name":"CinaVault","alive_message_interval_seconds":1800,"auto_start":true}"#,
            ),
            (
                "jf-chapter-segments",
                "Chapter Segments (MS-C)",
                r#"{"enabled":true,"auto_detect":true,"skip_intro":true,"skip_outro":true,"min_segment_seconds":10}"#,
            ),
            // ── MS-B (Emby) Plugins ──
            (
                "em-bookshelf",
                "Bookshelf (MS-B)",
                r#"{"enabled":true,"scan_epub":true,"scan_pdf":true,"scan_audiobook":true,"metadata_language":"en"}"#,
            ),
            (
                "em-bulky",
                "Bulky (MS-B)",
                r#"{"enabled":true,"batch_size":50,"auto_apply":false}"#,
            ),
            (
                "em-gamebrowser",
                "GameBrowser (MS-B)",
                r#"{"enabled":true,"emulator_path":"","roms_path":"","auto_scan":true}"#,
            ),
            // ── MS-A (Plex) Plugins ──
            (
                "px-hama",
                "HAMA (MS-A)",
                r#"{"enabled":true,"anidb_client":"cinavault","anidb_clientver":1,"prefer_anidb":true,"use_tvdb_fallback":true,"use_mal_fallback":true,"poster_language":"en"}"#,
            ),
            (
                "px-ass",
                "Absolute Series Scanner (MS-A)",
                r#"{"enabled":true,"absolute_numbering":true,"anime_mode":true}"#,
            ),
            (
                "px-kometa",
                "Kometa (MS-A)",
                r#"{"enabled":true,"config_path":"","run_interval_hours":24,"overlay_update":true,"collection_update":true}"#,
            ),
            (
                "px-bazarr",
                "Bazarr (MS-A)",
                r#"{"enabled":true,"host":"localhost","port":6767,"api_key":"","languages":["en"],"auto_download":true}"#,
            ),
            (
                "px-lambda",
                "Lambda (MS-A)",
                r#"{"enabled":true,"prefer_local":false,"fallback_tmdb":true}"#,
            ),
            (
                "px-kitana",
                "Kitana (MS-A)",
                r#"{"enabled":true,"host":"localhost","port":31337}"#,
            ),
            (
                "px-webtools",
                "WebTools (MS-A)",
                r#"{"enabled":true,"port":33400,"auto_update":true}"#,
            ),
            (
                "px-filebot",
                "FileBot (MS-A)",
                r#"{"enabled":true,"rename_format":"{n} ({y})","db":"TheMovieDB","lang":"en","non_strict":true}"#,
            ),
            // ── CinaVault Native Plugins ──
            (
                "cv-unified-adapter",
                "CinaVault Unified Adapter",
                r#"{"enabled":true,"compat_mode":"auto","api_translation":true,"event_bridge":true}"#,
            ),
            (
                "cv-metadata-engine",
                "CinaVault Metadata Engine",
                r#"{"enabled":true,"providers":["tmdb","omdb","tvdb","fanart","tpdb","stashdb","pgma","porn_site_nuxt","iafd","phoenixadult","trakt","anidb","mal"],"conflict_resolution":"highest_confidence","merge_strategy":"union","auto_enrich":true,"poster_download":true,"nfo_write":true,"batch_size":20}"#,
            ),
            (
                "cv-ai-match",
                "AI Media Matcher",
                r#"{"enabled":true,"model":"Qwen/Qwen3-4B-Instruct-2507","confidence_threshold":0.75,"use_audio_fingerprint":true,"use_visual_recognition":false,"fallback_to_filename":true}"#,
            ),
            (
                "cv-cloud-sync",
                "Cloud Sync Engine",
                r#"{"enabled":true,"providers":[],"sync_interval_minutes":60,"sync_watched":true,"sync_ratings":true,"sync_metadata":false,"conflict_resolution":"newest_wins"}"#,
            ),
            (
                "cv-thumb-gen",
                "Smart Thumbnail Generator",
                r#"{"enabled":true,"interval_seconds":300,"max_thumbs_per_item":5,"scene_detection":true,"face_detection":false,"composition_score":true,"output_format":"jpg","quality":85}"#,
            ),
            (
                "cv-chapter-detect",
                "Chapter Image Detector",
                r#"{"enabled":true,"auto_extract":true,"interval_seconds":600,"max_chapters":30,"output_format":"jpg","quality":85,"skip_existing":true}"#,
            ),
            (
                "cv-dup-finder",
                "Duplicate Finder",
                r#"{"enabled":true,"match_by":"hash","tolerance_mb":1,"auto_scan":false,"scan_interval_hours":168,"prefer_higher_resolution":true,"prefer_larger_file":false}"#,
            ),
            (
                "cv-vpn-manager",
                "VPN Manager",
                r#"{"enabled":false,"provider":"","config_path":"","auto_connect":false,"kill_switch":true,"reconnect_on_drop":true}"#,
            ),
            (
                "cv-transcode-engine",
                "Hardware Transcode Engine",
                r#"{"enabled":true,"hardware_acceleration":"auto","preferred_codec":"h264","max_bitrate_mbps":20,"crf":23,"preset":"fast","audio_codec":"aac","audio_bitrate_kbps":192}"#,
            ),
            (
                "cv-intro-skip",
                "Intro/Outro Skip Engine",
                r#"{"enabled":true,"detection_method":"chromaprint","min_intro_seconds":10,"max_intro_seconds":300,"skip_on_play":true,"show_skip_button":true}"#,
            ),
            // ── Download/Automation ──
            (
                "yt-dlp",
                "yt-dlp",
                r#"{"enabled":true,"format":"bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best","output_template":"%(title)s.%(ext)s","embed_thumbnail":true,"embed_subs":true,"write_info_json":false,"rate_limit":"50M","concurrent_fragments":4,"retries":3}"#,
            ),
            (
                "ffmpeg",
                "FFmpeg",
                r#"{"enabled":true,"hwaccel":"auto","threads":0,"loglevel":"error","default_video_codec":"libx264","default_audio_codec":"aac","default_subtitle_codec":"srt"}"#,
            ),
            (
                "mediainfo",
                "MediaInfo",
                r#"{"enabled":true,"full_output":false,"output_format":"JSON","cover_data":false}"#,
            ),
            (
                "mkvtoolnix",
                "MKVToolNix",
                r#"{"enabled":true,"default_language":"eng","generate_chapters":false,"attach_fonts":false,"compression":"none"}"#,
            ),
        ];
        for (plugin_key, name, config_json) in &plugin_configs {
            self.conn.execute(
                "INSERT OR IGNORE INTO plugins (plugin_key, name, installed, enabled, config_json) VALUES (?1, ?2, 1, 1, ?3)",
                params![plugin_key, name, config_json],
            )?;
        }

        self.cleanup_non_library_photo_artifacts()?;
        Ok(())
    }
    fn cleanup_non_library_photo_artifacts(&self) -> SqlResult<()> {
        // Remove ALL photo-type rows — they are poster/artwork files, not standalone media.
        // This covers: chapter images, sidecar artwork, video-matched posters, and any other
        // image file that was incorrectly ingested as a media item.
        self.conn
            .execute("DELETE FROM media_items WHERE media_type = 'photo'", [])?;
        Ok(())
    }

    #[cfg(test)]
    fn sync_sidecar_artwork_for_video_rows(&self) -> SqlResult<()> {
        let rows = {
            let mut stmt = self.conn.prepare(
                "SELECT file_path
                 FROM media_items
                 WHERE media_type IN ('adult', 'movie', 'episode', 'video')
                   AND (poster_path IS NULL OR trim(poster_path) = '')",
            )?;
            let iter = stmt.query_map([], |row| row.get::<_, String>(0))?;
            iter.collect::<Result<Vec<_>, _>>()?
        };

        for file_path in rows {
            let Some(poster_path) = sidecar_poster_path_for_video(Path::new(&file_path)) else {
                continue;
            };
            self.conn.execute(
                "UPDATE media_items
                 SET poster_path = ?1
                 WHERE file_path = ?2
                   AND (poster_path IS NULL OR trim(poster_path) = '')",
                params![poster_path.to_string_lossy().to_string(), file_path],
            )?;
        }

        Ok(())
    }

    fn ensure_column(&self, table: &str, column: &str, definition: &str) -> SqlResult<()> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut exists = false;
        for existing in columns {
            if existing?.eq_ignore_ascii_case(column) {
                exists = true;
                break;
            }
        }

        if !exists {
            self.conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
                [],
            )?;
        }
        Ok(())
    }

    // ── Settings ──
    pub fn get_all_settings_data(&self) -> SqlResult<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect()
    }

    pub fn get_setting_data(&self, key: &str) -> SqlResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(val)) => Ok(Some(val)),
            _ => Ok(None),
        }
    }

    pub fn set_setting_data(&self, key: &str, value: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    // ── Feature settings ──
    pub fn get_feature_settings_data(&self) -> SqlResult<Vec<serde_json::Value>> {
        let mut stmt = self
            .conn
            .prepare("SELECT feature_key, enabled, config_json FROM feature_settings")?;
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let enabled: bool = row.get(1)?;
            let config: String = row.get(2)?;
            Ok(serde_json::json!({
                "key": key,
                "enabled": enabled,
                "config": serde_json::from_str::<serde_json::Value>(&config).unwrap_or_default()
            }))
        })?;
        rows.collect()
    }

    pub fn set_feature_setting_data(
        &self,
        key: &str,
        enabled: bool,
        config: &str,
    ) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO feature_settings (feature_key, enabled, config_json) VALUES (?1, ?2, ?3)",
            params![key, enabled, config],
        )?;
        Ok(())
    }

    // ── Media items ──
    pub fn mark_current_library_adult(&mut self) -> SqlResult<AdultLibraryLabelResult> {
        let transaction = self.conn.transaction()?;
        let inventory_items =
            transaction.query_row("SELECT COUNT(*) FROM media_items", [], |row| {
                row.get::<_, usize>(0)
            })?;
        let items_already_adult = transaction.query_row(
            "SELECT COUNT(*) FROM media_items WHERE lower(trim(media_type)) = 'adult'",
            [],
            |row| row.get::<_, usize>(0),
        )?;
        let items_labeled_adult = transaction.execute(
            "UPDATE media_items
             SET media_type = 'adult'
             WHERE lower(trim(media_type)) <> 'adult'",
            [],
        )?;
        transaction.commit()?;

        Ok(AdultLibraryLabelResult {
            inventory_items,
            items_labeled_adult,
            items_already_adult,
        })
    }

    pub fn get_media_items_data(
        &self,
        media_type: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> SqlResult<Vec<MediaItem>> {
        let off = offset.unwrap_or(0);
        match (media_type, limit) {
            (Some(mt), Some(lim)) => {
                let mut stmt = self.conn.prepare(
                    "SELECT * FROM media_items WHERE media_type = ?1 ORDER BY date_added DESC LIMIT ?2 OFFSET ?3"
                )?;
                let rows = stmt.query_map(params![mt, lim, off], Self::row_to_media)?;
                rows.collect()
            }
            (Some(mt), None) => {
                let mut stmt = self.conn.prepare(
                    "SELECT * FROM media_items WHERE media_type = ?1 ORDER BY date_added DESC",
                )?;
                let rows = stmt.query_map(params![mt], Self::row_to_media)?;
                rows.collect()
            }
            (None, Some(lim)) => {
                let mut stmt = self.conn.prepare(
                    "SELECT * FROM media_items ORDER BY date_added DESC LIMIT ?1 OFFSET ?2",
                )?;
                let rows = stmt.query_map(params![lim, off], Self::row_to_media)?;
                rows.collect()
            }
            (None, None) => {
                let mut stmt = self
                    .conn
                    .prepare("SELECT * FROM media_items ORDER BY date_added DESC")?;
                let rows = stmt.query_map([], Self::row_to_media)?;
                rows.collect()
            }
        }
    }

    pub fn add_media_item_data(&self, item: &MediaItem) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT OR IGNORE INTO media_items (title, file_path, media_type, year, rating, overview, poster_path, backdrop_path, genre, duration, file_size, resolution, codec, verified, watched, favorite, date_added, tmdb_id, imdb_id, source_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                item.title, item.file_path, item.media_type, item.year, item.rating,
                item.overview, item.poster_path, item.backdrop_path, item.genre,
                item.duration, item.file_size, item.resolution, item.codec,
                item.verified, item.watched, item.favorite, item.date_added,
                item.tmdb_id, item.imdb_id, item.source_id
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn upsert_scanned_media_item_data(&self, item: &MediaItem) -> SqlResult<bool> {
        let existing_id = self.conn.query_row(
            "SELECT id FROM media_items WHERE file_path = ?1",
            params![&item.file_path],
            |row| row.get::<_, i64>(0),
        );

        match existing_id {
            Ok(id) => {
                self.conn.execute(
                    "UPDATE media_items
                     SET title = ?1,
                         media_type = ?2,
                         file_size = ?3,
                         source_id = ?4,
                         poster_path = CASE
                             WHEN (poster_path IS NULL OR trim(poster_path) = '')
                                  AND ?5 IS NOT NULL
                                  AND trim(?5) <> ''
                             THEN ?5
                             ELSE poster_path
                         END
                     WHERE id = ?6",
                    params![
                        item.title,
                        item.media_type,
                        item.file_size,
                        item.source_id,
                        item.poster_path,
                        id
                    ],
                )?;
                Ok(false)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                self.add_media_item_data(item)?;
                Ok(true)
            }
            Err(err) => Err(err),
        }
    }

    pub fn update_media_metadata_data(
        &self,
        file_path: &str,
        title: Option<&str>,
        overview: Option<&str>,
        poster_path: Option<&str>,
        year: Option<i32>,
        rating: Option<f64>,
        genre: Option<&str>,
        tmdb_id: Option<&str>,
        imdb_id: Option<&str>,
        media_type: Option<&str>,
    ) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE media_items
             SET title = COALESCE(?1, title),
                 overview = COALESCE(?2, overview),
                 poster_path = COALESCE(?3, poster_path),
                 year = COALESCE(?4, year),
                 rating = COALESCE(?5, rating),
                 genre = COALESCE(?6, genre),
                 tmdb_id = COALESCE(?7, tmdb_id),
                 imdb_id = COALESCE(?8, imdb_id),
                 media_type = COALESCE(?9, media_type)
             WHERE file_path = ?10",
            params![
                title,
                overview,
                poster_path,
                year,
                rating,
                genre,
                tmdb_id,
                imdb_id,
                media_type,
                file_path,
            ],
        )?;
        Ok(())
    }

    pub fn update_media_file_path_data(
        &self,
        old_file_path: &str,
        new_file_path: &str,
        new_title: &str,
    ) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE media_items
             SET file_path = ?1,
                 title = ?2
             WHERE file_path = ?3",
            params![new_file_path, new_title, old_file_path],
        )?;
        Ok(())
    }

    pub fn search_media_data(&self, query: &str) -> SqlResult<Vec<MediaItem>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT * FROM media_items WHERE title LIKE ?1 OR genre LIKE ?1 OR overview LIKE ?1 ORDER BY title"
        )?;
        let rows = stmt.query_map(params![pattern], |row| Self::row_to_media(row))?;
        rows.collect()
    }

    pub fn get_recent_media_data(&self, limit: i64) -> SqlResult<Vec<MediaItem>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM media_items ORDER BY date_added DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit], |row| Self::row_to_media(row))?;
        rows.collect()
    }

    pub fn get_unverified_media_data(&self) -> SqlResult<Vec<MediaItem>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM media_items WHERE verified = 0 ORDER BY date_added DESC")?;
        let rows = stmt.query_map([], |row| Self::row_to_media(row))?;
        rows.collect()
    }

    // ── Sources ──
    pub fn get_sources_data(&self) -> SqlResult<Vec<MediaSource>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM media_sources ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(MediaSource {
                id: Some(row.get(0)?),
                path: row.get(1)?,
                source_type: row.get(2)?,
                name: row.get(3)?,
                enabled: row.get(4)?,
                last_scanned: row.get(5)?,
                item_count: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn add_source_data(&self, source: &MediaSource) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT OR IGNORE INTO media_sources (path, source_type, name, enabled, item_count) VALUES (?1,?2,?3,?4,?5)",
            params![source.path, source.source_type, source.name, source.enabled, source.item_count],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn remove_source_data(&self, id: i64) -> SqlResult<()> {
        self.conn
            .execute("DELETE FROM media_items WHERE source_id = ?1", params![id])?;
        self.conn
            .execute("DELETE FROM media_sources WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn create_remote_access_user(
        &self,
        email: &str,
        password: &str,
        display_name: Option<&str>,
    ) -> Result<RemoteAccessUserProvision, String> {
        let email = normalize_remote_email(email)?;
        validate_remote_password(password)?;

        let now = chrono::Utc::now().to_rfc3339();
        // The PHC string produced by Argon2id includes the random salt and cost parameters.
        let password_salt = "argon2id".to_string();
        let password_hash = hash_remote_password(password)?;
        let access_key = generate_remote_access_key();
        let access_key_salt = new_secret_salt();
        let access_key_hash = hash_secret(&access_key_salt, &access_key);
        let access_key_preview = preview_secret(&access_key);
        let display_name = display_name
            .and_then(|value| non_empty_trimmed(value))
            .or_else(|| Some(email.clone()));

        self.conn
            .execute(
                "INSERT INTO remote_access_users
                 (email, display_name, password_salt, password_hash, access_key_salt,
                  access_key_hash, access_key_preview, enabled, permissions, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 'server:read,library:read,stream:play', ?8, ?8)
                 ON CONFLICT(email) DO UPDATE SET
                   display_name = excluded.display_name,
                   password_salt = excluded.password_salt,
                   password_hash = excluded.password_hash,
                   access_key_salt = excluded.access_key_salt,
                   access_key_hash = excluded.access_key_hash,
                   access_key_preview = excluded.access_key_preview,
                   enabled = 1,
                   updated_at = excluded.updated_at",
                params![
                    email,
                    display_name,
                    password_salt,
                    password_hash,
                    access_key_salt,
                    access_key_hash,
                    access_key_preview,
                    now,
                ],
            )
            .map_err(|err| err.to_string())?;

        let row = self
            .conn
            .query_row(
                "SELECT id, email, display_name, enabled, created_at
                 FROM remote_access_users
                 WHERE email = ?1",
                params![email],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, bool>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .map_err(|err| err.to_string())?;

        Ok(RemoteAccessUserProvision {
            id: row.0,
            email: row.1,
            display_name: row.2,
            access_key: access_key.clone(),
            access_key_preview: preview_secret(&access_key),
            enabled: row.3,
            created_at: row.4,
        })
    }

    pub fn authenticate_remote_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<RemoteAccessPrincipal>, String> {
        let email = normalize_remote_email(email)?;
        let row = self
            .conn
            .query_row(
                "SELECT id, email, display_name, password_salt, password_hash, enabled, permissions
                 FROM remote_access_users
                 WHERE email = ?1",
                params![email],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, bool>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(|err| err.to_string())?;

        let Some((id, email, display_name, salt, expected_hash, enabled, permissions)) = row else {
            return Ok(None);
        };
        if !enabled {
            return Ok(None);
        }
        let verified = if is_argon2_password_hash(&expected_hash) {
            verify_argon2_password(password, &expected_hash)
        } else {
            // Legacy SHA-256 hashes are accepted once, then upgraded after a successful login.
            let actual_hash = hash_secret(&salt, password);
            constant_time_eq(&actual_hash, &expected_hash)
        };
        if !verified {
            return Ok(None);
        }

        if !is_argon2_password_hash(&expected_hash) {
            let upgraded_hash = hash_remote_password(password)?;
            self.conn
                .execute(
                    "UPDATE remote_access_users
                     SET password_salt = 'argon2id', password_hash = ?1, updated_at = ?2
                     WHERE id = ?3 AND password_hash = ?4",
                    params![
                        upgraded_hash,
                        chrono::Utc::now().to_rfc3339(),
                        id,
                        expected_hash
                    ],
                )
                .map_err(|err| err.to_string())?;
        }

        self.create_remote_access_session(id, email, display_name, "password", &permissions)
    }

    pub fn authenticate_remote_access_key(
        &self,
        access_key: &str,
    ) -> Result<Option<RemoteAccessPrincipal>, String> {
        let access_key = access_key.trim();
        if access_key.is_empty() {
            return Ok(None);
        }

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, email, display_name, access_key_salt, access_key_hash, enabled, permissions
                 FROM remote_access_users",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, bool>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|err| err.to_string())?;

        for row in rows {
            let (id, email, display_name, salt, expected_hash, enabled, permissions) =
                row.map_err(|err| err.to_string())?;
            if !enabled {
                continue;
            }
            let actual_hash = hash_secret(&salt, access_key);
            if constant_time_eq(&actual_hash, &expected_hash) {
                return self.create_remote_access_session(
                    id,
                    email,
                    display_name,
                    "access_key",
                    &permissions,
                );
            }
        }

        Ok(None)
    }

    pub fn rotate_remote_access_key(
        &self,
        email: &str,
    ) -> Result<Option<RemoteAccessKeyRotation>, String> {
        let email = normalize_remote_email(email)?;
        let access_key = generate_remote_access_key();
        let access_key_salt = new_secret_salt();
        let access_key_hash = hash_secret(&access_key_salt, &access_key);
        let access_key_preview = preview_secret(&access_key);
        let updated_at = chrono::Utc::now().to_rfc3339();

        let changed = self
            .conn
            .execute(
                "UPDATE remote_access_users
                 SET access_key_salt = ?1,
                     access_key_hash = ?2,
                     access_key_preview = ?3,
                     updated_at = ?4
                 WHERE email = ?5",
                params![
                    access_key_salt,
                    access_key_hash,
                    access_key_preview,
                    updated_at,
                    email
                ],
            )
            .map_err(|err| err.to_string())?;
        if changed == 0 {
            return Ok(None);
        }

        Ok(Some(RemoteAccessKeyRotation {
            email,
            access_key: access_key.clone(),
            access_key_preview: preview_secret(&access_key),
        }))
    }

    pub fn set_remote_access_user_enabled(&self, email: &str, enabled: bool) -> Result<(), String> {
        let email = normalize_remote_email(email)?;
        self.conn
            .execute(
                "UPDATE remote_access_users
                 SET enabled = ?1, updated_at = ?2
                 WHERE email = ?3",
                params![enabled, chrono::Utc::now().to_rfc3339(), email],
            )
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn list_remote_access_users(&self) -> Result<Vec<RemoteAccessUserSummary>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, email, display_name, access_key_preview, enabled, permissions,
                        created_at, updated_at, last_login
                 FROM remote_access_users
                 ORDER BY email",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let permissions: String = row.get(5)?;
                Ok(RemoteAccessUserSummary {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    display_name: row.get(2)?,
                    access_key_preview: row.get(3)?,
                    enabled: row.get(4)?,
                    permissions: parse_permissions(&permissions),
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    last_login: row.get(8)?,
                })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn validate_remote_access_session(
        &self,
        session_token: &str,
    ) -> Result<Option<RemoteAccessPrincipal>, String> {
        if session_token.trim().len() < 32 || session_token.len() > 512 {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();
        let token_lookup = hash_session_token_lookup(session_token);
        let row = self
            .conn
            .query_row(
                "SELECT s.token_salt, s.token_hash, s.auth_method, s.expires_at,
                        u.id, u.email, u.display_name, u.permissions
                 FROM remote_access_sessions s
                 INNER JOIN remote_access_users u ON u.id = s.user_id
                 WHERE s.token_lookup = ?1 AND s.revoked = 0 AND s.expires_at > ?2 AND u.enabled = 1",
                params![token_lookup, now],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some((
            token_salt,
            token_hash,
            auth_method,
            expires_at,
            id,
            email,
            display_name,
            permissions,
        )) = row
        else {
            return Ok(None);
        };
        let candidate_hash = hash_secret(&token_salt, session_token);
        if !constant_time_eq(&candidate_hash, &token_hash) {
            return Ok(None);
        }

        Ok(Some(RemoteAccessPrincipal {
            id,
            email,
            display_name,
            auth_method,
            session_token: session_token.to_string(),
            expires_at,
            permissions: parse_permissions(&permissions),
        }))
    }

    fn create_remote_access_session(
        &self,
        user_id: i64,
        email: String,
        display_name: Option<String>,
        auth_method: &str,
        permissions: &str,
    ) -> Result<Option<RemoteAccessPrincipal>, String> {
        let session_token = generate_remote_session_token();
        let token_salt = new_secret_salt();
        let token_hash = hash_secret(&token_salt, &session_token);
        let token_lookup = hash_session_token_lookup(&session_token);
        let created_at = chrono::Utc::now();
        let expires_at = created_at + chrono::Duration::hours(12);
        let created_at = created_at.to_rfc3339();
        let expires_at_string = expires_at.to_rfc3339();

        self.conn
            .execute(
                "INSERT INTO remote_access_sessions
                 (user_id, token_salt, token_hash, token_lookup, auth_method, created_at, expires_at, revoked)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
                params![
                    user_id,
                    token_salt,
                    token_hash,
                    token_lookup,
                    auth_method,
                    created_at,
                    expires_at_string
                ],
            )
            .map_err(|err| err.to_string())?;
        self.conn
            .execute(
                "UPDATE remote_access_users SET last_login = ?1 WHERE id = ?2",
                params![created_at, user_id],
            )
            .map_err(|err| err.to_string())?;

        Ok(Some(RemoteAccessPrincipal {
            id: user_id,
            email,
            display_name,
            auth_method: auth_method.to_string(),
            session_token,
            expires_at: expires_at_string,
            permissions: parse_permissions(permissions),
        }))
    }

    fn row_to_media(row: &rusqlite::Row) -> rusqlite::Result<MediaItem> {
        Ok(MediaItem {
            id: Some(row.get(0)?),
            title: row.get(1)?,
            file_path: row.get(2)?,
            media_type: row.get(3)?,
            year: row.get(4)?,
            rating: row.get(5)?,
            overview: row.get(6)?,
            poster_path: row.get(7)?,
            backdrop_path: row.get(8)?,
            genre: row.get(9)?,
            duration: row.get(10)?,
            file_size: row.get(11)?,
            resolution: row.get(12)?,
            codec: row.get(13)?,
            verified: row.get(14)?,
            watched: row.get(15)?,
            favorite: row.get(16)?,
            date_added: row.get(17)?,
            last_played: row.get(18)?,
            tmdb_id: row.get(19)?,
            imdb_id: row.get(20)?,
            source_id: row.get(21)?,
        })
    }
}

fn normalize_remote_email(email: &str) -> Result<String, String> {
    let email = email.trim().to_ascii_lowercase();
    if email.len() < 5 || !email.contains('@') {
        return Err("A valid email address is required.".to_string());
    }
    let Some((local, domain)) = email.split_once('@') else {
        return Err("A valid email address is required.".to_string());
    };
    if local.trim().is_empty() || !domain.contains('.') || domain.ends_with('.') {
        return Err("A valid email address is required.".to_string());
    }
    Ok(email)
}

fn validate_remote_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Remote access passwords must be at least 8 characters.".to_string());
    }
    Ok(())
}

fn non_empty_trimmed(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn new_secret_salt() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn generate_remote_access_key() -> String {
    format!(
        "cvra_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn generate_remote_session_token() -> String {
    format!(
        "cvrs_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn preview_secret(secret: &str) -> String {
    let chars = secret.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(8);
    chars[start..].iter().collect()
}

fn hash_remote_password(password: &str) -> Result<String, String> {
    let random_salt = uuid::Uuid::new_v4();
    let salt = SaltString::encode_b64(random_salt.as_bytes())
        .map_err(|error| format!("Unable to generate remote-access password salt: {error}"))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| format!("Unable to hash remote-access password: {error}"))
}

fn is_argon2_password_hash(password_hash: &str) -> bool {
    password_hash.starts_with("$argon2")
}

fn verify_argon2_password(password: &str, password_hash: &str) -> bool {
    PasswordHash::new(password_hash)
        .ok()
        .and_then(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .ok()
        })
        .is_some()
}

fn hash_session_token_lookup(session_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"cinavault-remote-session-lookup-v1:");
    hasher.update(session_token.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

// High-entropy access and session tokens may use a fast salted hash; user passwords must not.
fn hash_secret(salt: &str, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":cinavault-remote-access:");
    hasher.update(secret.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn parse_permissions(permissions: &str) -> Vec<String> {
    permissions
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

// ════════════════════════════════════════════════════════════
//  Tauri Commands
// ════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_all_settings(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let settings = db.get_all_settings_data().map_err(|e| e.to_string())?;
    let mut map = serde_json::Map::new();
    for (k, v) in settings {
        map.insert(k, serde_json::Value::String(v));
    }
    Ok(serde_json::Value::Object(map))
}

#[tauri::command]
pub fn get_setting(state: State<AppState>, key: String) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_setting_data(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_setting_data(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_feature_settings(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_feature_settings_data().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_feature_setting(
    state: State<AppState>,
    key: String,
    enabled: bool,
    config: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_feature_setting_data(&key, enabled, &config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_remote_access_user(
    state: State<AppState>,
    email: String,
    password: String,
    display_name: Option<String>,
) -> Result<RemoteAccessUserProvision, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_remote_access_user(&email, &password, display_name.as_deref())
}

#[tauri::command]
pub fn authenticate_remote_password(
    state: State<AppState>,
    email: String,
    password: String,
) -> Result<Option<RemoteAccessPrincipal>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.authenticate_remote_password(&email, &password)
}

#[tauri::command]
pub fn authenticate_remote_access_key(
    state: State<AppState>,
    access_key: String,
) -> Result<Option<RemoteAccessPrincipal>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.authenticate_remote_access_key(&access_key)
}

#[tauri::command]
pub fn rotate_remote_access_key(
    state: State<AppState>,
    email: String,
) -> Result<Option<RemoteAccessKeyRotation>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rotate_remote_access_key(&email)
}

#[tauri::command]
pub fn set_remote_access_user_enabled(
    state: State<AppState>,
    email: String,
    enabled: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_remote_access_user_enabled(&email, enabled)
}

#[tauri::command]
pub fn list_remote_access_users(
    state: State<AppState>,
) -> Result<Vec<RemoteAccessUserSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_remote_access_users()
}

#[tauri::command]
pub fn get_remote_access_security_status(
    state: State<AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user_count = db.list_remote_access_users()?.len();
    let remote_enabled = db
        .get_setting_data("remote_access_enabled")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "true".to_string());
    let secure_mode = db
        .get_setting_data("remote_secure_connections")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "preferred".to_string());
    let public_port = db
        .get_setting_data("remote_public_port")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "32400".to_string());

    Ok(serde_json::json!({
        "remote_enabled": remote_enabled != "false",
        "secure_mode": secure_mode,
        "public_port": public_port,
        "account_count": user_count,
        "password_auth": true,
        "access_key_auth": true,
        "session_hours": 12,
        "permissions": ["server:read", "library:read", "stream:play"],
    }))
}

#[tauri::command]
pub fn get_media_items(
    state: State<AppState>,
    media_type: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<MediaItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_media_items_data(media_type.as_deref(), limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_item(state: State<AppState>, id: i64) -> Result<Option<MediaItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let items = db
        .get_media_items_data(None, Some(1), None)
        .map_err(|e| e.to_string())?;
    Ok(items.into_iter().find(|i| i.id == Some(id)))
}

#[tauri::command]
pub fn add_media_item(state: State<AppState>, item: MediaItem) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.add_media_item_data(&item).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_media_item(
    state: State<AppState>,
    id: i64,
    title: Option<String>,
    verified: Option<bool>,
    watched: Option<bool>,
    favorite: Option<bool>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(t) = title {
        db.conn
            .execute(
                "UPDATE media_items SET title = ?1 WHERE id = ?2",
                params![t, id],
            )
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = verified {
        db.conn
            .execute(
                "UPDATE media_items SET verified = ?1 WHERE id = ?2",
                params![v, id],
            )
            .map_err(|e| e.to_string())?;
    }
    if let Some(w) = watched {
        db.conn
            .execute(
                "UPDATE media_items SET watched = ?1 WHERE id = ?2",
                params![w, id],
            )
            .map_err(|e| e.to_string())?;
    }
    if let Some(f) = favorite {
        db.conn
            .execute(
                "UPDATE media_items SET favorite = ?1 WHERE id = ?2",
                params![f, id],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_media_item(state: State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM media_items WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Purge ALL media_type = 'photo' rows from the library.
/// These are poster/artwork image files incorrectly ingested as standalone media items.
/// Returns the count of rows removed.
#[tauri::command]
pub fn purge_photo_items(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let count: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM media_items WHERE media_type = 'photo'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM media_items WHERE media_type = 'photo'", [])
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "type": "purge_photo_items",
        "status": "success",
        "rows_removed": count,
        "message": format!("Removed {} photo/poster items from library", count),
    }))
}

#[tauri::command]
pub fn search_media(state: State<AppState>, query: String) -> Result<Vec<MediaItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search_media_data(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recent_media(
    state: State<AppState>,
    limit: Option<i64>,
) -> Result<Vec<MediaItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_recent_media_data(limit.unwrap_or(20))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_unverified_media(state: State<AppState>) -> Result<Vec<MediaItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_unverified_media_data().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn verify_media_item(state: State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn
        .execute(
            "UPDATE media_items SET verified = 1 WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_sources(state: State<AppState>) -> Result<Vec<MediaSource>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_sources_data().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_source(
    state: State<AppState>,
    path: String,
    source_type: String,
    name: String,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let source = MediaSource {
        id: None,
        path,
        source_type,
        name,
        enabled: true,
        last_scanned: None,
        item_count: 0,
    };
    db.add_source_data(&source).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_source(state: State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.remove_source_data(id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{hash_secret, hash_session_token_lookup, new_secret_salt, Database, MediaItem};
    use rusqlite::params;
    use std::fs;

    fn test_db_path(name: &str) -> String {
        let mut path = std::env::temp_dir();
        path.push(format!("cinavault-{name}-{}.db", uuid::Uuid::new_v4()));
        path.to_string_lossy().to_string()
    }

    fn sample_item(title: &str, file_path: &str) -> MediaItem {
        MediaItem {
            id: None,
            title: title.to_string(),
            file_path: file_path.to_string(),
            media_type: "movie".to_string(),
            year: None,
            rating: None,
            overview: None,
            poster_path: None,
            backdrop_path: None,
            genre: None,
            duration: None,
            file_size: Some(100),
            resolution: None,
            codec: None,
            verified: false,
            watched: false,
            favorite: false,
            date_added: "2026-05-06T00:00:00Z".to_string(),
            last_played: None,
            tmdb_id: None,
            imdb_id: None,
            source_id: None,
        }
    }

    #[test]
    fn current_inventory_adult_label_is_idempotent_and_preserves_artwork() {
        let db_path = test_db_path("current-inventory-adult");
        let mut db = Database::new(&db_path).expect("db should open");

        let mut movie = sample_item("Movie", r"C:\media\movie.mkv");
        movie.poster_path = Some(r"C:\media\movie-poster.jpg".to_string());
        movie.backdrop_path = Some(r"C:\media\movie-fanart.jpg".to_string());
        db.add_media_item_data(&movie)
            .expect("movie fixture should insert");

        let mut existing_adult = sample_item("Existing Adult", r"C:\media\adult.mkv");
        existing_adult.media_type = "Adult".to_string();
        db.add_media_item_data(&existing_adult)
            .expect("adult fixture should insert");

        let first = db
            .mark_current_library_adult()
            .expect("current inventory should be labeled");
        assert_eq!(first.inventory_items, 2);
        assert_eq!(first.items_labeled_adult, 1);
        assert_eq!(first.items_already_adult, 1);

        let preserved = db
            .conn
            .query_row(
                "SELECT media_type, poster_path, backdrop_path FROM media_items WHERE file_path = ?1",
                params![&movie.file_path],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .expect("labeled movie should remain readable");
        assert_eq!(preserved.0, "adult");
        assert_eq!(preserved.1, movie.poster_path);
        assert_eq!(preserved.2, movie.backdrop_path);

        let second = db
            .mark_current_library_adult()
            .expect("repeated labeling should succeed");
        assert_eq!(second.inventory_items, 2);
        assert_eq!(second.items_labeled_adult, 0);
        assert_eq!(second.items_already_adult, 2);

        let future_import = sample_item("Future Import", r"C:\media\future.mkv");
        db.add_media_item_data(&future_import)
            .expect("future import should insert normally");
        let future_type = db
            .conn
            .query_row(
                "SELECT media_type FROM media_items WHERE file_path = ?1",
                params![&future_import.file_path],
                |row| row.get::<_, String>(0),
            )
            .expect("future import should remain readable");
        assert_eq!(future_type, "movie");

        drop(db);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn scan_upsert_updates_existing_title_without_overwriting_user_flags() {
        let db_path = test_db_path("scan-upsert");
        let db = Database::new(&db_path).expect("db should open");

        let mut original = sample_item("File Name", r"C:\media\movie.mkv");
        db.add_media_item_data(&original)
            .expect("initial insert should succeed");
        db.conn
            .execute(
                "UPDATE media_items SET watched = 1, favorite = 1 WHERE file_path = ?1",
                params![&original.file_path],
            )
            .expect("should update flags");

        original.title = "Embedded Title".to_string();
        original.file_size = Some(200);
        let inserted = db
            .upsert_scanned_media_item_data(&original)
            .expect("scan upsert should succeed");

        assert!(
            !inserted,
            "existing rows should be refreshed, not counted as new"
        );

        let row = db
            .conn
            .query_row(
                "SELECT title, file_size, watched, favorite FROM media_items WHERE file_path = ?1",
                params![&original.file_path],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, bool>(2)?,
                        row.get::<_, bool>(3)?,
                    ))
                },
            )
            .expect("item should still exist");

        assert_eq!(row.0, "Embedded Title");
        assert_eq!(row.1, Some(200));
        assert!(row.2, "watched state should be preserved");
        assert!(row.3, "favorite state should be preserved");

        drop(db);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn scan_upsert_fills_missing_poster_for_existing_items() {
        let db_path = test_db_path("scan-upsert-poster");
        let db = Database::new(&db_path).expect("db should open");

        let mut original = sample_item("Movie", r"C:\media\movie.mkv");
        db.add_media_item_data(&original)
            .expect("initial insert should succeed");

        original.poster_path = Some(r"C:\media\movie-poster.jpg".to_string());
        let inserted = db
            .upsert_scanned_media_item_data(&original)
            .expect("scan upsert should succeed");

        assert!(!inserted, "existing rows should be refreshed, not inserted");

        let poster_path: Option<String> = db
            .conn
            .query_row(
                "SELECT poster_path FROM media_items WHERE file_path = ?1",
                params![&original.file_path],
                |row| row.get(0),
            )
            .expect("item should still exist");
        assert_eq!(poster_path, Some(r"C:\media\movie-poster.jpg".to_string()));

        drop(db);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn get_media_items_without_limit_returns_all_rows() {
        let db_path = test_db_path("get-all-media");
        let db = Database::new(&db_path).expect("db should open");

        for idx in 0..250 {
            let title = format!("Item {idx}");
            let path = format!(r"C:\media\item-{idx}.mkv");
            db.add_media_item_data(&sample_item(&title, &path))
                .expect("insert should succeed");
        }

        let all_items = db
            .get_media_items_data(None, None, Some(0))
            .expect("query should succeed");
        assert_eq!(all_items.len(), 250);

        drop(db);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn prefer_embedded_titles_defaults_to_true() {
        let db_path = test_db_path("embedded-title-default");
        let db = Database::new(&db_path).expect("db should open");

        assert_eq!(
            db.get_setting_data("prefer_embedded_titles")
                .expect("setting should load"),
            Some("true".to_string())
        );

        drop(db);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn search_media_is_not_capped_at_200() {
        let db_path = test_db_path("search-not-capped");
        let db = Database::new(&db_path).expect("db should open");

        for idx in 0..230 {
            let title = format!("Match {idx}");
            let path = format!(r"C:\media\match-{idx}.mkv");
            db.add_media_item_data(&sample_item(&title, &path))
                .expect("insert should succeed");
        }

        let matches = db
            .search_media_data("Match")
            .expect("search should succeed");
        assert_eq!(matches.len(), 230);

        drop(db);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn enrichment_update_preserves_user_flags() {
        let db_path = test_db_path("enrichment-update");
        let db = Database::new(&db_path).expect("db should open");

        let item = sample_item("Old Title", r"C:\media\old-title.mp4");
        db.add_media_item_data(&item)
            .expect("insert should succeed");
        db.conn
            .execute(
                "UPDATE media_items SET watched = 1, favorite = 1 WHERE file_path = ?1",
                params![&item.file_path],
            )
            .expect("flag update should succeed");

        db.update_media_metadata_data(
            &item.file_path,
            Some("Better Title"),
            Some("Overview text"),
            Some("https://poster"),
            Some(2024),
            Some(8.1),
            Some("Drama"),
            Some("123"),
            Some("tt123"),
            Some("adult"),
        )
        .expect("metadata update should succeed");

        let row = db.conn.query_row(
            "SELECT title, overview, watched, favorite, media_type FROM media_items WHERE file_path = ?1",
            params![&item.file_path],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, String>(4)?,
            )),
        ).expect("row should exist");

        assert_eq!(row.0, "Better Title");
        assert_eq!(row.1.as_deref(), Some("Overview text"));
        assert!(row.2);
        assert!(row.3);
        assert_eq!(row.4, "adult");

        drop(db);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn cleanup_removes_all_photo_rows_leaving_only_video_items() {
        let db_path = test_db_path("sidecar-artwork-cleanup");
        let media_dir =
            std::env::temp_dir().join(format!("cinavault-sidecar-db-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&media_dir).expect("media dir should be created");
        let video_path = media_dir.join("Movie.mp4");
        let poster_path = media_dir.join("Movie-poster.jpg");
        fs::write(&video_path, b"video").expect("video should exist");
        fs::write(&poster_path, b"poster").expect("poster should exist");
        let db = Database::new(&db_path).expect("db should open");
        let video = sample_item("Movie", &video_path.to_string_lossy());
        let mut poster = sample_item("poster", &poster_path.to_string_lossy());
        poster.media_type = "photo".to_string();
        let mut backdrop = sample_item("scene-poster", r"E:\Videos\Movie\scene-poster.webp");
        backdrop.media_type = "photo".to_string();
        let mut real_photo = sample_item("beach-day", r"E:\Photos\Vacation\beach-day.jpg");
        real_photo.media_type = "photo".to_string();
        db.add_media_item_data(&video)
            .expect("video row should insert");
        db.add_media_item_data(&poster)
            .expect("poster row should insert");
        db.add_media_item_data(&backdrop)
            .expect("poster suffix row should insert");
        db.add_media_item_data(&real_photo)
            .expect("real photo row should insert");
        db.cleanup_non_library_photo_artifacts()
            .expect("cleanup should succeed");
        // ALL photo rows are removed — only the video item remains
        let remaining = db
            .conn
            .query_row("SELECT COUNT(*) FROM media_items", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count should load");
        assert_eq!(
            remaining, 1,
            "only the video item should remain after cleanup"
        );
        let video_exists = db
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM media_items WHERE file_path = ?1)",
                params![video.file_path],
                |row| row.get::<_, bool>(0),
            )
            .expect("video row lookup should load");
        assert!(video_exists, "video item should survive cleanup");
        // All photo rows (poster, backdrop, real_photo) must be gone
        let photo_count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM media_items WHERE media_type = 'photo'",
                [],
                |row| row.get(0),
            )
            .expect("photo count should load");
        assert_eq!(photo_count, 0, "all photo rows should be removed");
        drop(db);
        let _ = fs::remove_file(db_path);
        let _ = fs::remove_dir_all(media_dir);
    }

    #[test]
    fn manual_sidecar_artwork_backfill_populates_video_posters() {
        let db_path = test_db_path("manual-sidecar-backfill");
        let media_dir = std::env::temp_dir().join(format!(
            "cinavault-manual-backfill-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&media_dir).expect("media dir should be created");
        let video_path = media_dir.join("Movie.mp4");
        let poster_path = media_dir.join("Movie-poster.jpg");
        fs::write(&video_path, b"video").expect("video should exist");
        fs::write(&poster_path, b"poster").expect("poster should exist");

        let db = Database::new(&db_path).expect("db should open");
        let video = sample_item("Movie", &video_path.to_string_lossy());
        db.add_media_item_data(&video)
            .expect("video row should insert");

        db.sync_sidecar_artwork_for_video_rows()
            .expect("manual backfill should succeed");

        let attached_poster = db
            .conn
            .query_row(
                "SELECT poster_path FROM media_items WHERE file_path = ?1",
                params![video.file_path],
                |row| row.get::<_, Option<String>>(0),
            )
            .expect("video row should load");
        assert_eq!(
            attached_poster.as_deref(),
            Some(poster_path.to_string_lossy().as_ref())
        );

        drop(db);
        let _ = fs::remove_file(db_path);
        let _ = fs::remove_dir_all(media_dir);
    }

    #[test]
    fn database_startup_does_not_backfill_video_posters_from_filesystem() {
        let db_path = test_db_path("startup-does-not-backfill-posters");
        let media_dir =
            std::env::temp_dir().join(format!("cinavault-startup-db-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&media_dir).expect("media dir should be created");
        let video_path = media_dir.join("Movie.mp4");
        let poster_path = media_dir.join("Movie-poster.jpg");
        fs::write(&video_path, b"video").expect("video should exist");
        fs::write(&poster_path, b"poster").expect("poster should exist");

        {
            let db = Database::new(&db_path).expect("db should open");
            let video = sample_item("Movie", &video_path.to_string_lossy());
            db.add_media_item_data(&video)
                .expect("video row should insert");
        }

        let db = Database::new(&db_path).expect("db should reopen quickly");
        let attached_poster = db
            .conn
            .query_row(
                "SELECT poster_path FROM media_items WHERE file_path = ?1",
                params![video_path.to_string_lossy().as_ref()],
                |row| row.get::<_, Option<String>>(0),
            )
            .expect("video row should load");
        assert_eq!(attached_poster, None);

        drop(db);
        let _ = fs::remove_file(db_path);
        let _ = fs::remove_dir_all(media_dir);
    }

    #[test]
    fn rename_update_changes_file_path_only_after_success() {
        let db_path = test_db_path("rename-update");
        let db = Database::new(&db_path).expect("db should open");

        let item = sample_item("Old Title", r"C:\media\old-title.mp4");
        db.add_media_item_data(&item)
            .expect("insert should succeed");

        db.update_media_file_path_data(&item.file_path, r"C:\media\New Title.mp4", "New Title")
            .expect("rename update should succeed");

        let row = db
            .conn
            .query_row(
                "SELECT title, file_path FROM media_items WHERE file_path = ?1",
                params![r"C:\media\New Title.mp4"],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("renamed row should exist");

        assert_eq!(row.0, "New Title");
        assert_eq!(row.1, r"C:\media\New Title.mp4");

        drop(db);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn remote_access_user_authenticates_with_email_password_or_access_key() {
        let db_path = test_db_path("remote-access-auth");
        let db = Database::new(&db_path).expect("db should open");

        let created = db
            .create_remote_access_user(" Owner@Example.COM ", "CorrectHorse42!", Some("Owner"))
            .expect("remote user should be created");

        assert_eq!(created.email, "owner@example.com");
        assert_eq!(created.display_name.as_deref(), Some("Owner"));
        assert!(created.access_key.starts_with("cvra_"));
        assert_eq!(created.access_key_preview.len(), 8);

        let stored_secret = db
            .conn
            .query_row(
                "SELECT password_hash, access_key_hash FROM remote_access_users WHERE email = ?1",
                params!["owner@example.com"],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("stored secrets should load");
        assert!(!stored_secret.0.contains("CorrectHorse42!"));
        assert!(stored_secret.0.starts_with("$argon2id$"));
        assert!(!stored_secret.1.contains(&created.access_key));

        let password_auth = db
            .authenticate_remote_password("owner@example.com", "CorrectHorse42!")
            .expect("password auth should run")
            .expect("correct password should authenticate");
        assert_eq!(password_auth.email, "owner@example.com");
        assert_eq!(password_auth.auth_method, "password");

        assert!(db
            .authenticate_remote_password("owner@example.com", "wrong-password")
            .expect("wrong password auth should run")
            .is_none());

        let key_auth = db
            .authenticate_remote_access_key(&created.access_key)
            .expect("access-key auth should run")
            .expect("correct key should authenticate");
        assert_eq!(key_auth.email, "owner@example.com");
        assert_eq!(key_auth.auth_method, "access_key");

        // Legacy SHA-256 credentials are upgraded only after a successful password login.
        let legacy_salt = new_secret_salt();
        let legacy_hash = hash_secret(&legacy_salt, "CorrectHorse42!");
        db.conn
            .execute(
                "UPDATE remote_access_users SET password_salt = ?1, password_hash = ?2 WHERE email = ?3",
                params![legacy_salt, legacy_hash, "owner@example.com"],
            )
            .expect("legacy password fixture should save");
        let migrated_auth = db
            .authenticate_remote_password("owner@example.com", "CorrectHorse42!")
            .expect("legacy password authentication should run")
            .expect("legacy password should authenticate once");
        let migrated_hash: String = db
            .conn
            .query_row(
                "SELECT password_hash FROM remote_access_users WHERE email = ?1",
                params!["owner@example.com"],
                |row| row.get(0),
            )
            .expect("migrated hash should load");
        assert!(migrated_hash.starts_with("$argon2id$"));

        assert!(db
            .validate_remote_access_session(&migrated_auth.session_token)
            .expect("session lookup should run")
            .is_some());
        db.conn
            .execute(
                "UPDATE remote_access_sessions SET revoked = 1 WHERE token_lookup = ?1",
                params![hash_session_token_lookup(&migrated_auth.session_token)],
            )
            .expect("session revocation should save");
        assert!(db
            .validate_remote_access_session(&migrated_auth.session_token)
            .expect("revoked session lookup should run")
            .is_none());

        drop(db);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn disabled_remote_access_user_cannot_authenticate() {
        let db_path = test_db_path("remote-access-disabled");
        let db = Database::new(&db_path).expect("db should open");

        let created = db
            .create_remote_access_user("viewer@example.com", "CorrectHorse42!", None)
            .expect("remote user should be created");
        db.conn
            .execute(
                "UPDATE remote_access_users SET enabled = 0 WHERE email = ?1",
                params!["viewer@example.com"],
            )
            .expect("disable should succeed");

        assert!(db
            .authenticate_remote_password("viewer@example.com", "CorrectHorse42!")
            .expect("password auth should run")
            .is_none());
        assert!(db
            .authenticate_remote_access_key(&created.access_key)
            .expect("access-key auth should run")
            .is_none());

        drop(db);
        let _ = fs::remove_file(db_path);
    }
}
