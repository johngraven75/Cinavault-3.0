use crate::library_artifacts::sidecar_poster_path_for_video;
use crate::{task_progress, AppState};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    StandardVideo,
    AdultVideo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrichmentMode {
    MetadataOnly,
    MetadataAndRename,
}

impl EnrichmentMode {
    fn as_report_mode(self) -> &'static str {
        match self {
            EnrichmentMode::MetadataOnly => "metadata_only",
            EnrichmentMode::MetadataAndRename => "metadata_and_filename_normalization",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameTarget {
    Ready(PathBuf, String),
    Collision(PathBuf),
    Invalid(String),
    Unchanged(PathBuf),
}

#[derive(Debug, Clone)]
pub struct LibraryItemRecord {
    pub id: i64,
    pub title: String,
    pub file_path: String,
    pub media_type: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub genre: Option<String>,
    pub tmdb_id: Option<String>,
    pub imdb_id: Option<String>,
    pub source_name: Option<String>,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderMatch {
    pub title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub genre: Option<String>,
    pub tmdb_id: Option<String>,
    pub imdb_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDecision {
    pub allow_rename: bool,
    pub normalized_title: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct EnrichmentItemSummary {
    id: i64,
    old_title: String,
    new_title: String,
    file_path: String,
    action: String,
}

#[derive(Debug, Serialize)]
pub struct LibraryEnrichmentReport {
    #[serde(rename = "type")]
    pub result_type: &'static str,
    pub status: &'static str,
    pub mode: &'static str,
    pub items_scanned: usize,
    pub metadata_items_enriched: usize,
    pub metadata_fields_updated: usize,
    pub metadata_updated: usize,
    pub titles_improved: usize,
    pub items_reclassified_as_adult: usize,
    pub files_renamed: usize,
    pub rename_collisions_skipped: usize,
    pub rename_failures: usize,
    pub low_confidence_metadata_only: usize,
    pub skipped_missing_files: usize,
    pub skipped_non_video_items: usize,
    pub posters_downloaded: usize,
    pub sidecars_written: usize,
    pub provider_errors: Vec<String>,
    pub samples: Vec<EnrichmentItemSummary>,
}

pub(crate) fn has_adult_hint(text: &str) -> bool {
    let lower = text.replace(['\\', '/', '_', '-', '.'], " ").to_lowercase();

    [
        "adult",
        "porn",
        "xxx",
        "nsfw",
        "personal x",
        "x library",
        "vids x",
        "videos x",
        "18 plus",
        "18+",
        "erotic",
        "explicit",
        "onlyfans",
        "fansly",
        "brazzers",
        "pornhub",
        "xvideos",
        "xnxx",
    ]
    .iter()
    .any(|hint| lower.contains(hint))
}

pub fn classify_library_item(item: &LibraryItemRecord) -> SourceKind {
    if item.media_type.eq_ignore_ascii_case("adult")
        || has_adult_hint(&item.title)
        || has_adult_hint(&item.file_path)
        || item
            .source_name
            .as_deref()
            .map(has_adult_hint)
            .unwrap_or(false)
        || item
            .source_path
            .as_deref()
            .map(has_adult_hint)
            .unwrap_or(false)
    {
        SourceKind::AdultVideo
    } else {
        SourceKind::StandardVideo
    }
}

pub fn normalize_filename_title(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.to_string());
    let stem = stem.trim();
    if stem.is_empty() || looks_like_timestamp_only(stem) {
        return String::new();
    }

    let mut text = stem.replace(['.', '_'], " ");
    text = text.replace('-', " ");
    text = Regex::new(
        r"(?i)\[[^\]]*\]|\([^\)]*(1080p|720p|2160p|x264|x265|hevc|aac|web[- ]?dl|bluray)[^\)]*\)",
    )
    .expect("release group regex should compile")
    .replace_all(&text, " ")
    .to_string();

    let year = Regex::new(r"\b(19\d{2}|20\d{2})\b")
        .expect("year regex should compile")
        .captures(&text)
        .and_then(|captures| captures.get(1).map(|year| year.as_str().to_string()));

    text = Regex::new(
        r"(?i)\b(480p|576p|720p|1080p|1440p|2160p|4k|8k|x264|x265|h264|h265|hevc|avc|aac|ac3|dts|ddp5?|atmos|web\s?dl|webrip|bluray|brrip|dvdrip|hdrip|proper|repack|extended|remux|yts|rarbg|eztv|group)\b",
    )
    .expect("release token regex should compile")
    .replace_all(&text, " ")
    .to_string();

    if let Some(ref year) = year {
        text = Regex::new(&format!(r"\b{}\b", regex::escape(year)))
            .expect("escaped year regex should compile")
            .replace_all(&text, " ")
            .to_string();
    }

    text = Regex::new(r"\s+")
        .expect("space regex should compile")
        .replace_all(text.trim(), " ")
        .trim_matches(|c: char| c == '-' || c == '.' || c.is_whitespace())
        .to_string();

    if text.is_empty() || looks_like_timestamp_only(&text) {
        return String::new();
    }

    let titled = title_case(&text);
    match year {
        Some(year) if !titled.contains(&year) => format!("{titled} ({year})"),
        _ => titled,
    }
}

pub fn build_query_candidates(
    item: &LibraryItemRecord,
    embedded_title: Option<String>,
) -> Vec<String> {
    let mut queries = Vec::new();
    push_unique_query(&mut queries, embedded_title);
    push_unique_query(&mut queries, Some(item.title.trim().to_string()));

    let normalized = normalize_filename_title(&item.file_path);
    push_unique_query(&mut queries, Some(normalized));

    if let Some(stem) = Path::new(&item.file_path).file_stem() {
        let reduced = stem
            .to_string_lossy()
            .replace(['.', '_', '-'], " ")
            .split_whitespace()
            .filter(|part| !looks_like_timestamp_only(part))
            .collect::<Vec<_>>()
            .join(" ");
        push_unique_query(&mut queries, Some(reduced));
    }

    queries
}

pub fn rename_confidence(
    item: &LibraryItemRecord,
    embedded_title: Option<&str>,
    provider: &ProviderMatch,
    mode: EnrichmentMode,
) -> RenameDecision {
    if mode != EnrichmentMode::MetadataAndRename {
        return RenameDecision {
            allow_rename: false,
            normalized_title: provider.title.as_deref().and_then(clean_title_for_display),
            reason: "metadata-only mode".to_string(),
        };
    }

    let provider_title = provider.title.as_deref().and_then(clean_title_for_display);
    let Some(provider_title) = provider_title else {
        return RenameDecision {
            allow_rename: false,
            normalized_title: None,
            reason: "provider did not return a usable title".to_string(),
        };
    };

    let embedded_support = embedded_title
        .and_then(clean_title_for_display)
        .map(|title| strong_title_match(&title, &provider_title))
        .unwrap_or(false);
    let filename_title = normalize_filename_title(&item.file_path);
    let filename_support =
        !filename_title.is_empty() && strong_title_match(&filename_title, &provider_title);
    let current_support =
        !is_low_quality_title(&item.title) && strong_title_match(&item.title, &provider_title);
    let stable_id = provider
        .tmdb_id
        .as_deref()
        .map(|id| !id.trim().is_empty())
        .unwrap_or(false)
        || provider
            .imdb_id
            .as_deref()
            .map(|id| !id.trim().is_empty())
            .unwrap_or(false);

    let local_title_support = embedded_support || filename_support || current_support;
    let external_or_embedded_support = embedded_support || stable_id;
    let allow_rename = local_title_support && external_or_embedded_support;
    let reason = match (
        allow_rename,
        local_title_support,
        external_or_embedded_support,
    ) {
        (true, _, _) => "balanced confidence satisfied".to_string(),
        (false, false, _) => "provider title lacks local title support".to_string(),
        (false, true, false) => {
            "provider title lacks stable id or embedded title support".to_string()
        }
        (false, true, true) => "balanced confidence was not satisfied".to_string(),
    };

    RenameDecision {
        allow_rename,
        normalized_title: Some(provider_title),
        reason,
    }
}

pub fn safe_rename_target(source: &Path, normalized_title: &str) -> RenameTarget {
    let Some(parent) = source.parent() else {
        return RenameTarget::Invalid("source file has no parent".to_string());
    };
    let Some(extension) = source.extension().and_then(|ext| ext.to_str()) else {
        return RenameTarget::Invalid("source file has no extension".to_string());
    };

    let cleaned = sanitize_windows_filename(normalized_title);
    if cleaned.is_empty() {
        return RenameTarget::Invalid("normalized title is empty".to_string());
    }

    let candidate = parent.join(format!("{cleaned}.{extension}"));
    if candidate == source {
        return RenameTarget::Unchanged(candidate);
    }
    if candidate.exists() {
        return RenameTarget::Collision(candidate);
    }

    RenameTarget::Ready(candidate, cleaned)
}

#[tauri::command]
pub async fn run_library_enrichment(
    state: State<'_, AppState>,
    rename_files: bool,
) -> Result<LibraryEnrichmentReport, String> {
    let mode = if rename_files {
        EnrichmentMode::MetadataAndRename
    } else {
        EnrichmentMode::MetadataOnly
    };

    let (items, provider_keys) = {
        let db = state.db.lock().map_err(|err| err.to_string())?;
        let mut stmt = db
            .conn
            .prepare(
                "SELECT mi.id,
                        mi.title,
                        mi.file_path,
                        mi.media_type,
                        mi.overview,
                        mi.poster_path,
                        mi.year,
                        mi.rating,
                        mi.genre,
                        mi.tmdb_id,
                        mi.imdb_id,
                        ms.name,
                        ms.path
                 FROM media_items mi
                 LEFT JOIN media_sources ms ON ms.id = mi.source_id
                 WHERE mi.media_type IN ('adult', 'movie', 'episode', 'video')
                 ORDER BY date_added DESC",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LibraryItemRecord {
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
                    source_name: row.get(11)?,
                    source_path: row.get(12)?,
                })
            })
            .map_err(|err| err.to_string())?;
        let items = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())?;

        let provider_keys = load_provider_keys(&db)?;
        (items, provider_keys)
    };

