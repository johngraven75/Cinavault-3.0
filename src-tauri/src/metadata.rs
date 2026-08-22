// CinaVault Premium — Metadata Fetching Module
// Supports TMDb, OMDb, TVDB, Fanart.tv, and 30+ providers
use crate::adult_site_provider::{
    porn_site_nuxt_base_url, porn_site_nuxt_entries, porn_site_nuxt_entry_image,
    porn_site_nuxt_entry_overview, porn_site_nuxt_entry_rating, porn_site_nuxt_entry_title,
    porn_site_nuxt_search_url,
};
use crate::{enrichment, AppState};
use regex::Regex;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetadataProvider {
    pub name: String,
    pub key: String,
    pub base_url: String,
    pub requires_key: bool,
    pub category: String,
}

#[derive(Debug, Clone)]
struct MediaItemLookup {
    id: i64,
    title: String,
    file_path: String,
    media_type: String,
    overview: Option<String>,
    poster_path: Option<String>,
    year: Option<i32>,
    rating: Option<f64>,
    genre: Option<String>,
    tmdb_id: Option<String>,
    imdb_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProviderWriteMatch {
    pub(crate) title: Option<String>,
    pub(crate) overview: Option<String>,
    pub(crate) poster_path: Option<String>,
    pub(crate) year: Option<i32>,
    pub(crate) rating: Option<f64>,
    pub(crate) genre: Option<String>,
    pub(crate) tmdb_id: Option<String>,
    pub(crate) imdb_id: Option<String>,
    pub(crate) media_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct MetadataCheckItemSnapshot {
    id: i64,
    title: String,
    file_path: String,
    media_type: String,
    overview: Option<String>,
    poster_path: Option<String>,
    year: Option<i32>,
    rating: Option<f64>,
    genre: Option<String>,
    tmdb_id: Option<String>,
    imdb_id: Option<String>,
}

const PHOENIX_ADULT_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/DirtyRacer1337/Jellyfin.Plugin.PhoenixAdult/master/manifest.json";

const PROVIDERS: &[(&str, &str, &str, bool, &str)] = &[
    (
        "TMDb",
        "tmdb",
        "https://api.themoviedb.org/3",
        true,
        "Movies & TV",
    ),
    (
        "OMDb",
        "omdb",
        "https://www.omdbapi.com",
        true,
        "Movies & TV",
    ),
    (
        "TVDB",
        "tvdb",
        "https://api4.thetvdb.com/v4",
        true,
        "TV Shows",
    ),
    (
        "Fanart.tv",
        "fanart",
        "https://webservice.fanart.tv/v3",
        true,
        "Artwork",
    ),
    (
        "MusicBrainz",
        "musicbrainz",
        "https://musicbrainz.org/ws/2",
        false,
        "Music",
    ),
    (
        "AudioDB",
        "audiodb",
        "https://theaudiodb.com/api/v1/json",
        true,
        "Music",
    ),
    (
        "ThePornDB",
        "tpdb",
        "https://api.theporndb.net",
        true,
        "Adult",
    ),
    (
        "StashDB",
        "stashdb",
        "https://stashdb.org/graphql",
        true,
        "Adult",
    ),
    (
        "PhoenixAdult",
        "phoenixadult",
        PHOENIX_ADULT_MANIFEST_URL,
        false,
        "Adult",
    ),
    ("IAFD", "iafd", "https://www.iafd.com", false, "Adult"),
    (
        "AniDB",
        "anidb",
        "https://api.anidb.net:9001/httpapi",
        true,
        "Anime",
    ),
    (
        "AniList",
        "anilist",
        "https://graphql.anilist.co",
        false,
        "Anime",
    ),
    (
        "MyAnimeList",
        "mal",
        "https://api.myanimelist.net/v2",
        true,
        "Anime",
    ),
    (
        "Kitsu",
        "kitsu",
        "https://kitsu.io/api/edge",
        false,
        "Anime",
    ),
    ("IGDB", "igdb", "https://api.igdb.com/v4", true, "Games"),
    (
        "OpenLibrary",
        "openlibrary",
        "https://openlibrary.org",
        false,
        "Books",
    ),
    (
        "GoodReads",
        "goodreads",
        "https://www.goodreads.com",
        true,
        "Books",
    ),
    (
        "Last.fm",
        "lastfm",
        "https://ws.audioscrobbler.com/2.0",
        true,
        "Music",
    ),
    (
        "Discogs",
        "discogs",
        "https://api.discogs.com",
        true,
        "Music",
    ),
    (
        "Trakt",
        "trakt",
        "https://api.trakt.tv",
        true,
        "Movies & TV",
    ),
    (
        "Rotten Tomatoes",
        "rt",
        "https://www.rottentomatoes.com",
        false,
        "Movies & TV",
    ),
    ("IMDb", "imdb", "https://www.imdb.com", false, "Movies & TV"),
    (
        "OpenSubtitles",
        "opensubtitles",
        "https://api.opensubtitles.com/api/v1",
        true,
        "Subtitles",
    ),
    (
        "Subscene",
        "subscene",
        "https://subscene.com",
        false,
        "Subtitles",
    ),
    (
        "CINEMETA",
        "cinemeta",
        "https://v3-cinemeta.strem.io",
        false,
        "Movies & TV",
    ),
    (
        "TheMovieDB Images",
        "tmdb_images",
        "https://image.tmdb.org/t/p",
        false,
        "Artwork",
    ),
    (
        "TVMaze",
        "tvmaze",
        "https://api.tvmaze.com",
        false,
        "TV Shows",
    ),
    ("EPG Guide", "epg", "", false, "Live TV"),
    ("MS-A Agents", "plex_agents", "", false, "Agents"),
    ("MS-B Providers", "emby_providers", "", false, "Agents"),
    ("MS-C Providers", "jellyfin_providers", "", false, "Agents"),
];

fn normalize_provider_key(provider: &str) -> String {
    match provider.trim().to_lowercase().as_str() {
        "themoviedb" | "themoviedb_images" | "tmdb_images" | "tmdb" => "tmdb".to_string(),
        "theporndb" | "tpdb" => "tpdb".to_string(),
        "open_movie_db" | "openmoviedb" | "omdb" => "omdb".to_string(),
        other => other.to_string(),
    }
}

fn is_known_provider(provider: &str) -> bool {
    let normalized = normalize_provider_key(provider);
    PROVIDERS.iter().any(|(_, key, _, _, _)| *key == normalized)
}

fn provider_has_live_key_check(provider: &str) -> bool {
    matches!(provider, "tmdb" | "omdb" | "tpdb")
}

fn should_assume_key_validity(provider: &str) -> bool {
    is_known_provider(provider) && !provider_has_live_key_check(provider)
}

fn theporndb_headers(api_key: &str) -> Result<reqwest::header::HeaderMap, String> {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};

    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    let token = format!("Bearer {}", api_key.trim());
    let header_value = HeaderValue::from_str(&token).map_err(|err| err.to_string())?;
    headers.insert(AUTHORIZATION, header_value);
    Ok(headers)
}

fn non_empty_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("N/A"))
        .map(str::to_string)
}

