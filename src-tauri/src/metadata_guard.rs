use crate::metadata::MetadataProvider;
use crate::{metadata_ext, AppState};
use rusqlite::params;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};
use tauri::State;

const IMPLEMENTED_PROVIDERS: &[&str] = &[
    "tmdb",
    "omdb",
    "tvmaze",
    "musicbrainz",
    "tpdb",
    "stashdb",
    "phoenixadult",
    "pgma",
    "porn_site_nuxt",
];

#[derive(Clone)]
struct ItemRecord {
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

#[derive(Clone, Default)]
struct MatchData {
    provider: String,
    title: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    year: Option<i32>,
    rating: Option<f64>,
    genre: Option<String>,
    tmdb_id: Option<String>,
    imdb_id: Option<String>,
    media_type: Option<String>,
}

#[derive(Default)]
struct UpdateData {
    title: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    year: Option<i32>,
    rating: Option<f64>,
    genre: Option<String>,
    tmdb_id: Option<String>,
    imdb_id: Option<String>,
    media_type: Option<String>,
}

#[derive(Serialize)]
struct UpdatedItem {
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

fn normalize_provider_key(provider: &str) -> String {
    match provider.trim().to_ascii_lowercase().as_str() {
        "themoviedb" | "themoviedb_images" | "tmdb_images" | "tmdb" => "tmdb".to_string(),
        "theporndb" | "tpdb" => "tpdb".to_string(),
        "open_movie_db" | "openmoviedb" | "omdb" => "omdb".to_string(),
        "pgma-modernized" | "pgma_modernized" | "pgma modernized" | "plex pgma" => "pgma".to_string(),
        "irenehub" | "porn-site-nuxt" | "porn_site_nuxt" => "porn_site_nuxt".to_string(),
        other => other.to_string(),
    }
}

fn ensure_implemented(provider: &str) -> Result<String, String> {
    let normalized = normalize_provider_key(provider);
    IMPLEMENTED_PROVIDERS
        .contains(&normalized.as_str())
        .then_some(normalized)
        .ok_or_else(|| format!("Metadata provider '{provider}' is not implemented in this build"))
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("N/A"))
        .map(str::to_string)
}

fn parse_year(value: Option<&str>) -> Option<i32> {
    let value = value?.trim();
    (value.len() >= 4).then(|| value[..4].parse::<i32>().ok()).flatten()
}

fn normalized_words(value: &str) -> Vec<String> {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|word| word.len() > 1)
        .map(str::to_string)
        .collect()
}

fn title_matches(expected: &str, candidate: &str) -> bool {
    let expected_words = normalized_words(expected);
    let candidate_words = normalized_words(candidate);
    if expected_words.is_empty() || candidate_words.is_empty() {
        return false;
    }
    let shared = expected_words
        .iter()
        .filter(|word| candidate_words.contains(word))
        .count();
    shared * 2 >= expected_words.len().min(candidate_words.len()).max(1)
}

fn is_adult(item: &ItemRecord) -> bool {
    if item.media_type.eq_ignore_ascii_case("adult") {
        return true;
    }
    let text = format!("{} {}", item.title, item.file_path).to_ascii_lowercase();
    ["adult", "porn", "xxx", "nsfw"].iter().any(|hint| text.contains(hint))
}

fn encoded(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC).to_string()
}

async fn tmdb_match(client: &reqwest::Client, query: &str, key: Option<&String>) -> Result<Option<MatchData>, String> {
    let Some(key) = key.filter(|key| !key.trim().is_empty()) else { return Ok(None); };
    let url = format!("https://api.themoviedb.org/3/search/multi?api_key={}&query={}&include_adult=false&page=1", key, encoded(query));
    let data = client.get(url).send().await.map_err(|e| e.to_string())?.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    let Some(first) = data.get("results").and_then(|v| v.as_array()).and_then(|v| v.first()) else { return Ok(None); };
    let title = clean(first.get("title").or_else(|| first.get("name")).and_then(|v| v.as_str()));
    if !title.as_deref().is_some_and(|value| title_matches(query, value)) { return Ok(None); }
    Ok(Some(MatchData {
        provider: "tmdb".to_string(),
        title,
        overview: clean(first.get("overview").and_then(|v| v.as_str())),
        poster_path: first.get("poster_path").and_then(|v| v.as_str()).filter(|v| !v.is_empty()).map(|v| format!("https://image.tmdb.org/t/p/w500{v}")),
        year: parse_year(first.get("release_date").or_else(|| first.get("first_air_date")).and_then(|v| v.as_str())),
        rating: first.get("vote_average").and_then(|v| v.as_f64()).filter(|v| *v > 0.0),
        tmdb_id: first.get("id").and_then(|v| v.as_i64()).map(|v| v.to_string()),
        ..MatchData::default()
    }))
}