    let mut report = LibraryEnrichmentReport {
        result_type: "library_enrichment",
        status: "success",
        mode: mode.as_report_mode(),
        items_scanned: items.len(),
        metadata_items_enriched: 0,
        metadata_fields_updated: 0,
        metadata_updated: 0,
        titles_improved: 0,
        items_reclassified_as_adult: 0,
        files_renamed: 0,
        rename_collisions_skipped: 0,
        rename_failures: 0,
        low_confidence_metadata_only: 0,
        skipped_missing_files: 0,
        skipped_non_video_items: 0,
        posters_downloaded: 0,
        sidecars_written: 0,
        provider_errors: Vec::new(),
        samples: Vec::new(),
    };
    let mut progress = task_progress::MetadataTaskGuard::start(
        "library_enrichment",
        if rename_files {
            "Normalize Filenames"
        } else {
            "Enrich Metadata"
        },
        items.len(),
        "Preparing library metadata enrichment",
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|err| err.to_string())?;

    let total_items = items.len();
    for (index, item) in items.into_iter().enumerate() {
        if task_progress::stop_requested() {
            report
                .provider_errors
                .push("Stopped by user before processing the next item".to_string());
            break;
        }
        progress.update(
            index + 1,
            format!("Enriching metadata for {} of {}", index + 1, total_items),
        );
        if !is_video_library_item(&item) {
            report.skipped_non_video_items += 1;
            continue;
        }

        let source = Path::new(&item.file_path);
        if !source.exists() {
            report.skipped_missing_files += 1;
            continue;
        }

        let embedded_title = extract_embedded_title(&item.file_path);
        let source_kind = classify_library_item(&item);
        let queries = build_query_candidates(&item, embedded_title.clone());
        let remote_provider = resolve_provider_match(
            &client,
            &provider_keys,
            source_kind.clone(),
            &queries,
            &item.file_path,
            &mut report.provider_errors,
        )
        .await;
        let local_title_provider = (source_kind != SourceKind::AdultVideo)
            .then(|| {
                local_embedded_title_match(embedded_title.as_deref())
                    .or_else(|| local_display_title_match(&item))
            })
            .flatten();
        let local_artwork_provider = (source_kind != SourceKind::AdultVideo)
            .then(|| local_sidecar_artwork_match(&item))
            .flatten();
        let provider = [
            remote_provider,
            local_title_provider,
            local_artwork_provider,
        ]
        .into_iter()
        .flatten()
        .reduce(merge_provider_matches);

        let Some(provider) = provider else {
            report.low_confidence_metadata_only += 1;
            continue;
        };

        let mut update = build_metadata_update(&item, &provider, &source_kind);

        // Acquire new provider posters and migrate previously stored remote URLs to local sidecars.
        let poster_candidate = update.poster_path.clone().or_else(|| {
            item.poster_path
                .clone()
                .filter(|path| path.starts_with("http://") || path.starts_with("https://"))
        });
        if let Some(remote_url) = poster_candidate {
            if remote_url.starts_with("http://") || remote_url.starts_with("https://") {
                match download_poster_to_sidecar(&client, &remote_url, &item.file_path).await {
                    Ok(local_path) => {
                        if update.poster_path.is_none() {
                            update.changed_fields += 1;
                        }
                        update.poster_path = Some(local_path);
                        report.posters_downloaded += 1;
                    }
                    Err(err) => {
                        report
                            .provider_errors
                            .push(format!("poster_download/{}: {}", item.file_path, err));
                    }
                }
            }
        }

        // Write NFO sidecar with all metadata fields
        if update.changed_fields > 0 || update.poster_path.is_some() {
            let nfo_title = update.title.as_deref().unwrap_or(&item.title);
            let nfo_year = update.year.or(item.year);
            let nfo_overview = update.overview.as_deref().or(item.overview.as_deref());
            let nfo_rating = update.rating.or(item.rating);
            let nfo_genre = update.genre.as_deref().or(item.genre.as_deref());
            let nfo_tmdb = update.tmdb_id.as_deref().or(item.tmdb_id.as_deref());
            let nfo_imdb = update.imdb_id.as_deref().or(item.imdb_id.as_deref());
            let nfo_poster = update.poster_path.as_deref();
            match write_nfo_sidecar(
                &item.file_path,
                nfo_title,
                nfo_year,
                nfo_overview,
                nfo_rating,
                nfo_genre,
                nfo_tmdb,
                nfo_imdb,
                nfo_poster,
            ) {
                Ok(()) => report.sidecars_written += 1,
                Err(err) => report
                    .provider_errors
                    .push(format!("nfo_write/{}: {}", item.file_path, err)),
            }
        }

        if update.changed_fields > 0 {
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
            report.metadata_items_enriched += 1;
            report.metadata_fields_updated += update.changed_fields;
            report.metadata_updated += 1;
            if update.title.is_some() {
                report.titles_improved += 1;
            }
            if update.media_type.as_deref() == Some("adult") {
                report.items_reclassified_as_adult += 1;
            }
            push_sample(
                &mut report.samples,
                item.id,
                &item.title,
                update.title.as_deref().unwrap_or(&item.title),
                &item.file_path,
                "metadata_updated",
            );
        }

        if mode != EnrichmentMode::MetadataAndRename {
            continue;
        }

        let decision = rename_confidence(&item, embedded_title.as_deref(), &provider, mode);
        if !decision.allow_rename {
            report.low_confidence_metadata_only += 1;
            continue;
        }

        let Some(normalized_title) = decision.normalized_title else {
            report.low_confidence_metadata_only += 1;
            continue;
        };

        match safe_rename_target(source, &normalized_title) {
            RenameTarget::Ready(target, cleaned_title) => match std::fs::rename(source, &target) {
                Ok(()) => {
                    let new_path = target.to_string_lossy().to_string();
                    let db = state.db.lock().map_err(|err| err.to_string())?;
                    db.update_media_file_path_data(&item.file_path, &new_path, &cleaned_title)
                        .map_err(|err| err.to_string())?;
                    report.files_renamed += 1;
                    push_sample(
                        &mut report.samples,
                        item.id,
                        &item.title,
                        &cleaned_title,
                        &item.file_path,
                        "file_renamed",
                    );
                }
                Err(_) => report.rename_failures += 1,
            },
            RenameTarget::Collision(_) => report.rename_collisions_skipped += 1,
            RenameTarget::Invalid(_) | RenameTarget::Unchanged(_) => {
                report.low_confidence_metadata_only += 1;
            }
        }
    }