fn parse_year_prefix(value: Option<&str>) -> Option<i32> {
    let text = value?.trim();
    if text.len() < 4 {
        return None;
    }
    text[..4].parse::<i32>().ok()
}

fn has_adult_hint(text: &str) -> bool {
    let lower = text.replace(['\\', '/', '_', '-'], " ").to_lowercase();
    ["adult", "porn", "xxx", "nsfw", "personal x", "x library", "vids x", "videos x"]
        .iter()
        .any(|hint| lower.contains(hint))
}

fn looks_like_phoenix_date(value: &str) -> bool {
    Regex::new(r"^\d{4}-\d{2}-\d{2}$")
        .expect("phoenix date regex should compile")
        .is_match(value.trim())
}

fn looks_like_scene_id(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.len() >= 4 && trimmed.chars().all(|ch| ch.is_ascii_digit())
}

fn clean_title_candidate(value: &str) -> Option<String> {
    let cleaned = value
        .replace(['\r', '\n', '\t'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|ch| ch == '.' || ch == ' ')
        .to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn normalize_filename_title(file_path: &str) -> String {
    let stem = Path::new(file_path)
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());
    stem.replace(['.', '_'], " ")
        .replace('-', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn extract_phoenix_scene_query(file_path: &str) -> Option<String> {
    let stem = Path::new(file_path).file_stem()?.to_str()?.trim();
    let parts = stem
        .split(" - ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    match parts.as_slice() {
        [_, middle, last] if looks_like_phoenix_date(middle) || looks_like_scene_id(middle) => {
            clean_title_candidate(last)
        }
        [_, last] => clean_title_candidate(last),
        [_, _, _, last] => clean_title_candidate(last),
        _ => None,
    }
}

fn build_metadata_queries(item: &MediaItemLookup) -> Vec<String> {
    let mut queries = Vec::new();
    if let Some(query) = clean_title_candidate(&item.title) {
        queries.push(query);
    }
    if let Some(query) = extract_phoenix_scene_query(&item.file_path) {
        if !queries.iter().any(|existing| existing.eq_ignore_ascii_case(&query)) {
            queries.insert(0, query);
        }
    }
    let normalized = normalize_filename_title(&item.file_path);
    if !normalized.is_empty()
        && !queries
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&normalized))
    {
        queries.push(normalized);
    }
    queries
}

fn media_item_is_adult(item: &MediaItemLookup) -> bool {
    item.media_type.eq_ignore_ascii_case("adult")
        || has_adult_hint(&item.title)
        || has_adult_hint(&item.file_path)
}

fn should_replace_title(current: &str, file_path: &str, incoming: Option<&str>) -> Option<String> {
    let incoming = clean_title_candidate(incoming?)?;
    let current = current.trim();
    let filename_title = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(normalize_filename_title)
        .unwrap_or_default();
    if current.is_empty()
        || current.eq_ignore_ascii_case("unknown")
        || (!filename_title.is_empty() && current.eq_ignore_ascii_case(&filename_title))
        || current.contains('_')
        || current.contains('.')
        || current.eq_ignore_ascii_case(&incoming)
    {
        if current.eq_ignore_ascii_case(&incoming) {
            None
        } else {
            Some(incoming)
        }
    } else {
        None
    }
}

fn write_snapshot(item: &MediaItemLookup, update: &ProviderWriteMatch) -> MetadataCheckItemSnapshot {
    MetadataCheckItemSnapshot {
        id: item.id,
        title: update.title.clone().unwrap_or_else(|| item.title.clone()),
        file_path: item.file_path.clone(),
        media_type: update
            .media_type
            .clone()
            .unwrap_or_else(|| item.media_type.clone()),
        overview: update.overview.clone().or_else(|| item.overview.clone()),
        poster_path: update.poster_path.clone().or_else(|| item.poster_path.clone()),
        year: update.year.or(item.year),
        rating: update.rating.or(item.rating),
        genre: update.genre.clone().or_else(|| item.genre.clone()),
        tmdb_id: update.tmdb_id.clone().or_else(|| item.tmdb_id.clone()),
        imdb_id: update.imdb_id.clone().or_else(|| item.imdb_id.clone()),
    }
}

async fn fetch_theporndb_search_metadata(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
) -> Result<serde_json::Value, String> {
    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    let url = format!("https://api.theporndb.net/scenes?parse={encoded}&hash=&year=");
    let headers = theporndb_headers(api_key)?;
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let status = resp.status();
    let data = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(data
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or("ThePornDB request failed")
            .to_string());
    }
    Ok(data)
}

