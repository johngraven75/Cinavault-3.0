use crate::db::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const SCHEMA_VERSION: u32 = 2;
const REGISTRY_FILE: &str = "metadata-providers.json";
static CONFIG_ROOT: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataProviderDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub enabled: bool,
    pub requires_key: bool,
    pub implemented: bool,
    pub endpoint: Option<String>,
    pub custom_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataProviderRegistry {
    pub schema_version: u32,
    pub policy: String,
    pub credentials_storage: String,
    pub portable_across_operating_systems: bool,
    pub providers: Vec<MetadataProviderDefinition>,
}

pub fn configure(config_root: PathBuf) {
    let _ = CONFIG_ROOT.set(config_root);
}

fn configured_root() -> Result<PathBuf, String> {
    CONFIG_ROOT
        .get()
        .cloned()
        .ok_or_else(|| "Metadata provider configuration root is not initialized".to_string())
}

fn registry_path() -> Result<PathBuf, String> {
    Ok(configured_root()?.join("config").join(REGISTRY_FILE))
}

fn provider(
    id: &str,
    name: &str,
    category: &str,
    requires_key: bool,
    implemented: bool,
    endpoint: Option<&str>,
) -> MetadataProviderDefinition {
    MetadataProviderDefinition {
        id: id.to_string(),
        name: name.to_string(),
        category: category.to_string(),
        enabled: true,
        requires_key,
        implemented,
        endpoint: endpoint.map(str::to_string),
        custom_endpoint: None,
    }
}

pub fn default_registry() -> MetadataProviderRegistry {
    MetadataProviderRegistry {
        schema_version: SCHEMA_VERSION,
        policy: "all_providers_enabled".to_string(),
        credentials_storage: "native_credentials_store_not_portable_json".to_string(),
        portable_across_operating_systems: true,
        providers: vec![
            provider(
                "tmdb",
                "TMDb",
                "Movies & TV",
                true,
                true,
                Some("https://api.themoviedb.org/3"),
            ),
            provider(
                "omdb",
                "OMDb",
                "Movies & TV",
                true,
                true,
                Some("https://www.omdbapi.com"),
            ),
            provider(
                "tvdb",
                "TVDB",
                "Movies & TV",
                true,
                false,
                Some("https://api4.thetvdb.com/v4"),
            ),
            provider(
                "trakt",
                "Trakt",
                "Movies & TV",
                true,
                false,
                Some("https://api.trakt.tv"),
            ),
            provider(
                "imdb",
                "IMDb",
                "Movies & TV",
                false,
                false,
                Some("https://www.imdb.com"),
            ),
            provider(
                "rotten_tomatoes",
                "Rotten Tomatoes",
                "Movies & TV",
                false,
                false,
                Some("https://www.rottentomatoes.com"),
            ),
            provider(
                "cinemeta",
                "CINEMETA",
                "Movies & TV",
                false,
                true,
                Some("https://v3-cinemeta.strem.io"),
            ),
            provider(
                "tvmaze",
                "TVMaze",
                "Movies & TV",
                false,
                true,
                Some("https://api.tvmaze.com"),
            ),
            provider(
                "musicbrainz",
                "MusicBrainz",
                "Music",
                false,
                true,
                Some("https://musicbrainz.org/ws/2"),
            ),
            provider(
                "audiodb",
                "AudioDB",
                "Music",
                true,
                false,
                Some("https://theaudiodb.com/api/v1/json"),
            ),
            provider(
                "lastfm",
                "Last.fm",
                "Music",
                true,
                false,
                Some("https://ws.audioscrobbler.com/2.0"),
            ),
            provider(
                "discogs",
                "Discogs",
                "Music",
                true,
                false,
                Some("https://api.discogs.com"),
            ),
            provider(
                "anidb",
                "AniDB",
                "Anime",
                true,
                false,
                Some("https://api.anidb.net:9001/httpapi"),
            ),
            provider(
                "anilist",
                "AniList",
                "Anime",
                false,
                false,
                Some("https://graphql.anilist.co"),
            ),
            provider(
                "myanimelist",
                "MyAnimeList",
                "Anime",
                true,
                false,
                Some("https://api.myanimelist.net/v2"),
            ),
            provider(
                "kitsu",
                "Kitsu",
                "Anime",
                false,
                false,
                Some("https://kitsu.io/api/edge"),
            ),
            provider(
                "fanarttv",
                "Fanart.tv",
                "Artwork",
                true,
                false,
                Some("https://webservice.fanart.tv/v3"),
            ),
            provider(
                "tmdb_images",
                "TheMovieDB Images",
                "Artwork",
                true,
                true,
                Some("https://image.tmdb.org/t/p"),
            ),
            provider(
                "pgma",
                "PGMA Modernized",
                "Adult",
                false,
                true,
                Some("cinavault://pgma-bridge"),
            ),
            provider(
                "porn_site_nuxt",
                "Porn Site Nuxt",
                "Adult",
                false,
                true,
                None,
            ),
            provider(
                "tpdb",
                "ThePornDB",
                "Adult",
                true,
                true,
                Some("https://api.theporndb.net"),
            ),
            provider(
                "stashdb",
                "StashDB",
                "Adult",
                true,
                true,
                Some("https://stashdb.org/graphql"),
            ),
            provider("phoenixadult", "PhoenixAdult", "Adult", false, true, None),
            provider(
                "iafd",
                "IAFD",
                "Adult",
                false,
                false,
                Some("https://www.iafd.com"),
            ),
            provider(
                "opensubtitles",
                "OpenSubtitles",
                "Subtitles",
                true,
                false,
                Some("https://api.opensubtitles.com/api/v1"),
            ),
            provider(
                "subscene",
                "Subscene",
                "Subtitles",
                false,
                false,
                Some("https://subscene.com"),
            ),
            provider(
                "igdb",
                "IGDB",
                "Other",
                true,
                false,
                Some("https://api.igdb.com/v4"),
            ),
            provider(
                "openlibrary",
                "OpenLibrary",
                "Other",
                false,
                false,
                Some("https://openlibrary.org"),
            ),
            provider(
                "goodreads",
                "GoodReads",
                "Other",
                true,
                false,
                Some("https://www.goodreads.com"),
            ),
            provider("epg_guide", "EPG Guide", "Other", false, false, None),
            provider("plex_agents", "MS-A Agents", "Agents", false, false, None),
            provider(
                "emby_providers",
                "MS-B Providers",
                "Agents",
                false,
                false,
                None,
            ),
            provider(
                "jellyfin_providers",
                "MS-C Providers",
                "Agents",
                false,
                false,
                None,
            ),
        ],
    }
}