    progress.finish(format!(
        "Metadata enrichment complete: {} metadata updates, {} files renamed",
        report.metadata_updated, report.files_renamed
    ));

    Ok(report)
}

#[derive(Debug, Default)]
struct MetadataUpdate {
    title: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    year: Option<i32>,
    rating: Option<f64>,
    genre: Option<String>,
    tmdb_id: Option<String>,
    imdb_id: Option<String>,
    media_type: Option<String>,
    changed_fields: usize,
}

fn load_provider_keys(db: &crate::db::Database) -> Result<HashMap<String, String>, String> {
    let mut stmt = db
        .conn
        .prepare("SELECT provider, api_key FROM api_keys")
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| err.to_string())?;

    let mut keys = HashMap::new();
    for row in rows {
        let (provider, key) = row.map_err(|err| err.to_string())?;
        if key.trim().is_empty() {
            continue;
        }
        keys.insert(provider.trim().to_lowercase(), key.clone());
        keys.insert(normalize_provider_key(&provider), key);
    }
    Ok(keys)
}

fn normalize_provider_key(provider: &str) -> String {
    match provider.trim().to_lowercase().as_str() {
        "themoviedb" | "themoviedb_images" | "tmdb_images" | "tmdb" => "tmdb".to_string(),
        "theporndb" | "tpdb" => "tpdb".to_string(),
        "open_movie_db" | "openmoviedb" | "omdb" => "omdb".to_string(),
        other => other.to_string(),
    }
}

async fn resolve_provider_match(
    client: &reqwest::Client,
    provider_keys: &HashMap<String, String>,
    source_kind: SourceKind,
    queries: &[String],
    file_path: &str,
    provider_errors: &mut Vec<String>,
) -> Option<ProviderMatch> {
    for query in queries {
        let mut matches = Vec::new();
        match source_kind {
            SourceKind::AdultVideo => {
                if let Some(result) = crate::metadata::fetch_adult_metadata_for_batch(
                    client,
                    provider_keys,
                    query,
                    file_path,
                    provider_errors,
                )
                .await
                {
                    matches.push(ProviderMatch {
                        title: result.title,
                        overview: result.overview,
                        poster_path: result.poster_path,
                        year: result.year,
                        rating: result.rating,
                        genre: result.genre,
                        tmdb_id: result.tmdb_id,
                        imdb_id: result.imdb_id,
                    });
                }
            }
            SourceKind::StandardVideo => {
                fetch_standard_metadata(
                    client,
                    provider_keys,
                    query,
                    provider_errors,
                    &mut matches,
                )
                .await;
            }
        }

        let merged = matches.into_iter().reduce(merge_provider_matches);
        if merged.as_ref().and_then(|m| m.title.as_ref()).is_some() {
            return merged;
        }
    }

    None
}

