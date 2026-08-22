use crate::db::{Database, MediaItem};
use crate::metadata_keyless::{self, KeylessMetadataMatch};
use crate::AppState;
use serde::Serialize;
use std::path::Path;
use tauri::State;

#[derive(Debug, Clone, Default)]
struct MetadataUpdate {
    title: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    year: Option<i32>,
    rating: Option<f64>,
    genre: Option<String>,
    imdb_id: Option<String>,
    provider: Option<String>,
    poster_cached: bool,
    errors: Vec<String>,
}

impl MetadataUpdate {
    fn changed_fields(&self) -> usize {
        [
            self.title.is_some(),
            self.overview.is_some(),
            self.poster_path.is_some(),
            self.year.is_some(),
            self.rating.is_some(),
            self.genre.is_some(),
            self.imdb_id.is_some(),
        ]
        .into_iter()
        .filter(|value| *value)
        .count()
    }
}

#[derive(Debug, Default)]
struct KeylessPrepassReport {
    items_updated: usize,
    fields_updated: usize,
    posters_cached: usize,
    provider_errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SingleItemMetadataResult {
    #[serde(rename = "type")]
    result_type: &'static str,
    status: &'static str,
    item_id: i64,
    metadata_updated: bool,
    metadata_fields_updated: usize,
    providers_matched: Vec<String>,
    provider_errors: Vec<String>,
    poster_cached: bool,
    message: String,
    updated_item: MediaItem,
}

fn is_standard_video(item: &MediaItem) -> bool {
    matches!(
        item.media_type.trim().to_ascii_lowercase().as_str(),
        "movie" | "episode" | "video"
    )
}

fn blank(value: Option<&str>) -> bool {
    value.map(str::trim).map(str::is_empty).unwrap_or(true)
}

fn low_quality_title(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("unknown")
        || trimmed.contains('_')
        || trimmed.contains('.')
        || regex::Regex::new(
            r"(?i)\b(S\d{1,2}E\d{1,3}|\d{1,2}x\d{1,3}|480p|720p|1080p|2160p|x264|x265|webrip|bluray)\b",
        )
        .expect("low-quality title regex should compile")
        .is_match(trimmed)
}

fn needs_keyless_work(item: &MediaItem) -> bool {
    is_standard_video(item)
        && (low_quality_title(&item.title)
            || blank(item.overview.as_deref())
            || item.year.is_none()
            || item.rating.is_none()
            || blank(item.genre.as_deref())
            || blank(item.imdb_id.as_deref())
            || blank(item.poster_path.as_deref())
            || item
                .poster_path
                .as_deref()
                .is_some_and(|value| value.starts_with("https://")))
}

fn build_update(item: &MediaItem, matched: KeylessMetadataMatch) -> MetadataUpdate {
    let mut update = MetadataUpdate {
        provider: Some(matched.provider),
        ..MetadataUpdate::default()
    };

    if low_quality_title(&item.title) {
        update.title = matched.title.filter(|value| !value.trim().is_empty());
    }
    if blank(item.overview.as_deref()) {
        update.overview = matched.overview.filter(|value| !value.trim().is_empty());
    }
    if item.year.is_none() {
        update.year = matched.year;
    }
    if item.rating.is_none() {
        update.rating = matched.rating;
    }
    if blank(item.genre.as_deref()) {
        update.genre = matched.genre.filter(|value| !value.trim().is_empty());
    }
    if blank(item.imdb_id.as_deref()) {
        update.imdb_id = matched.imdb_id.filter(|value| !value.trim().is_empty());
    }

    update
}

async fn resolve_keyless_update(
    client: &reqwest::Client,
    app_data_dir: &Path,
    item: &MediaItem,
) -> Result<Option<MetadataUpdate>, String> {
    if !needs_keyless_work(item) {
        return Ok(None);
    }
    let id = item
        .id
        .ok_or_else(|| "Media row has no database id".to_string())?;
    let query = metadata_keyless::metadata_query(&item.title, &item.file_path);
    if query.trim().is_empty() {
        return Ok(None);
    }

    let matched = metadata_keyless::fetch_keyless_match(client, &query, &item.media_type).await?;
    let Some(matched) = matched else {
        return Ok(None);
    };
    let poster_url = matched.poster_url.clone().or_else(|| {
        item.poster_path
            .clone()
            .filter(|value| value.starts_with("https://"))
    });
    let mut update = build_update(item, matched);

    if let Some(url) = poster_url {
        match metadata_keyless::cache_remote_artwork(client, app_data_dir, id, "poster", &url).await
        {
            Ok(cached) => {
                if item.poster_path.as_deref() != Some(cached.path.as_str()) {
                    update.poster_path = Some(cached.path);
                }
                update.poster_cached = true;
            }
            Err(error) => update
                .errors
                .push(format!("artwork_cache/{query}: {error}")),
        }
    }

    Ok(Some(update))
}

fn apply_update(
    database: &Database,
    item: &MediaItem,
    update: &MetadataUpdate,
) -> Result<usize, String> {
    let changed_fields = update.changed_fields();
    if changed_fields == 0 {
        return Ok(0);
    }

    database
        .update_media_metadata_data(
            &item.file_path,
            update.title.as_deref(),
            update.overview.as_deref(),
            update.poster_path.as_deref(),
            update.year,
            update.rating,
            update.genre.as_deref(),
            None,
            update.imdb_id.as_deref(),
            None,
        )
        .map_err(|error| error.to_string())?;
    Ok(changed_fields)
}

fn load_item(database: &Database, id: i64) -> Result<MediaItem, String> {
    database
        .get_media_items_data(None, None, None)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|item| item.id == Some(id))
        .ok_or_else(|| format!("Media item {id} was not found"))
}