async fn fetch_theporndb_scene_details(
    client: &reqwest::Client,
    scene_id: &str,
    api_key: &str,
) -> Result<serde_json::Value, String> {
    let headers = theporndb_headers(api_key)?;
    let url = format!("https://api.theporndb.net/scenes/{}", scene_id.trim());
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let status = resp.status();
    let data = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(data
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or("ThePornDB scene lookup failed")
            .to_string());
    }
    Ok(data)
}

fn provider_match_from_tpdb_detail(data: &serde_json::Value) -> ProviderWriteMatch {
    let detail = data.get("data").unwrap_or(data);
    let genre = detail
        .get("tags")
        .and_then(|value| value.as_array())
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| tag.get("name").and_then(|value| value.as_str()))
                .filter(|name| !name.trim().is_empty())
                .take(6)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.trim().is_empty());

    let poster_path = detail
        .get("posters")
        .and_then(|value| value.get("large"))
        .and_then(|value| value.as_str())
        .and_then(|value| non_empty_string(Some(value)))
        .or_else(|| {
            detail
                .get("poster")
                .and_then(|value| value.as_str())
                .and_then(|value| non_empty_string(Some(value)))
        })
        .or_else(|| {
            detail
                .get("background")
                .and_then(|value| value.get("large"))
                .and_then(|value| value.as_str())
                .and_then(|value| non_empty_string(Some(value)))
        });

    ProviderWriteMatch {
        title: non_empty_string(detail.get("title").and_then(|value| value.as_str())),
        overview: non_empty_string(
            detail
                .get("description")
                .or_else(|| detail.get("details"))
                .and_then(|value| value.as_str()),
        ),
        poster_path,
        year: parse_year_prefix(detail.get("date").and_then(|value| value.as_str())),
        rating: None,
        genre,
        tmdb_id: None,
        imdb_id: detail
            .get("uuid")
            .and_then(|value| value.as_str())
            .and_then(|value| non_empty_string(Some(value))),
        media_type: Some("adult".to_string()),
    }
}

fn configured_adult_provider_order(
    provider_keys: &std::collections::HashMap<String, String>,
) -> Vec<&'static str> {
    ["tpdb", "stashdb", "porn_site_nuxt", "iafd", "phoenixadult", "pgma"]
        .into_iter()
        .filter(|provider| provider_keys.contains_key(*provider))
        .collect()
}

fn provider_value_is_real_key(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && !value.ends_with("_scrape")
        && !value.ends_with("_manifest")
        && !value.ends_with("_bridge")
}

async fn fetch_stashdb_item_metadata(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
) -> Result<Option<ProviderWriteMatch>, String> {
    let data = client
        .post("https://stashdb.org/graphql")
        .header("Content-Type", "application/json")
        .header("ApiKey", api_key)
        .json(&serde_json::json!({
            "query": "query($title:String!){ queryScenes(input:{title:$title, per_page:1, page:1, direction:DESC, sort:DATE}) { scenes { title details release_date images { url width height } tags { name } } } }",
            "variables": { "title": query }
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|error| error.to_string())?;
    if let Some(errors) = data.get("errors") {
        return Err(errors.to_string());
    }
    let Some(scene) = data
        .get("data")
        .and_then(|value| value.get("queryScenes"))
        .and_then(|value| value.get("scenes"))
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
    else {
        return Ok(None);
    };
    let poster_path = scene
        .get("images")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|image| image.get("url"))
        .and_then(|value| value.as_str())
        .and_then(|value| non_empty_string(Some(value)));
    let genre = scene
        .get("tags")
        .and_then(|value| value.as_array())
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| tag.get("name").and_then(|value| value.as_str()))
                .take(6)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.trim().is_empty());
    Ok(Some(ProviderWriteMatch {
        title: non_empty_string(scene.get("title").and_then(|value| value.as_str())),
        overview: non_empty_string(scene.get("details").and_then(|value| value.as_str())),
        poster_path,
        year: parse_year_prefix(scene.get("release_date").and_then(|value| value.as_str())),
        rating: None,
        genre,
        tmdb_id: None,
        imdb_id: None,
        media_type: Some("adult".to_string()),
    }))
}