async fn omdb_match(client: &reqwest::Client, query: &str, key: Option<&String>) -> Result<Option<MatchData>, String> {
    let Some(key) = key.filter(|key| !key.trim().is_empty()) else { return Ok(None); };
    let url = format!("https://www.omdbapi.com/?apikey={}&t={}&plot=full", key, encoded(query));
    let data = client.get(url).send().await.map_err(|e| e.to_string())?.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    if data.get("Response").and_then(|v| v.as_str()) != Some("True") { return Ok(None); }
    let title = clean(data.get("Title").and_then(|v| v.as_str()));
    if !title.as_deref().is_some_and(|value| title_matches(query, value)) { return Ok(None); }
    Ok(Some(MatchData {
        provider: "omdb".to_string(),
        title,
        overview: clean(data.get("Plot").and_then(|v| v.as_str())),
        poster_path: clean(data.get("Poster").and_then(|v| v.as_str())),
        year: parse_year(data.get("Year").and_then(|v| v.as_str())),
        rating: data.get("imdbRating").and_then(|v| v.as_str()).and_then(|v| v.parse::<f64>().ok()).filter(|v| *v > 0.0),
        genre: clean(data.get("Genre").and_then(|v| v.as_str())),
        imdb_id: clean(data.get("imdbID").and_then(|v| v.as_str())),
        ..MatchData::default()
    }))
}

async fn tpdb_match(client: &reqwest::Client, query: &str, key: Option<&String>) -> Result<Option<MatchData>, String> {
    let Some(key) = key.filter(|key| !key.trim().is_empty()) else { return Ok(None); };
    let search = client
        .get(format!("https://api.theporndb.net/scenes?parse={}&hash=&year=", encoded(query)))
        .bearer_auth(key)
        .send().await.map_err(|e| e.to_string())?
        .json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    let Some(id) = search.get("data").and_then(|v| v.as_array()).and_then(|v| v.first()).and_then(|v| v.get("uuid").or_else(|| v.get("UUID"))).and_then(|v| v.as_str()) else { return Ok(None); };
    let detail = client.get(format!("https://api.theporndb.net/scenes/{id}")).bearer_auth(key).send().await.map_err(|e| e.to_string())?.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    let detail = detail.get("data").unwrap_or(&detail);
    let title = clean(detail.get("title").and_then(|v| v.as_str()));
    if !title.as_deref().is_some_and(|value| title_matches(query, value)) { return Ok(None); }
    let genre = detail.get("tags").and_then(|v| v.as_array()).map(|tags| tags.iter().filter_map(|tag| tag.get("name").and_then(|v| v.as_str())).take(10).collect::<Vec<_>>().join(", ")).filter(|v| !v.is_empty());
    let poster_path = detail.get("posters").and_then(|v| v.get("large")).and_then(|v| v.as_str()).or_else(|| detail.get("poster").and_then(|v| v.as_str())).and_then(|v| clean(Some(v)));
    Ok(Some(MatchData {
        provider: "tpdb".to_string(),
        title,
        overview: clean(detail.get("description").or_else(|| detail.get("details")).and_then(|v| v.as_str())),
        poster_path,
        year: parse_year(detail.get("date").and_then(|v| v.as_str())),
        genre,
        imdb_id: clean(detail.get("uuid").and_then(|v| v.as_str())),
        media_type: Some("adult".to_string()),
        ..MatchData::default()
    }))
}

async fn stashdb_match(client: &reqwest::Client, query: &str, key: Option<&String>) -> Result<Option<MatchData>, String> {
    let Some(key) = key.filter(|key| !key.trim().is_empty()) else { return Ok(None); };
    let data = client.post("https://stashdb.org/graphql").header("ApiKey", key).json(&serde_json::json!({
        "query": "query($title:String!){ queryScenes(input:{title:$title, per_page:3, page:1, direction:DESC, sort:DATE}) { scenes { title details release_date images { url width height } tags { name } } } }",
        "variables": { "title": query }
    })).send().await.map_err(|e| e.to_string())?.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    let Some(scene) = data.get("data").and_then(|v| v.get("queryScenes")).and_then(|v| v.get("scenes")).and_then(|v| v.as_array()).and_then(|items| items.iter().find(|scene| scene.get("title").and_then(|v| v.as_str()).is_some_and(|title| title_matches(query, title)))) else { return Ok(None); };
    let title = clean(scene.get("title").and_then(|v| v.as_str()));
    let genre = scene.get("tags").and_then(|v| v.as_array()).map(|tags| tags.iter().filter_map(|tag| tag.get("name").and_then(|v| v.as_str())).take(10).collect::<Vec<_>>().join(", ")).filter(|v| !v.is_empty());
    Ok(Some(MatchData {
        provider: "stashdb".to_string(),
        title,
        overview: clean(scene.get("details").and_then(|v| v.as_str())),
        poster_path: scene.get("images").and_then(|v| v.as_array()).and_then(|v| v.first()).and_then(|v| v.get("url")).and_then(|v| v.as_str()).and_then(|v| clean(Some(v))),
        year: parse_year(scene.get("release_date").and_then(|v| v.as_str())),
        genre,
        media_type: Some("adult".to_string()),
        ..MatchData::default()
    }))
}