async fn fetch_standard_metadata(
    client: &reqwest::Client,
    provider_keys: &HashMap<String, String>,
    query: &str,
    provider_errors: &mut Vec<String>,
    matches: &mut Vec<ProviderMatch>,
) {
    if let Some(key) = provider_keys.get("tmdb") {
        match fetch_tmdb_metadata(client, key, query).await {
            Ok(Some(result)) => matches.push(result),
            Ok(None) => {}
            Err(err) => provider_errors.push(format!("tmdb/{query}: {err}")),
        }
    }
    if let Some(key) = provider_keys.get("omdb") {
        match fetch_omdb_metadata(client, key, query).await {
            Ok(Some(result)) => matches.push(result),
            Ok(None) => {}
            Err(err) => provider_errors.push(format!("omdb/{query}: {err}")),
        }
    }
}

async fn fetch_tmdb_metadata(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
) -> Result<Option<ProviderMatch>, String> {
    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    let url = format!(
        "https://api.themoviedb.org/3/search/multi?api_key={api_key}&query={encoded}&include_adult=true&page=1"
    );
    let data = client
        .get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|err| err.to_string())?;
    let Some(first) = data
        .get("results")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
    else {
        return Ok(None);
    };

    Ok(Some(ProviderMatch {
        title: non_empty_string(first.get("title").and_then(|value| value.as_str()))
            .or_else(|| non_empty_string(first.get("name").and_then(|value| value.as_str()))),
        overview: non_empty_string(first.get("overview").and_then(|value| value.as_str())),
        poster_path: first
            .get("poster_path")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
            .map(|poster| format!("https://image.tmdb.org/t/p/w500{poster}")),
        year: parse_year_prefix(first.get("release_date").and_then(|value| value.as_str()))
            .or_else(|| {
                parse_year_prefix(first.get("first_air_date").and_then(|value| value.as_str()))
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
    }))
}

async fn fetch_omdb_metadata(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
) -> Result<Option<ProviderMatch>, String> {
    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    let url = format!("https://www.omdbapi.com/?apikey={api_key}&t={encoded}&plot=full");
    let data = client
        .get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|err| err.to_string())?;
    if data.get("Response").and_then(|value| value.as_str()) != Some("True") {
        return Ok(None);
    }

    Ok(Some(ProviderMatch {
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
    }))
}

async fn fetch_stashdb_metadata(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
) -> Result<Option<ProviderMatch>, String> {
    let body = serde_json::json!({
        "query": "query($title:String!){ queryScenes(input:{title:$title, per_page:1, page:1, direction:DESC, sort:DATE}) { scenes { title details release_date images { url width height } tags { name } urls { url } } } }",
        "variables": { "title": query }
    });

    let data = client
        .post("https://stashdb.org/graphql")
        .header("Content-Type", "application/json")
        .header("ApiKey", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| err.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|err| err.to_string())?;
    if data.get("errors").is_some() {
        return Ok(None);
    }

    let Some(first) = data
        .get("data")
        .and_then(|value| value.get("queryScenes"))
        .and_then(|value| value.get("scenes"))
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
    else {
        return Ok(None);
    };

    let genre = first
        .get("tags")
        .and_then(|value| value.as_array())
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| tag.get("name").and_then(|value| value.as_str()))
                .filter(|name| !name.trim().is_empty())
                .take(5)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.trim().is_empty());

    Ok(Some(ProviderMatch {
        title: non_empty_string(first.get("title").and_then(|value| value.as_str())),
        overview: non_empty_string(first.get("details").and_then(|value| value.as_str())),
        poster_path: first
            .get("images")
            .and_then(|value| value.as_array())
            .and_then(|images| images.first())
            .and_then(|image| image.get("url"))
            .and_then(|value| value.as_str())
            .and_then(|value| non_empty_string(Some(value))),
        year: parse_year_prefix(first.get("release_date").and_then(|value| value.as_str())),
        rating: None,
        genre,
        tmdb_id: None,
        imdb_id: None,
    }))
}

fn merge_provider_matches(mut primary: ProviderMatch, secondary: ProviderMatch) -> ProviderMatch {
    if primary
        .title
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        primary.title = secondary.title;
    }
    if primary
        .overview
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        primary.overview = secondary.overview;
    }
    if primary
        .poster_path
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        primary.poster_path = secondary.poster_path;
    }
    if primary.year.is_none() {
        primary.year = secondary.year;
    }
    if primary.rating.is_none() {
        primary.rating = secondary.rating;
    }
    if primary
        .genre
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        primary.genre = secondary.genre;
    }
    if primary
        .tmdb_id
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        primary.tmdb_id = secondary.tmdb_id;
    }
    if primary
        .imdb_id
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        primary.imdb_id = secondary.imdb_id;
    }
    primary
}

fn local_display_title_match(item: &LibraryItemRecord) -> Option<ProviderMatch> {
    let title = normalize_filename_title(&item.file_path);
    if title.is_empty() || !should_update_title(&item.title, &title, &item.file_path) {
        return None;
    }

    Some(ProviderMatch {
        title: Some(title),
        ..ProviderMatch::default()
    })
}

fn local_embedded_title_match(embedded_title: Option<&str>) -> Option<ProviderMatch> {
    let title = embedded_title.and_then(clean_title_for_display)?;

    Some(ProviderMatch {
        title: Some(title),
        ..ProviderMatch::default()
    })
}

fn local_sidecar_artwork_match(item: &LibraryItemRecord) -> Option<ProviderMatch> {
    let poster_path = sidecar_poster_path_for_video(Path::new(&item.file_path))?;

    Some(ProviderMatch {
        poster_path: Some(poster_path.to_string_lossy().to_string()),
        ..ProviderMatch::default()
    })
}