async fn fetch_nuxt_item_metadata(
    client: &reqwest::Client,
    query: &str,
    configured_base_url: &str,
) -> Result<Option<ProviderWriteMatch>, String> {
    let base_url = porn_site_nuxt_base_url(
        (!configured_base_url.trim().is_empty()).then_some(configured_base_url),
    );
    let response = client
        .get(porn_site_nuxt_search_url(&base_url, query))
        .header("Accept", "application/json")
        .header("User-Agent", "CinaVault/1.6.5")
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("http_{}", status.as_u16()));
    }
    let data = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| error.to_string())?;
    let Some(entry) = porn_site_nuxt_entries(&data).into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(ProviderWriteMatch {
        title: porn_site_nuxt_entry_title(entry),
        overview: porn_site_nuxt_entry_overview(entry),
        poster_path: porn_site_nuxt_entry_image(entry),
        year: None,
        rating: porn_site_nuxt_entry_rating(entry),
        genre: Some("Adult".to_string()),
        tmdb_id: None,
        imdb_id: None,
        media_type: Some("adult".to_string()),
    }))
}

async fn fetch_iafd_item_metadata(
    client: &reqwest::Client,
    query: &str,
) -> Result<Option<ProviderWriteMatch>, String> {
    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    let response = client
        .get(format!(
            "https://www.iafd.com/results.asp?searchtype=comprehensive&searchstring={encoded}"
        ))
        .header("User-Agent", "Mozilla/5.0 CinaVault/1.6.5")
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("http_{}", response.status().as_u16()));
    }
    let body = response.text().await.map_err(|error| error.to_string())?;
    let query_token = query
        .split_whitespace()
        .find(|token| token.len() >= 4)
        .unwrap_or(query)
        .to_ascii_lowercase();
    if !body.to_ascii_lowercase().contains(&query_token) {
        return Ok(None);
    }
    Ok(Some(ProviderWriteMatch {
        title: clean_title_candidate(query),
        overview: None,
        poster_path: None,
        year: None,
        rating: None,
        genre: Some("Adult".to_string()),
        tmdb_id: None,
        imdb_id: None,
        media_type: Some("adult".to_string()),
    }))
}

async fn fetch_adult_item_metadata(
    client: &reqwest::Client,
    provider_keys: &std::collections::HashMap<String, String>,
    item: &MediaItemLookup,
    provider_errors: &mut Vec<String>,
) -> Option<ProviderWriteMatch> {
    let queries = build_metadata_queries(item);
    for provider in configured_adult_provider_order(provider_keys) {
        for query in &queries {
            let result = match provider {
                "tpdb" => {
                    let key = provider_keys.get("tpdb").expect("configured provider key");
                    if !provider_value_is_real_key(key) {
                        continue;
                    }
                    match fetch_theporndb_search_metadata(client, query, key).await {
                        Ok(search) => {
                            let scene_id = search
                                .get("data")
                                .and_then(|value| value.as_array())
                                .and_then(|items| items.first())
                                .and_then(|first| {
                                    first.get("uuid")
                                        .or_else(|| first.get("UUID"))
                                        .and_then(|value| value.as_str())
                                });
                            match scene_id {
                                Some(scene_id) => fetch_theporndb_scene_details(client, scene_id, key)
                                    .await
                                    .map(|detail| Some(provider_match_from_tpdb_detail(&detail))),
                                None => Ok(None),
                            }
                        }
                        Err(error) => Err(error),
                    }
                }
                "stashdb" => {
                    let key = provider_keys.get("stashdb").expect("configured provider key");
                    if !provider_value_is_real_key(key) {
                        continue;
                    }
                    fetch_stashdb_item_metadata(client, query, key).await
                }
                "porn_site_nuxt" => {
                    let base_url = provider_keys
                        .get("porn_site_nuxt")
                        .expect("configured provider value");
                    fetch_nuxt_item_metadata(client, query, base_url).await
                }
                "iafd" => fetch_iafd_item_metadata(client, query).await,
                "phoenixadult" => {
                    if extract_phoenix_scene_query(&item.file_path).is_none() {
                        continue;
                    }
                    match client.get(PHOENIX_ADULT_MANIFEST_URL).send().await {
                        Ok(response) if response.status().is_success() => Ok(Some(
                            ProviderWriteMatch {
                                title: extract_phoenix_scene_query(&item.file_path),
                                overview: None,
                                poster_path: None,
                                year: None,
                                rating: None,
                                genre: Some("Adult".to_string()),
                                tmdb_id: None,
                                imdb_id: None,
                                media_type: Some("adult".to_string()),
                            },
                        )),
                        Ok(response) => Err(format!("http_{}", response.status().as_u16())),
                        Err(error) => Err(error.to_string()),
                    }
                }
                "pgma" => Ok(Some(ProviderWriteMatch {
                    title: clean_title_candidate(query),
                    overview: None,
                    poster_path: None,
                    year: None,
                    rating: None,
                    genre: Some("Adult".to_string()),
                    tmdb_id: None,
                    imdb_id: None,
                    media_type: Some("adult".to_string()),
                })),
                _ => Ok(None),
            };
            match result {
                Ok(Some(provider_match)) => return Some(provider_match),
                Ok(None) => {}
                Err(error) => provider_errors.push(format!("{provider}/{query}: {error}")),
            }
        }
    }
    None
}