async fn run_keyless_prepass(state: &State<'_, AppState>) -> Result<KeylessPrepassReport, String> {
    let items = {
        let database = state.db.lock().map_err(|error| error.to_string())?;
        database
            .get_media_items_data(None, None, None)
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|item| needs_keyless_work(item) && Path::new(&item.file_path).is_file())
            .collect::<Vec<_>>()
    };

    let client = metadata_keyless::http_client()?;
    let mut report = KeylessPrepassReport::default();

    for item in items {
        match resolve_keyless_update(&client, &state.app_data_dir, &item).await {
            Ok(Some(update)) => {
                let changed = {
                    let database = state.db.lock().map_err(|error| error.to_string())?;
                    apply_update(&database, &item, &update)?
                };
                if changed > 0 {
                    report.items_updated += 1;
                    report.fields_updated += changed;
                }
                if update.poster_cached {
                    report.posters_cached += 1;
                }
                report.provider_errors.extend(update.errors);
            }
            Ok(None) => {}
            Err(error) => report
                .provider_errors
                .push(format!("keyless_metadata/{}: {error}", item.file_path)),
        }
    }

    Ok(report)
}

#[tauri::command]
pub async fn run_library_enrichment(
    state: State<'_, AppState>,
    rename_files: bool,
) -> Result<crate::enrichment::LibraryEnrichmentReport, String> {
    let prepass = run_keyless_prepass(&state).await?;
    let mut report = crate::enrichment::run_library_enrichment(state, rename_files).await?;

    report.metadata_items_enriched += prepass.items_updated;
    report.metadata_fields_updated += prepass.fields_updated;
    report.metadata_updated += prepass.items_updated;
    report.posters_downloaded += prepass.posters_cached;
    report.provider_errors.extend(prepass.provider_errors);
    Ok(report)
}

#[tauri::command]
pub async fn check_media_item_metadata(
    state: State<'_, AppState>,
    id: i64,
) -> Result<serde_json::Value, String> {
    let item = {
        let database = state.db.lock().map_err(|error| error.to_string())?;
        load_item(&database, id)?
    };

    if is_standard_video(&item) {
        let client = metadata_keyless::http_client()?;
        match resolve_keyless_update(&client, &state.app_data_dir, &item).await {
            Ok(Some(update)) => {
                let changed = {
                    let database = state.db.lock().map_err(|error| error.to_string())?;
                    apply_update(&database, &item, &update)?
                };
                if changed > 0 || update.poster_cached {
                    let updated_item = {
                        let database = state.db.lock().map_err(|error| error.to_string())?;
                        load_item(&database, id)?
                    };
                    let provider = update.provider.clone().into_iter().collect::<Vec<_>>();
                    return serde_json::to_value(SingleItemMetadataResult {
                        result_type: "single_item_metadata_check",
                        status: "success",
                        item_id: id,
                        metadata_updated: changed > 0,
                        metadata_fields_updated: changed,
                        providers_matched: provider,
                        provider_errors: update.errors,
                        poster_cached: update.poster_cached,
                        message: if update.poster_cached {
                            "Metadata matched and poster cached into CinaVault application data"
                                .to_string()
                        } else {
                            "Metadata matched from a keyless provider".to_string()
                        },
                        updated_item,
                    })
                    .map_err(|error| error.to_string());
                }
            }
            Ok(None) => {}
            Err(error) => log::warn!("Keyless metadata lookup failed for item {id}: {error}"),
        }
    }

    crate::metadata_guard::check_media_item_metadata(state, id).await
}

#[cfg(test)]
mod tests {
    use super::{apply_update, load_item, resolve_keyless_update};
    use crate::db::{Database, MediaItem};
    use crate::metadata_keyless;
    use std::fs;
    use std::path::Path;
    use uuid::Uuid;

    fn media_item(title: &str, media_type: &str, file_path: String) -> MediaItem {
        MediaItem {
            id: None,
            title: title.to_string(),
            file_path,
            media_type: media_type.to_string(),
            year: None,
            rating: None,
            overview: None,
            poster_path: None,
            backdrop_path: None,
            genre: None,
            duration: None,
            file_size: Some(1),
            resolution: None,
            codec: None,
            verified: false,
            watched: false,
            favorite: false,
            date_added: "2026-07-26T00:00:00Z".to_string(),
            last_played: None,
            tmdb_id: None,
            imdb_id: None,
            source_id: None,
        }
    }