fn consensus_string(values: impl Iterator<Item = Option<String>>) -> Option<String> {
    let mut counts = BTreeMap::<String, (usize, String)>::new();
    for value in values.flatten().filter(|value| !value.trim().is_empty()) {
        let key = value.trim().to_ascii_lowercase();
        let entry = counts.entry(key).or_insert((0, value));
        entry.0 += 1;
    }
    counts.into_values().max_by_key(|(count, _)| *count).map(|(_, value)| value)
}

fn merge_matches(item: &ItemRecord, matches: &[MatchData], adult: bool) -> UpdateData {
    let title = consensus_string(matches.iter().map(|m| m.title.clone()));
    let overview = consensus_string(matches.iter().map(|m| m.overview.clone()));
    let poster_path = consensus_string(matches.iter().map(|m| m.poster_path.clone()));
    let genre = consensus_string(matches.iter().map(|m| m.genre.clone()));
    let tmdb_id = consensus_string(matches.iter().map(|m| m.tmdb_id.clone()));
    let imdb_id = consensus_string(matches.iter().map(|m| m.imdb_id.clone()));
    let year = matches.iter().filter_map(|m| m.year).next();
    let rating = matches.iter().filter_map(|m| m.rating).next();
    UpdateData {
        title: if item.title.trim().is_empty() || item.title.eq_ignore_ascii_case("unknown") { title } else { None },
        overview: item.overview.as_deref().filter(|v| !v.trim().is_empty()).is_none().then_some(overview).flatten(),
        poster_path: item.poster_path.as_deref().filter(|v| !v.trim().is_empty()).is_none().then_some(poster_path).flatten(),
        year: item.year.is_none().then_some(year).flatten(),
        rating: item.rating.is_none().then_some(rating).flatten(),
        genre: item.genre.as_deref().filter(|v| !v.trim().is_empty()).is_none().then_some(genre).flatten(),
        tmdb_id: item.tmdb_id.as_deref().filter(|v| !v.trim().is_empty()).is_none().then_some(tmdb_id).flatten(),
        imdb_id: item.imdb_id.as_deref().filter(|v| !v.trim().is_empty()).is_none().then_some(imdb_id).flatten(),
        media_type: (adult && !item.media_type.eq_ignore_ascii_case("adult")).then(|| "adult".to_string()),
    }
}

fn changed_fields(update: &UpdateData) -> usize {
    [update.title.is_some(), update.overview.is_some(), update.poster_path.is_some(), update.year.is_some(), update.rating.is_some(), update.genre.is_some(), update.tmdb_id.is_some(), update.imdb_id.is_some(), update.media_type.is_some()].into_iter().filter(|value| *value).count()
}

#[tauri::command]
pub fn get_metadata_providers() -> Vec<MetadataProvider> {
    metadata_ext::get_metadata_providers().into_iter().filter(|provider| IMPLEMENTED_PROVIDERS.contains(&provider.key.as_str())).collect()
}

#[tauri::command]
pub async fn fetch_metadata(provider: String, query: String, api_key: Option<String>) -> Result<serde_json::Value, String> {
    metadata_ext::fetch_metadata(ensure_implemented(&provider)?, query, api_key).await
}

#[tauri::command]
pub async fn search_metadata(provider: String, query: String, media_type: Option<String>, api_key: Option<String>) -> Result<serde_json::Value, String> {
    metadata_ext::search_metadata(ensure_implemented(&provider)?, query, media_type, api_key).await
}