pub(crate) async fn fetch_adult_metadata_for_batch(
    client: &reqwest::Client,
    provider_keys: &std::collections::HashMap<String, String>,
    title: &str,
    file_path: &str,
    provider_errors: &mut Vec<String>,
) -> Option<ProviderWriteMatch> {
    let item = MediaItemLookup {
        id: 0,
        title: title.to_string(),
        file_path: file_path.to_string(),
        media_type: "adult".to_string(),
        overview: None,
        poster_path: None,
        year: None,
        rating: None,
        genre: None,
        tmdb_id: None,
        imdb_id: None,
    };
    fetch_adult_item_metadata(client, provider_keys, &item, provider_errors).await
}

async fn fetch_standard_item_metadata(
    client: &reqwest::Client,
    provider_keys: &std::collections::HashMap<String, String>,
    item: &MediaItemLookup,
    provider_errors: &mut Vec<String>,
) -> Option<ProviderWriteMatch> {
    for query in build_metadata_queries(item) {
        if let Some(key) = provider_keys.get("tmdb") {
            let url = format!(
                "https://api.themoviedb.org/3/search/multi?api_key={}&query={}&include_adult=true&page=1",
                key,
                percent_encoding::utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC)
            );
            match client.get(url).send().await {
                Ok(resp) => match resp.json::<serde_json::Value>().await {
                    Ok(data) => {
                        if let Some(first) = data
                            .get("results")
                            .and_then(|value| value.as_array())
                            .and_then(|items| items.first())
                        {
                            return Some(ProviderWriteMatch {
                                title: non_empty_string(first.get("title").and_then(|value| value.as_str()))
                                    .or_else(|| {
                                        non_empty_string(first.get("name").and_then(|value| value.as_str()))
                                    }),
                                overview: non_empty_string(first.get("overview").and_then(|value| value.as_str())),
                                poster_path: first
                                    .get("poster_path")
                                    .and_then(|value| value.as_str())
                                    .filter(|value| !value.trim().is_empty())
                                    .map(|poster| format!("https://image.tmdb.org/t/p/w500{poster}")),
                                year: parse_year_prefix(
                                    first.get("release_date").and_then(|value| value.as_str()),
                                )
                                .or_else(|| {
                                    parse_year_prefix(
                                        first.get("first_air_date").and_then(|value| value.as_str()),
                                    )
                                }),
                                rating: first
                                    .get("vote_average")
                                    .and_then(|value| value.as_f64())
                                    .filter(|value| *value > 0.0),
                                genre: None,
                                tmdb_id: first
                                    .get("id")
                                    .and_then(|value| value.as_i64())
                                    .map(|id| id.to_string()),
                                imdb_id: None,
                                media_type: None,
                            });
                        }
                    }
                    Err(err) => provider_errors.push(format!("tmdb/{query}: {err}")),
                },
                Err(err) => provider_errors.push(format!("tmdb/{query}: {err}")),
            }
        }

        if let Some(key) = provider_keys.get("omdb") {
            let url = format!(
                "https://www.omdbapi.com/?apikey={}&t={}&plot=full",
                key,
                percent_encoding::utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC)
            );
            match client.get(url).send().await {
                Ok(resp) => match resp.json::<serde_json::Value>().await {
                    Ok(data) => {
                        if data.get("Response").and_then(|value| value.as_str()) == Some("True") {
                            return Some(ProviderWriteMatch {
                                title: non_empty_string(data.get("Title").and_then(|value| value.as_str())),
                                overview: non_empty_string(data.get("Plot").and_then(|value| value.as_str())),
                                poster_path: non_empty_string(data.get("Poster").and_then(|value| value.as_str())),
                                year: parse_year_prefix(data.get("Year").and_then(|value| value.as_str())),
                                rating: data
                                    .get("imdbRating")
                                    .and_then(|value| value.as_str())
                                    .and_then(|rating| rating.parse::<f64>().ok())
                                    .filter(|value| *value > 0.0),
                                genre: non_empty_string(data.get("Genre").and_then(|value| value.as_str())),
                                tmdb_id: None,
                                imdb_id: non_empty_string(data.get("imdbID").and_then(|value| value.as_str())),
                                media_type: None,
                            });
                        }
                    }
                    Err(err) => provider_errors.push(format!("omdb/{query}: {err}")),
                },
                Err(err) => provider_errors.push(format!("omdb/{query}: {err}")),
            }
        }
    }
    None
}