    fn insert_fixture(database: &Database, item: MediaItem) -> MediaItem {
        database
            .add_media_item_data(&item)
            .expect("media fixture should insert");
        database
            .get_media_items_data(None, None, None)
            .expect("media fixture should be readable")
            .into_iter()
            .find(|candidate| candidate.file_path == item.file_path)
            .expect("media fixture should exist")
    }

    fn assert_cached_poster(reloaded: &MediaItem, app_dir: &Path) {
        let poster = reloaded
            .poster_path
            .clone()
            .expect("poster path should be posted to SQLite");
        let poster_path = std::path::PathBuf::from(&poster);
        assert!(poster_path.is_file(), "cached poster must exist on disk");
        assert!(poster_path.starts_with(app_dir.join("artwork")));
        let bytes = fs::read(&poster_path).expect("cached poster should be readable");
        assert!(
            bytes.len() > 1024,
            "cached poster should contain real image bytes"
        );
        assert!(
            bytes.starts_with(&[0xFF, 0xD8, 0xFF])
                || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"),
            "cached poster must have a recognized image signature"
        );
    }

    #[tokio::test]
    #[ignore = "requires live TVMaze and artwork network access"]
    async fn live_metadata_poster_acceptance_tvmaze_series() {
        let root = std::env::temp_dir().join(format!("cinavault-tv-live-{}", Uuid::new_v4()));
        let media_dir = root.join("media");
        let app_dir = root.join("appdata");
        fs::create_dir_all(&media_dir).expect("test media directory should be created");
        fs::create_dir_all(&app_dir).expect("test app directory should be created");
        let media_path = media_dir.join("Breaking.Bad.S01E01.1080p.mkv");
        fs::write(&media_path, [0u8]).expect("current media fixture should exist");

        let database = Database::new(":memory:").expect("test database should initialize");
        let inserted = insert_fixture(
            &database,
            media_item(
                "Breaking.Bad.S01E01.1080p",
                "episode",
                media_path.to_string_lossy().to_string(),
            ),
        );

        let client =
            metadata_keyless::http_client().expect("live metadata client should initialize");
        let update = resolve_keyless_update(&client, &app_dir, &inserted)
            .await
            .expect("live keyless lookup should complete")
            .expect("Breaking Bad should resolve through live TVMaze metadata");
        assert_eq!(update.provider.as_deref(), Some("tvmaze"));
        assert!(update.poster_cached, "live poster bytes must be cached");
        let changed = apply_update(&database, &inserted, &update)
            .expect("live metadata should write to SQLite");
        assert!(changed >= 3, "multiple metadata fields should be written");

        let reloaded = load_item(&database, inserted.id.expect("inserted media id"))
            .expect("updated media row should reload");
        assert!(
            reloaded.title.to_ascii_lowercase().contains("breaking bad"),
            "provider title should be posted to the media row"
        );
        assert!(reloaded.year.is_some());
        assert!(reloaded
            .imdb_id
            .as_deref()
            .is_some_and(|value| value.starts_with("tt")));
        assert_cached_poster(&reloaded, &app_dir);

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    #[ignore = "requires live Cinemeta and artwork network access"]
    async fn live_metadata_poster_acceptance_cinemeta_movie() {
        let root = std::env::temp_dir().join(format!("cinavault-movie-live-{}", Uuid::new_v4()));
        let media_dir = root.join("media");
        let app_dir = root.join("appdata");
        fs::create_dir_all(&media_dir).expect("test media directory should be created");
        fs::create_dir_all(&app_dir).expect("test app directory should be created");
        let media_path = media_dir.join("Inception.2010.1080p.mkv");
        fs::write(&media_path, [0u8]).expect("current movie fixture should exist");

        let database = Database::new(":memory:").expect("test database should initialize");
        let inserted = insert_fixture(
            &database,
            media_item(
                "Inception.2010.1080p",
                "movie",
                media_path.to_string_lossy().to_string(),
            ),
        );

        let client =
            metadata_keyless::http_client().expect("live metadata client should initialize");
        let update = resolve_keyless_update(&client, &app_dir, &inserted)
            .await
            .expect("live movie lookup should complete")
            .expect("Inception should resolve through live Cinemeta metadata");
        assert_eq!(update.provider.as_deref(), Some("cinemeta"));
        assert!(
            update.poster_cached,
            "live movie poster bytes must be cached"
        );
        let changed = apply_update(&database, &inserted, &update)
            .expect("live movie metadata should write to SQLite");
        assert!(
            changed >= 3,
            "multiple movie metadata fields should be written"
        );

        let reloaded = load_item(&database, inserted.id.expect("inserted media id"))
            .expect("updated movie row should reload");
        assert_eq!(reloaded.title, "Inception");
        assert_eq!(reloaded.year, Some(2010));
        assert!(reloaded
            .imdb_id
            .as_deref()
            .is_some_and(|value| value.starts_with("tt")));
        assert_cached_poster(&reloaded, &app_dir);

        let _ = fs::remove_dir_all(root);
    }
}