fn build_metadata_update(
    item: &LibraryItemRecord,
    provider: &ProviderMatch,
    source_kind: &SourceKind,
) -> MetadataUpdate {
    let mut update = MetadataUpdate::default();

    if let Some(title) = provider.title.as_deref().and_then(clean_title_for_display) {
        if should_update_title(&item.title, &title, &item.file_path) {
            update.title = Some(title);
            update.changed_fields += 1;
        }
    }
    if should_update_optional_text(item.overview.as_deref(), provider.overview.as_deref()) {
        update.overview = provider
            .overview
            .as_deref()
            .and_then(|value| non_empty_string(Some(value)));
        update.changed_fields += usize::from(update.overview.is_some());
    }
    if should_update_optional_text(item.poster_path.as_deref(), provider.poster_path.as_deref()) {
        update.poster_path = provider
            .poster_path
            .as_deref()
            .and_then(|value| non_empty_string(Some(value)));
        update.changed_fields += usize::from(update.poster_path.is_some());
    }
    if item.year.is_none() && provider.year.is_some() {
        update.year = provider.year;
        update.changed_fields += 1;
    }
    if item.rating.is_none() && provider.rating.is_some() {
        update.rating = provider.rating;
        update.changed_fields += 1;
    }
    if should_update_optional_text(item.genre.as_deref(), provider.genre.as_deref()) {
        update.genre = provider
            .genre
            .as_deref()
            .and_then(|value| non_empty_string(Some(value)));
        update.changed_fields += usize::from(update.genre.is_some());
    }
    if should_update_optional_text(item.tmdb_id.as_deref(), provider.tmdb_id.as_deref()) {
        update.tmdb_id = provider
            .tmdb_id
            .as_deref()
            .and_then(|value| non_empty_string(Some(value)));
        update.changed_fields += usize::from(update.tmdb_id.is_some());
    }
    if should_update_optional_text(item.imdb_id.as_deref(), provider.imdb_id.as_deref()) {
        update.imdb_id = provider
            .imdb_id
            .as_deref()
            .and_then(|value| non_empty_string(Some(value)));
        update.changed_fields += usize::from(update.imdb_id.is_some());
    }
    if *source_kind == SourceKind::AdultVideo && !item.media_type.eq_ignore_ascii_case("adult") {
        update.media_type = Some("adult".to_string());
        update.changed_fields += 1;
    }

    update
}

const MAX_POSTER_BYTES: usize = 25 * 1024 * 1024;

fn poster_extension(url: &str, content_type: Option<&str>) -> &'static str {
    let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
    if content_type.contains("png") {
        return "png";
    }
    if content_type.contains("webp") {
        return "webp";
    }
    if content_type.contains("gif") {
        return "gif";
    }
    let url_path = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    if url_path.ends_with(".png") {
        "png"
    } else if url_path.ends_with(".webp") {
        "webp"
    } else if url_path.ends_with(".gif") {
        "gif"
    } else {
        "jpg"
    }
}

fn valid_poster_payload(content_type: Option<&str>, bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() > MAX_POSTER_BYTES {
        return false;
    }
    if content_type
        .map(|value| value.to_ascii_lowercase().starts_with("text/"))
        .unwrap_or(false)
    {
        return false;
    }
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
}