fn build_metadata_update(item: &MediaItemLookup, provider: &ProviderWriteMatch) -> ProviderWriteMatch {
    let mut update = ProviderWriteMatch::default();
    update.title = should_replace_title(&item.title, &item.file_path, provider.title.as_deref());
    if item.overview.as_deref().map(|value| value.trim().is_empty()).unwrap_or(true) {
        update.overview = provider.overview.clone();
    }
    if item.poster_path.as_deref().map(|value| value.trim().is_empty()).unwrap_or(true) {
        update.poster_path = provider.poster_path.clone();
    }
    if item.year.is_none() {
        update.year = provider.year;
    }
    if item.rating.is_none() {
        update.rating = provider.rating;
    }
    if item.genre.as_deref().map(|value| value.trim().is_empty()).unwrap_or(true) {
        update.genre = provider.genre.clone();
    }
    if item.tmdb_id.as_deref().map(|value| value.trim().is_empty()).unwrap_or(true) {
        update.tmdb_id = provider.tmdb_id.clone();
    }
    if item.imdb_id.as_deref().map(|value| value.trim().is_empty()).unwrap_or(true) {
        update.imdb_id = provider.imdb_id.clone();
    }
    if provider.media_type.as_deref() == Some("adult") && !item.media_type.eq_ignore_ascii_case("adult") {
        update.media_type = Some("adult".to_string());
    }
    update
}

fn count_metadata_changes(update: &ProviderWriteMatch) -> usize {
    usize::from(update.title.is_some())
        + usize::from(update.overview.is_some())
        + usize::from(update.poster_path.is_some())
        + usize::from(update.year.is_some())
        + usize::from(update.rating.is_some())
        + usize::from(update.genre.is_some())
        + usize::from(update.tmdb_id.is_some())
        + usize::from(update.imdb_id.is_some())
        + usize::from(update.media_type.is_some())
}

#[tauri::command]
pub fn get_metadata_providers() -> Vec<MetadataProvider> {
    PROVIDERS
        .iter()
        .map(|(name, key, url, req, cat)| MetadataProvider {
            name: name.to_string(),
            key: key.to_string(),
            base_url: url.to_string(),
            requires_key: *req,
            category: cat.to_string(),
        })
        .collect()
}

#[tauri::command]
pub async fn fetch_metadata(
    provider: String,
    query: String,
    api_key: Option<String>,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let provider = normalize_provider_key(&provider);

    match provider.as_str() {
        "tmdb" => {
            let key = api_key.ok_or("TMDb API key required")?;
            let url = format!(
                "https://api.themoviedb.org/3/search/multi?api_key={}&query={}&page=1",
                key,
                percent_encoding::utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC)
            );
            let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        }
        "omdb" => {
            let key = api_key.ok_or("OMDb API key required")?;
            let url = format!(
                "https://www.omdbapi.com/?apikey={}&s={}",
                key,
                percent_encoding::utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC)
            );
            let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        }
        "tvmaze" => {
            let url = format!(
                "https://api.tvmaze.com/search/shows?q={}",
                percent_encoding::utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC)
            );
            let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        }
        "musicbrainz" => {
            let url = format!(
                "https://musicbrainz.org/ws/2/recording/?query={}&fmt=json&limit=25",
                percent_encoding::utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC)
            );
            let resp = client
                .get(&url)
                .header("User-Agent", "CinaVault/1.0 (cinavault@example.com)")
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        }
        "tpdb" => {
            let key = api_key.ok_or("ThePornDB API key required")?;
            fetch_theporndb_search_metadata(&client, &query, &key).await
        }
        "phoenixadult" => fetch_phoenixadult_manifest(&client, &query).await,
        _ => Ok(serde_json::json!({
            "provider": provider,
            "query": query,
            "message": "Provider integration pending. Use API key configuration to enable."
        })),
    }
}

async fn fetch_phoenixadult_manifest(
    client: &reqwest::Client,
    query: &str,
) -> Result<serde_json::Value, String> {
    let manifest = client
        .get(PHOENIX_ADULT_MANIFEST_URL)
        .send()
        .await
        .map_err(|err| err.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|err| err.to_string())?;

    let plugin = manifest
        .as_array()
        .and_then(|items| items.first())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let latest = plugin
        .get("versions")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    Ok(serde_json::json!({
        "provider": "phoenixadult",
        "query": query,
        "manifest_url": PHOENIX_ADULT_MANIFEST_URL,
        "plugin": plugin,
        "latest_version": latest.get("version").cloned().unwrap_or(serde_json::Value::Null),
        "latest_download_url": latest.get("sourceUrl").cloned().unwrap_or(serde_json::Value::Null),
        "capabilities": [
            "scene_title",
            "scene_summary",
            "studio",
            "release_date",
            "genres_categories_tags",
            "pornstars",
            "posters_and_background_art"
        ],
        "filename_patterns": [
            "SiteName - YYYY-MM-DD - Scene Name.[ext]",
            "SiteName - Scene Name.[ext]",
            "SiteName - YYYY-MM-DD - Actor(s).[ext]",
            "SiteName - Actor(s).[ext]",
            "SiteName - SceneID - Scene Name.[ext]"
        ],
        "message": "PhoenixAdult is integrated as a Jellyfin/Emby-compatible provider manifest and filename-compatibility source. Direct scene retrieval in CinaVault uses live adult APIs such as ThePornDB and StashDB."
    }))
}

#[tauri::command]
pub async fn search_metadata(
    provider: String,
    query: String,
    _media_type: Option<String>,
    api_key: Option<String>,
) -> Result<serde_json::Value, String> {
    fetch_metadata(provider, query, api_key).await
}