#[tauri::command]
pub async fn check_media_item_metadata(state: State<'_, AppState>, id: i64) -> Result<serde_json::Value, String> {
    let started = Instant::now();
    let (item, keys) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let item = db.conn.query_row("SELECT id,title,file_path,media_type,overview,poster_path,year,rating,genre,tmdb_id,imdb_id FROM media_items WHERE id=?1", params![id], |row| Ok(ItemRecord { id: row.get(0)?, title: row.get(1)?, file_path: row.get(2)?, media_type: row.get(3)?, overview: row.get(4)?, poster_path: row.get(5)?, year: row.get(6)?, rating: row.get(7)?, genre: row.get(8)?, tmdb_id: row.get(9)?, imdb_id: row.get(10)? })).map_err(|e| e.to_string())?;
        let mut stmt = db.conn.prepare("SELECT provider,api_key FROM api_keys").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(|e| e.to_string())?;
        let mut keys = HashMap::new();
        for row in rows { let (provider, key) = row.map_err(|e| e.to_string())?; keys.insert(normalize_provider_key(&provider), key); }
        (item, keys)
    };

    let adult = is_adult(&item);
    let query = item.title.clone();
    let client = reqwest::Client::builder().connect_timeout(Duration::from_secs(2)).timeout(Duration::from_secs(7)).build().map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    let mut matches = Vec::new();

    if adult {
        let (tpdb, stashdb) = tokio::join!(tpdb_match(&client, &query, keys.get("tpdb")), stashdb_match(&client, &query, keys.get("stashdb")));
        for result in [tpdb, stashdb] { match result { Ok(Some(value)) if value.media_type.as_deref() == Some("adult") => matches.push(value), Ok(_) => {}, Err(error) => errors.push(error) } }
    } else {
        let (tmdb, omdb) = tokio::join!(tmdb_match(&client, &query, keys.get("tmdb")), omdb_match(&client, &query, keys.get("omdb")));
        for result in [tmdb, omdb] { match result { Ok(Some(value)) if value.media_type.as_deref() != Some("adult") => matches.push(value), Ok(_) => {}, Err(error) => errors.push(error) } }
    }

    let providers = matches.iter().map(|value| value.provider.clone()).collect::<Vec<_>>();
    let update = merge_matches(&item, &matches, adult);
    let count = changed_fields(&update);
    if count > 0 {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.conn.execute("UPDATE media_items SET title=COALESCE(?1,title),overview=COALESCE(?2,overview),poster_path=COALESCE(?3,poster_path),year=COALESCE(?4,year),rating=COALESCE(?5,rating),genre=COALESCE(?6,genre),tmdb_id=COALESCE(?7,tmdb_id),imdb_id=COALESCE(?8,imdb_id),media_type=COALESCE(?9,media_type) WHERE id=?10", params![update.title, update.overview, update.poster_path, update.year, update.rating, update.genre, update.tmdb_id, update.imdb_id, update.media_type, id]).map_err(|e| e.to_string())?;
    }

    let updated = UpdatedItem { id: item.id, title: update.title.clone().unwrap_or(item.title), file_path: item.file_path, media_type: update.media_type.clone().unwrap_or(item.media_type), overview: update.overview.clone().or(item.overview), poster_path: update.poster_path.clone().or(item.poster_path), year: update.year.or(item.year), rating: update.rating.or(item.rating), genre: update.genre.clone().or(item.genre), tmdb_id: update.tmdb_id.clone().or(item.tmdb_id), imdb_id: update.imdb_id.clone().or(item.imdb_id) };
    Ok(serde_json::json!({ "type": "single_item_metadata_check", "status": if count > 0 { "success" } else if matches.is_empty() { "no_match" } else { "no_changes" }, "item_id": id, "metadata_updated": count > 0, "metadata_fields_updated": count, "providers_matched": providers, "provider_errors": errors, "elapsed_ms": started.elapsed().as_millis(), "message": if count > 0 { format!("Metadata and artwork updated from {} matching provider(s)", matches.len()) } else { "Metadata check completed without new fields".to_string() }, "updated_item": updated }))
}

#[tauri::command]
pub fn get_provider_status(state: State<AppState>) -> Result<serde_json::Value, String> { metadata_ext::get_provider_status(state) }

#[tauri::command]
pub async fn test_api_key(provider: String, api_key: String) -> Result<serde_json::Value, String> { metadata_ext::test_api_key(ensure_implemented(&provider)?, api_key).await }

#[tauri::command]
pub fn set_api_key(state: State<AppState>, provider: String, api_key: String) -> Result<(), String> { metadata_ext::set_api_key(state, ensure_implemented(&provider)?, api_key) }

#[tauri::command]
pub fn get_api_keys(state: State<AppState>) -> Result<serde_json::Value, String> { metadata_ext::get_api_keys(state) }