fn merge_existing(
    mut defaults: MetadataProviderRegistry,
    current: Option<MetadataProviderRegistry>,
) -> MetadataProviderRegistry {
    let current_by_id: HashMap<String, MetadataProviderDefinition> = current
        .map(|registry| {
            registry
                .providers
                .into_iter()
                .map(|provider| (provider.id.clone(), provider))
                .collect()
        })
        .unwrap_or_default();

    for provider in &mut defaults.providers {
        if let Some(existing) = current_by_id.get(&provider.id) {
            provider.custom_endpoint = existing
                .custom_endpoint
                .clone()
                .filter(|value| !value.trim().is_empty());
        }
        provider.enabled = true;
    }
    defaults
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Metadata provider registry has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    fs::rename(&temporary, path).map_err(|error| error.to_string())
}

fn ui_provider_json(registry: &MetadataProviderRegistry) -> Result<String, String> {
    let providers = registry
        .providers
        .iter()
        .map(|provider| {
            serde_json::json!({
                "id": provider.id,
                "name": provider.name,
                "category": provider.category,
                "enabled": true
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&providers).map_err(|error| error.to_string())
}

pub fn ensure_registry(database: &Database) -> Result<MetadataProviderRegistry, String> {
    let path = registry_path()?;
    let current = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<MetadataProviderRegistry>(&text).ok())
    } else {
        None
    };
    let registry = merge_existing(default_registry(), current);
    let bytes = serde_json::to_vec_pretty(&registry).map_err(|error| error.to_string())?;
    atomic_write(&path, &bytes)?;

    let providers = ui_provider_json(&registry)?;
    database
        .conn
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params!["_metadataProviders", providers],
        )
        .map_err(|error| error.to_string())?;
    for (key, value) in [
        ("metadata_provider_policy", "all_providers_enabled"),
        ("metadata_provider_schema", "2"),
        ("metadata_provider_portability", "cross_os_registry"),
        ("metadata_provider_credentials", "native_secure_store"),
    ] {
        database
            .conn
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(registry)
}

pub fn public_registry() -> Result<MetadataProviderRegistry, String> {
    let path = registry_path()?;
    if path.exists() {
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        if let Ok(registry) = serde_json::from_str::<MetadataProviderRegistry>(&text) {
            return Ok(merge_existing(default_registry(), Some(registry)));
        }
    }
    Ok(default_registry())
}

#[tauri::command]
pub fn get_metadata_provider_registry() -> Result<MetadataProviderRegistry, String> {
    public_registry()
}

#[tauri::command]
pub fn ensure_metadata_provider_registry(
    state: tauri::State<'_, crate::AppState>,
) -> Result<MetadataProviderRegistry, String> {
    let database = state.db.lock().map_err(|error| error.to_string())?;
    ensure_registry(&database)
}

#[cfg(test)]
mod tests {
    use super::default_registry;

    #[test]
    fn every_provider_is_enabled_by_default() {
        let registry = default_registry();
        assert!(registry.providers.len() >= 30);
        assert!(registry.providers.iter().all(|provider| provider.enabled));
    }

    #[test]
    fn portable_registry_never_contains_credentials() {
        let json = serde_json::to_string(&default_registry()).expect("registry serializes");
        assert!(!json.contains("apiKey"));
        assert!(!json.contains("accessToken"));
        assert!(!json.contains("secret"));
    }
}