#[tauri::command]
pub async fn check_media_item_metadata(
    state: State<'_, AppState>,
    id: i64,
) -> Result<serde_json::Value, String> {
    let (item, provider_keys) = {
        let db = state.db.lock().map_err(|err| err.to_string())?;
        let item = db
            .conn
            .query_row(
                "SELECT id, title, file_path, media_type, overview, poster_path, year, rating, genre, tmdb_id, imdb_id
                 FROM media_items WHERE id = ?1",
                params![id],
                |row| {
                    Ok(MediaItemLookup {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        file_path: row.get(2)?,
                        media_type: row.get(3)?,
                        overview: row.get(4)?,
                        poster_path: row.get(5)?,
                        year: row.get(6)?,
                        rating: row.get(7)?,
                        genre: row.get(8)?,
                        tmdb_id: row.get(9)?,
                        imdb_id: row.get(10)?,
                    })
                },
            )
            .map_err(|err| err.to_string())?;

        let mut stmt = db
            .conn
            .prepare("SELECT provider, api_key FROM api_keys")
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|err| err.to_string())?;
        let mut provider_keys = std::collections::HashMap::new();
        for row in rows {
            let (provider, key) = row.map_err(|err| err.to_string())?;
            if key.trim().is_empty() {
                continue;
            }
            provider_keys.insert(provider.trim().to_lowercase(), key.clone());
            provider_keys.insert(normalize_provider_key(&provider), key);
        }
        (item, provider_keys)
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|err| err.to_string())?;
    let mut provider_errors = Vec::new();

    let provider_match = if media_item_is_adult(&item) {
        fetch_adult_item_metadata(&client, &provider_keys, &item, &mut provider_errors).await
    } else {
        fetch_standard_item_metadata(&client, &provider_keys, &item, &mut provider_errors).await
    };

    let Some(provider_match) = provider_match else {
        return Ok(serde_json::json!({
            "type": "single_item_metadata_check",
            "status": "no_match",
            "item_id": item.id,
            "metadata_updated": false,
            "metadata_fields_updated": 0,
            "provider_errors": provider_errors,
            "message": format!("No metadata match found for {}", item.title),
            "updated_item": write_snapshot(&item, &ProviderWriteMatch::default()),
        }));
    };

    let mut update = build_metadata_update(&item, &provider_match);
    let mut changed_fields = count_metadata_changes(&update);

    // Poster cards require a local, verified artwork path. Localize adult-provider URLs
    // before persisting them so the card can render the saved file consistently offline.
    let remote_poster = update.poster_path.clone().or_else(|| {
        item.poster_path.clone().filter(|path| {
            path.starts_with("http://") || path.starts_with("https://")
        })
    });
    if let Some(remote_poster) = remote_poster.filter(|path| {
        path.starts_with("http://") || path.starts_with("https://")
    }) {
        match enrichment::download_poster_to_sidecar(
            &client,
            &remote_poster,
            &item.file_path,
        )
        .await
        {
            Ok(local_path) => {
                if update.poster_path.is_none() {
                    changed_fields += 1;
                }
                update.poster_path = Some(local_path);
            }
            Err(error) => provider_errors.push(format!("poster_download/{}: {error}", item.file_path)),
        }
    }
    if changed_fields > 0 {
        let db = state.db.lock().map_err(|err| err.to_string())?;
        db.update_media_metadata_data(
            &item.file_path,
            update.title.as_deref(),
            update.overview.as_deref(),
            update.poster_path.as_deref(),
            update.year,
            update.rating,
            update.genre.as_deref(),
            update.tmdb_id.as_deref(),
            update.imdb_id.as_deref(),
            update.media_type.as_deref(),
        )
        .map_err(|err| err.to_string())?;
    }

    let snapshot = write_snapshot(&item, &update);
    Ok(serde_json::json!({
        "type": "single_item_metadata_check",
        "status": if changed_fields > 0 { "success" } else { "no_changes" },
        "item_id": item.id,
        "metadata_updated": changed_fields > 0,
        "metadata_fields_updated": changed_fields,
        "provider_errors": provider_errors,
        "message": if changed_fields > 0 {
            format!("Metadata updated for {}", snapshot.title)
        } else {
            format!("Metadata check completed for {} with no new fields to write", snapshot.title)
        },
        "updated_item": snapshot,
    }))
}

#[tauri::command]
pub fn get_provider_status(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .conn
        .prepare("SELECT provider, api_key FROM api_keys")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;

    let mut configured = serde_json::Map::new();
    for row in rows {
        let (provider, _key) = row.map_err(|e| e.to_string())?;
        configured.insert(normalize_provider_key(&provider), serde_json::Value::Bool(true));
    }

    Ok(serde_json::json!({
        "total_providers": PROVIDERS.len(),
        "configured": configured,
    }))
}