fn write_poster_sidecar_bytes(
    video_path: &str,
    extension: &str,
    bytes: &[u8],
) -> Result<String, String> {
    if !valid_poster_payload(None, bytes) {
        return Err("poster payload is empty, too large, or not a supported image".to_string());
    }
    let video = Path::new(video_path);
    let parent = video.parent().ok_or("video file has no parent directory")?;
    let stem = video
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("video file has no stem")?;
    let extension = match extension.to_ascii_lowercase().as_str() {
        "png" => "png",
        "webp" => "webp",
        "gif" => "gif",
        _ => "jpg",
    };
    let sidecar_path = parent.join(format!("{stem}-poster.{extension}"));
    if let Ok(existing_bytes) = std::fs::read(&sidecar_path) {
        if valid_poster_payload(None, &existing_bytes) {
            return Ok(sidecar_path.to_string_lossy().to_string());
        }
    }

    let temporary_path = parent.join(format!("{stem}-poster.{extension}.part"));
    let previous_path = parent.join(format!("{stem}-poster.{extension}.previous"));
    {
        let mut file = std::fs::File::create(&temporary_path)
            .map_err(|error| format!("poster create failed: {error}"))?;
        file.write_all(bytes)
            .map_err(|error| format!("poster write failed: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("poster sync failed: {error}"))?;
    }
    if sidecar_path.exists() {
        let _ = std::fs::remove_file(&previous_path);
        std::fs::rename(&sidecar_path, &previous_path)
            .map_err(|error| format!("poster backup before replacement failed: {error}"))?;
    }
    if let Err(error) = std::fs::rename(&temporary_path, &sidecar_path) {
        if previous_path.exists() {
            let _ = std::fs::rename(&previous_path, &sidecar_path);
        }
        return Err(format!("poster finalize failed: {error}"));
    }
    if previous_path.exists() {
        let _ = std::fs::remove_file(&previous_path);
    }
    Ok(sidecar_path.to_string_lossy().to_string())
}

/// Downloads a remote poster image URL to a verified local sidecar file next to the video.
pub(crate) async fn download_poster_to_sidecar(
    client: &reqwest::Client,
    url: &str,
    video_path: &str,
) -> Result<String, String> {
    let response = client
        .get(url)
        .header("User-Agent", "CinaVault/1.6.4")
        .send()
        .await
        .map_err(|error| format!("poster fetch failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("poster HTTP {}", response.status()));
    }
    if response
        .content_length()
        .map(|length| length > MAX_POSTER_BYTES as u64)
        .unwrap_or(false)
    {
        return Err(format!("poster exceeds {} bytes", MAX_POSTER_BYTES));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let extension = poster_extension(url, content_type.as_deref());
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("poster read failed: {error}"))?;
    if !valid_poster_payload(content_type.as_deref(), &bytes) {
        return Err("poster response was not a supported image".to_string());
    }
    write_poster_sidecar_bytes(video_path, extension, &bytes)
}

/// Writes a Kodi-compatible NFO sidecar XML file next to the video file.
fn write_nfo_sidecar(
    video_path: &str,
    title: &str,
    year: Option<i32>,
    overview: Option<&str>,
    rating: Option<f64>,
    genre: Option<&str>,
    tmdb_id: Option<&str>,
    imdb_id: Option<&str>,
    poster_path: Option<&str>,
) -> Result<(), String> {
    let video = Path::new(video_path);
    let parent = video.parent().ok_or("video file has no parent directory")?;
    let stem = video
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("video file has no stem")?;

    let nfo_path = parent.join(format!("{stem}.nfo"));

    let escaped_title = xml_escape(title);
    let year_str = year.map(|y| y.to_string()).unwrap_or_default();
    let overview_str = overview.map(xml_escape).unwrap_or_default();
    let rating_str = rating.map(|r| format!("{r:.1}")).unwrap_or_default();
    let genre_str = genre.map(xml_escape).unwrap_or_default();
    let tmdb_str = tmdb_id.unwrap_or_default();
    let imdb_str = imdb_id.unwrap_or_default();
    let thumb_str = poster_path.unwrap_or_default();

    let nfo_content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <movie>\n\
         \t<title>{escaped_title}</title>\n\
         \t<year>{year_str}</year>\n\
         \t<plot>{overview_str}</plot>\n\
         \t<rating>{rating_str}</rating>\n\
         \t<genre>{genre_str}</genre>\n\
         \t<uniqueid type=\"tmdb\">{tmdb_str}</uniqueid>\n\
         \t<uniqueid type=\"imdb\">{imdb_str}</uniqueid>\n\
         \t<thumb aspect=\"poster\">{thumb_str}</thumb>\n\
         </movie>\n"
    );

    std::fs::write(&nfo_path, nfo_content.as_bytes())
        .map_err(|e| format!("nfo write failed: {e}"))?;
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn should_update_optional_text(current: Option<&str>, incoming: Option<&str>) -> bool {
    let current_blank = current.map(|value| value.trim().is_empty()).unwrap_or(true);
    current_blank
        && incoming
            .map(|value| {
                let trimmed = value.trim();
                !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("N/A")
            })
            .unwrap_or(false)
}

fn is_video_library_item(item: &LibraryItemRecord) -> bool {
    matches!(
        item.media_type.to_lowercase().as_str(),
        "adult" | "movie" | "episode" | "video"
    ) && is_video_path(&item.file_path)
}

fn is_video_path(file_path: &str) -> bool {
    let lower = file_path.to_lowercase();
    [
        ".mp4", ".mkv", ".avi", ".mov", ".wmv", ".flv", ".webm", ".m4v", ".mpg", ".mpeg", ".ts",
        ".m2ts", ".vob", ".ogv", ".3gp", ".divx", ".rm", ".rmvb", ".asf",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension))
}

fn extract_embedded_title(file_path: &str) -> Option<String> {
    let mut cmd = std::process::Command::new("ffprobe");
    cmd.args([
        "-v",
        "error",
        "-show_entries",
        "format_tags=title:stream_tags=title",
        "-of",
        "default=nw=1:nk=1",
        file_path,
    ]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
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

fn looks_like_timestamp_only(value: &str) -> bool {
    let compact = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    Regex::new(r"^\d{8,14}$")
        .expect("timestamp regex should compile")
        .is_match(&compact)
}

fn is_low_quality_title(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("unknown")
        || looks_like_timestamp_only(trimmed)
        || trimmed.contains('_')
        || Regex::new(r"(?i)\b(480p|720p|1080p|2160p|x264|x265|webrip|bluray)\b")
            .expect("quality marker regex should compile")
            .is_match(trimmed)
}

fn clean_title_for_display(value: &str) -> Option<String> {
    let cleaned = value
        .replace(['\r', '\n', '\t'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|ch| ch == '.' || ch == ' ')
        .to_string();

    if cleaned.is_empty() || looks_like_timestamp_only(&cleaned) {
        None
    } else {
        Some(cleaned)
    }
}

fn push_unique_query(queries: &mut Vec<String>, value: Option<String>) {
    let Some(value) = value.and_then(|v| clean_title_for_display(&v)) else {
        return;
    };

    if !queries
        .iter()
        .any(|existing| strong_title_match(existing, &value))
    {
        queries.push(value);
    }
}

fn strong_title_match(left: &str, right: &str) -> bool {
    let left = canonical_title(left);
    let right = canonical_title(right);
    if left.is_empty() || right.is_empty() {
        return false;
    }
    if left == right {
        return true;
    }

    let left_tokens = left.split_whitespace().collect::<Vec<_>>();
    let right_tokens = right.split_whitespace().collect::<Vec<_>>();
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return false;
    }
    let common = left_tokens
        .iter()
        .filter(|token| right_tokens.contains(token))
        .count();
    let larger = left_tokens.len().max(right_tokens.len());
    common >= 2 && (common as f32 / larger as f32) >= 0.8
}

fn canonical_title(value: &str) -> String {
    let without_release_tokens = Regex::new(
        r"(?i)\b(480p|576p|720p|1080p|1440p|2160p|4k|8k|x264|x265|h264|h265|hevc|avc|aac|ac3|dts|web\s?dl|webrip|bluray|brrip|dvdrip|hdrip|proper|repack|extended|remux|group)\b",
    )
    .expect("release token regex should compile")
    .replace_all(value, " ")
    .to_string();

    without_release_tokens
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_windows_filename(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => ' ',
            _ => ch,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|ch| ch == '.' || ch == ' ')
        .to_string();

    let upper = cleaned.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.contains(&upper.as_str()) {
        String::new()
    } else {
        cleaned
    }
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(_)
                    if word
                        .chars()
                        .all(|ch| ch.is_ascii_uppercase() || !ch.is_ascii_alphabetic()) =>
                {
                    word.to_string()
                }
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn should_update_title(current_title: &str, normalized_title: &str, file_path: &str) -> bool {
    if normalized_title.is_empty() {
        return false;
    }
    let current = current_title.trim();
    if current.is_empty() || current.eq_ignore_ascii_case("unknown") {
        return true;
    }
    if titles_match(current, normalized_title) {
        return false;
    }
    if is_low_quality_title(current) {
        return true;
    }
    let filename_title = Path::new(file_path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().replace(['.', '_', '-'], " "))
        .unwrap_or_default();
    current.eq_ignore_ascii_case(filename_title.trim())
        || current.contains('.')
        || current.contains('_')
}

fn titles_match(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

fn push_sample(
    samples: &mut Vec<EnrichmentItemSummary>,
    id: i64,
    old_title: &str,
    new_title: &str,
    file_path: &str,
    action: &str,
) {
    if samples.len() >= 10 {
        return;
    }
    samples.push(EnrichmentItemSummary {
        id,
        old_title: old_title.to_string(),
        new_title: new_title.to_string(),
        file_path: file_path.to_string(),
        action: action.to_string(),
    });
}

#[derive(Debug, serde::Serialize)]
pub struct AdultMetadataReport {
    #[serde(rename = "type")]
    result_type: &'static str,
    status: &'static str,
    items_scanned: usize,
    metadata_items_enriched: usize,
    metadata_fields_updated: usize,
    posters_updated: usize,
    sidecars_written: usize,
    configured_adult_providers: Vec<String>,
    provider_count: usize,
    provider_errors: Vec<String>,
}

/// Dedicated command for gathering metadata specifically for adult media items.
/// Uses TPDB, StashDB, and any other configured adult providers.
#[tauri::command]
pub async fn gather_adult_metadata(
    state: State<'_, AppState>,
) -> Result<AdultMetadataReport, String> {
    let (items, provider_keys) = {
        let db = state.db.lock().map_err(|err| err.to_string())?;
        let mut stmt = db
            .conn
            .prepare(
                "SELECT mi.id, mi.title, mi.file_path, mi.media_type, mi.overview, \
                 mi.poster_path, mi.year, mi.rating, mi.genre, mi.tmdb_id, mi.imdb_id, \
                 ms.name, ms.path \
                 FROM media_items mi \
                 LEFT JOIN media_sources ms ON ms.id = mi.source_id \
                 WHERE mi.media_type = 'adult' \
                 ORDER BY date_added DESC",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LibraryItemRecord {
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
                    source_name: row.get(11)?,
                    source_path: row.get(12)?,
                })
            })
            .map_err(|err| err.to_string())?;
        let items = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())?;
        let provider_keys = load_provider_keys(&db)?;
        (items, provider_keys)
    };

    let configured_adult_providers: Vec<String> = [
        "tpdb",
        "stashdb",
        "pgma",
        "porn_site_nuxt",
        "iafd",
        "phoenixadult",
    ]
    .iter()
    .filter(|&&p| provider_keys.contains_key(p))
    .map(|p| p.to_string())
    .collect();

    let mut report = AdultMetadataReport {
        result_type: "adult_metadata_gather",
        status: "success",
        items_scanned: items.len(),
        metadata_items_enriched: 0,
        metadata_fields_updated: 0,
        posters_updated: 0,
        sidecars_written: 0,
        provider_count: configured_adult_providers.len(),
        configured_adult_providers: configured_adult_providers.clone(),
        provider_errors: Vec::new(),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|err| err.to_string())?;

    for item in items {
        if task_progress::stop_requested() {
            report
                .provider_errors
                .push("Stopped by user before processing the next item".to_string());
            break;
        }
        if !is_video_library_item(&item) {
            continue;
        }
        let source = Path::new(&item.file_path);
        if !source.exists() {
            continue;
        }

        let embedded_title = extract_embedded_title(&item.file_path);
        let queries = build_query_candidates(&item, embedded_title.clone());
        let remote_provider = resolve_provider_match(
            &client,
            &provider_keys,
            SourceKind::AdultVideo,
            &queries,
            &item.file_path,
            &mut report.provider_errors,
        )
        .await;

        let Some(provider) = remote_provider else {
            continue;
        };

        let mut update = build_metadata_update(&item, &provider, &SourceKind::AdultVideo);

        let poster_candidate = update.poster_path.clone().or_else(|| {
            item.poster_path
                .clone()
                .filter(|path| path.starts_with("http://") || path.starts_with("https://"))
        });
        if let Some(remote_url) = poster_candidate {
            if remote_url.starts_with("http://") || remote_url.starts_with("https://") {
                match download_poster_to_sidecar(&client, &remote_url, &item.file_path).await {
                    Ok(local_path) => {
                        if update.poster_path.is_none() {
                            update.changed_fields += 1;
                        }
                        update.poster_path = Some(local_path);
                        report.posters_updated += 1;
                    }
                    Err(err) => {
                        report
                            .provider_errors
                            .push(format!("poster/{}: {}", item.file_path, err));
                    }
                }
            }
        }

        if update.changed_fields > 0 || update.poster_path.is_some() {
            let nfo_title = update.title.as_deref().unwrap_or(&item.title);
            if let Err(err) = write_nfo_sidecar(
                &item.file_path,
                nfo_title,
                update.year.or(item.year),
                update.overview.as_deref().or(item.overview.as_deref()),
                update.rating.or(item.rating),
                update.genre.as_deref().or(item.genre.as_deref()),
                update.tmdb_id.as_deref().or(item.tmdb_id.as_deref()),
                update.imdb_id.as_deref().or(item.imdb_id.as_deref()),
                update.poster_path.as_deref(),
            ) {
                report
                    .provider_errors
                    .push(format!("nfo/{}: {}", item.file_path, err));
            } else {
                report.sidecars_written += 1;
            }
        }

        if update.changed_fields > 0 {
            let db = state.db.lock().map_err(|err| err.to_string())?;
            let _ = db.update_media_metadata_data(
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
            );
            report.metadata_items_enriched += 1;
            report.metadata_fields_updated += update.changed_fields;
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{
        build_metadata_update, build_query_candidates, classify_library_item,
        local_embedded_title_match, local_sidecar_artwork_match, normalize_filename_title,
        rename_confidence, safe_rename_target, valid_poster_payload, write_poster_sidecar_bytes,
        EnrichmentMode, LibraryItemRecord, ProviderMatch, RenameTarget, SourceKind,
    };
    use std::fs;

    fn sample_item(title: &str, file_path: &str, source_name: Option<&str>) -> LibraryItemRecord {
        LibraryItemRecord {
            id: 1,
            title: title.to_string(),
            file_path: file_path.to_string(),
            media_type: "movie".to_string(),
            overview: None,
            poster_path: None,
            year: None,
            rating: None,
            genre: None,
            tmdb_id: None,
            imdb_id: None,
            source_name: source_name.map(str::to_string),
            source_path: None,
        }
    }

    #[test]
    fn normalizes_common_release_filename_into_clean_title() {
        assert_eq!(
            normalize_filename_title("My.Movie.2024.1080p.x264-GROUP.mkv"),
            "My Movie (2024)"
        );
    }

    #[test]
    fn timestamp_only_filename_is_not_renamed_without_metadata() {
        assert_eq!(normalize_filename_title("2024-08-31_141904.mp4"), "");
    }

    #[test]
    fn classifies_adult_sources_from_source_name_hints() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Personal Vids X\Media\2024-08-31_141904.mp4",
            Some("Personal Vids X"),
        );

        assert_eq!(classify_library_item(&item), SourceKind::AdultVideo);
    }

    #[test]
    fn builds_query_candidates_from_embedded_title_before_filename() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Videos\2024-08-31_141904.mp4",
            Some("General Video"),
        );

        let queries = build_query_candidates(&item, Some("Actual Scene Title".to_string()));

        assert_eq!(
            queries.first().map(String::as_str),
            Some("Actual Scene Title")
        );
    }

    #[test]
    fn embedded_title_fallback_enriches_metadata_when_provider_lookup_misses() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Videos\2024-08-31_141904.mp4",
            Some("General Video"),
        );

        let provider = local_embedded_title_match(Some("Actual Scene Title"))
            .expect("embedded title should produce a local metadata match");
        let update = build_metadata_update(&item, &provider, &SourceKind::StandardVideo);

        assert_eq!(provider.title.as_deref(), Some("Actual Scene Title"));
        assert_eq!(update.title.as_deref(), Some("Actual Scene Title"));
        assert_eq!(update.changed_fields, 1);
    }

    #[test]
    fn acquired_poster_is_validated_and_atomically_written_as_a_sidecar() {
        let dir =
            std::env::temp_dir().join(format!("cinavault-poster-write-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let video = dir.join("Movie.mp4");
        fs::write(&video, b"video").expect("video should be created");
        let png = b"\x89PNG\r\n\x1a\nvalid-image-payload";

        assert!(valid_poster_payload(Some("image/png"), png));
        assert!(!valid_poster_payload(
            Some("text/html"),
            b"<html>error</html>"
        ));
        let poster = write_poster_sidecar_bytes(&video.to_string_lossy(), "png", png)
            .expect("poster sidecar should be written");

        assert!(poster.ends_with("Movie-poster.png"));
        assert_eq!(fs::read(&poster).unwrap(), png);
        assert!(!dir.join("Movie-poster.png.part").exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_existing_poster_sidecar_is_replaced_with_verified_image_bytes() {
        let dir =
            std::env::temp_dir().join(format!("cinavault-poster-replace-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let video = dir.join("Movie.mp4");
        let stale_poster = dir.join("Movie-poster.jpg");
        let jpg = b"\xFF\xD8\xFFverified-image-payload";
        fs::write(&video, b"video").expect("video should be created");
        fs::write(&stale_poster, b"<html>expired poster URL</html>")
            .expect("stale poster should be created");

        let poster = write_poster_sidecar_bytes(&video.to_string_lossy(), "jpg", jpg)
            .expect("verified poster should replace invalid sidecar");

        assert_eq!(poster, stale_poster.to_string_lossy());
        assert_eq!(fs::read(&poster).unwrap(), jpg);
        assert!(!dir.join("Movie-poster.jpg.part").exists());
        assert!(!dir.join("Movie-poster.jpg.previous").exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sidecar_artwork_fallback_populates_missing_posters() {
        let dir = std::env::temp_dir().join(format!("cinavault-sidecar-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let video = dir.join("Actual Scene Title.mp4");
        let poster = dir.join("Actual Scene Title-poster.jpg");
        fs::write(&video, b"video").expect("video should be created");
        fs::write(&poster, b"poster").expect("poster should be created");

        let item = sample_item(
            "Actual Scene Title",
            &video.to_string_lossy(),
            Some("General Video"),
        );

        let provider =
            local_sidecar_artwork_match(&item).expect("sidecar artwork should produce a match");
        let update = build_metadata_update(&item, &provider, &SourceKind::StandardVideo);

        assert_eq!(
            update.poster_path.as_deref(),
            Some(poster.to_string_lossy().as_ref())
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn allows_rename_for_balanced_confidence_when_provider_and_embedded_title_agree() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Videos\2024-08-31_141904.mp4",
            Some("General Video"),
        );
        let provider = ProviderMatch {
            title: Some("Actual Scene Title".to_string()),
            overview: None,
            poster_path: None,
            year: None,
            rating: None,
            genre: None,
            tmdb_id: Some("123".to_string()),
            imdb_id: None,
        };

        let confidence = rename_confidence(
            &item,
            Some("Actual Scene Title"),
            &provider,
            EnrichmentMode::MetadataAndRename,
        );

        assert!(confidence.allow_rename);
        assert_eq!(
            confidence.normalized_title.as_deref(),
            Some("Actual Scene Title")
        );
    }

    #[test]
    fn blocks_rename_when_provider_title_conflicts_with_embedded_title() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Videos\2024-08-31_141904.mp4",
            Some("General Video"),
        );
        let provider = ProviderMatch {
            title: Some("Different Provider Title".to_string()),
            overview: None,
            poster_path: None,
            year: None,
            rating: None,
            genre: None,
            tmdb_id: Some("123".to_string()),
            imdb_id: None,
        };

        let confidence = rename_confidence(
            &item,
            Some("Actual Scene Title"),
            &provider,
            EnrichmentMode::MetadataAndRename,
        );

        assert!(!confidence.allow_rename);
    }

    #[test]
    fn blocks_rename_when_stable_id_has_no_local_title_support() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Videos\2024-08-31_141904.mp4",
            Some("General Video"),
        );
        let provider = ProviderMatch {
            title: Some("Actual Scene Title".to_string()),
            overview: None,
            poster_path: None,
            year: None,
            rating: None,
            genre: None,
            tmdb_id: Some("123".to_string()),
            imdb_id: None,
        };

        let confidence =
            rename_confidence(&item, None, &provider, EnrichmentMode::MetadataAndRename);

        assert!(!confidence.allow_rename);
    }

    #[test]
    fn metadata_only_mode_updates_fields_without_allowing_rename() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Videos\2024-08-31_141904.mp4",
            Some("General Video"),
        );
        let provider = ProviderMatch {
            title: Some("Actual Scene Title".to_string()),
            overview: Some("Summary".to_string()),
            poster_path: None,
            year: Some(2024),
            rating: None,
            genre: None,
            tmdb_id: Some("123".to_string()),
            imdb_id: None,
        };

        let decision = rename_confidence(
            &item,
            Some("Actual Scene Title"),
            &provider,
            EnrichmentMode::MetadataOnly,
        );

        assert!(!decision.allow_rename);
    }

    #[test]
    fn safe_rename_target_blocks_existing_destination() {
        let dir =
            std::env::temp_dir().join(format!("cinavault-enrichment-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let source = dir.join("My.Movie.2024.1080p.mkv");
        let existing = dir.join("My Movie (2024).mkv");
        fs::write(&source, b"source").expect("source should be created");
        fs::write(&existing, b"existing").expect("existing target should be created");

        assert!(matches!(
            safe_rename_target(&source, "My Movie (2024)"),
            RenameTarget::Collision(_)
        ));

        let _ = fs::remove_dir_all(dir);
    }
}