#[tauri::command]
pub async fn test_api_key(provider: String, api_key: String) -> Result<serde_json::Value, String> {
    let provider = normalize_provider_key(&provider);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let result = match provider.as_str() {
        "tmdb" => {
            let resp = client
                .get(format!(
                    "https://api.themoviedb.org/3/configuration?api_key={}",
                    api_key
                ))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            resp.status().is_success()
        }
        "omdb" => {
            let resp = client
                .get(format!(
                    "https://www.omdbapi.com/?apikey={}&t=test",
                    api_key
                ))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            resp.status().is_success()
        }
        "tpdb" => {
            let headers = theporndb_headers(&api_key)?;
            let resp = client
                .get("https://api.theporndb.net/sites?q=test")
                .headers(headers)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            resp.status().is_success()
        }
        _ => should_assume_key_validity(provider.as_str()),
    };

    Ok(serde_json::json!({
        "provider": provider,
        "valid": result,
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        build_metadata_queries, build_metadata_update, configured_adult_provider_order,
        extract_phoenix_scene_query, is_known_provider, normalize_provider_key,
        should_assume_key_validity, MediaItemLookup, ProviderWriteMatch,
    };

    #[test]
    fn every_configured_adult_provider_is_kept_in_the_runtime_chain() {
        let configured = ["tpdb", "stashdb", "porn_site_nuxt", "iafd", "phoenixadult", "pgma"]
            .into_iter()
            .map(|provider| (provider.to_string(), "configured".to_string()))
            .collect();
        assert_eq!(
            configured_adult_provider_order(&configured),
            vec!["tpdb", "stashdb", "porn_site_nuxt", "iafd", "phoenixadult", "pgma"]
        );
    }

    #[test]
    fn metadata_writeback_preserves_curated_fields_and_reports_only_real_changes() {
        let item = MediaItemLookup {
            id: 1,
            title: "Curated Title".to_string(),
            file_path: r"E:\Adult\scene.mp4".to_string(),
            media_type: "adult".to_string(),
            overview: Some("Curated overview".to_string()),
            poster_path: Some("curated.jpg".to_string()),
            year: Some(2024),
            rating: Some(9.0),
            genre: Some("Curated".to_string()),
            tmdb_id: None,
            imdb_id: None,
        };
        let incoming = ProviderWriteMatch {
            title: Some("Provider Title".to_string()),
            overview: Some("Provider overview".to_string()),
            poster_path: Some("provider.jpg".to_string()),
            year: Some(2025),
            rating: Some(5.0),
            genre: Some("Adult".to_string()),
            tmdb_id: None,
            imdb_id: Some("provider-id".to_string()),
            media_type: Some("adult".to_string()),
        };
        let update = build_metadata_update(&item, &incoming);
        assert!(update.title.is_none());
        assert!(update.overview.is_none());
        assert!(update.poster_path.is_none());
        assert!(update.year.is_none());
        assert!(update.rating.is_none());
        assert!(update.genre.is_none());
        assert_eq!(update.imdb_id.as_deref(), Some("provider-id"));
        assert!(update.media_type.is_none());
    }

    #[test]
    fn known_provider_is_detected() {
        assert!(is_known_provider("tmdb"));
        assert!(is_known_provider("themoviedb_images"));
        assert!(is_known_provider("tpdb"));
        assert!(!is_known_provider("unknown_provider"));
    }

    #[test]
    fn provider_key_aliases_are_normalized() {
        assert_eq!(normalize_provider_key("themoviedb_images"), "tmdb");
        assert_eq!(normalize_provider_key("theporndb"), "tpdb");
        assert_eq!(normalize_provider_key("openmoviedb"), "omdb");
    }

    #[test]
    fn unknown_provider_is_not_assumed_valid() {
        assert!(!should_assume_key_validity("unknown_provider"));
    }

    #[test]
    fn known_provider_without_live_check_is_assumed_valid() {
        assert!(should_assume_key_validity("tvdb"));
    }

    #[test]
    fn known_provider_with_live_check_is_not_assumed_valid() {
        assert!(!should_assume_key_validity("tmdb"));
        assert!(!should_assume_key_validity("tpdb"));
    }

    #[test]
    fn phoenix_filename_scene_query_is_extracted() {
        assert_eq!(
            extract_phoenix_scene_query(r"E:\Adult\Blacked - 2018-12-11 - The Real Thing.mp4")
                .as_deref(),
            Some("The Real Thing")
        );
    }

    #[test]
    fn phoenix_filename_query_is_prioritized() {
        let item = MediaItemLookup {
            id: 1,
            title: "Blacked - 2018-12-11 - The Real Thing".to_string(),
            file_path: r"E:\Adult\Blacked - 2018-12-11 - The Real Thing.mp4".to_string(),
            media_type: "adult".to_string(),
            overview: None,
            poster_path: None,
            year: None,
            rating: None,
            genre: None,
            tmdb_id: None,
            imdb_id: None,
        };

        let queries = build_metadata_queries(&item);
        assert_eq!(queries.first().map(String::as_str), Some("The Real Thing"));
    }
}

#[tauri::command]
pub fn set_api_key(
    state: State<AppState>,
    provider: String,
    api_key: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let provider = normalize_provider_key(&provider);
    db.conn
        .execute(
            "INSERT OR REPLACE INTO api_keys (provider, api_key) VALUES (?1, ?2)",
            params![provider, api_key],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_api_keys(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .conn
        .prepare("SELECT provider, api_key FROM api_keys")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;

    let mut keys = serde_json::Map::new();
    for row in rows {
        let (provider, key) = row.map_err(|e| e.to_string())?;
        let normalized_provider = normalize_provider_key(&provider);
        let masked = if key.len() > 4 {
            format!("{}...{}", &key[..2], &key[key.len() - 2..])
        } else {
            "****".to_string()
        };
        keys.insert(normalized_provider, serde_json::Value::String(masked));
    }

    Ok(serde_json::Value::Object(keys))
}
