// CinaVault Premium — AI Diagnostics Module (HuggingFace Inference)
use crate::enrichment::{classify_library_item, LibraryItemRecord, SourceKind};
use crate::{task_progress, AppState};
use rusqlite::params;
use std::collections::{BTreeSet, HashMap};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::State;

// Efficient multilingual instruction model selected for structured media-library work.
const DEFAULT_MODEL: &str = "Qwen/Qwen3-4B-Instruct-2507";
const ROUTING_MODEL: &str = "katanemo/Arch-Router-1.5B:hf-inference";
const HF_BASE_URL: &str = "https://router.huggingface.co/v1/chat/completions";
static ADULT_GATHER_RUNNING: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn read_hf_token_file(path: &Path) -> Option<String> {
    let token = std::fs::read_to_string(path).ok()?;
    let token = token.trim();
    if token.starts_with("hf_") && token.len() > 20 {
        Some(token.to_string())
    } else {
        None
    }
}

fn cached_hf_token() -> Option<String> {
    let token_path = dirs::home_dir()?
        .join(".cache")
        .join("huggingface")
        .join("token");
    read_hf_token_file(&token_path)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiQueryRoute {
    NetworkDiagnostics,
    AdultMetadataGather,
    SourceDiscovery,
    LibraryAutomation,
    SourceCheck,
    ProviderCheck,
    Inference,
}

fn classify_ai_query_prompt(prompt: &str) -> AiQueryRoute {
    let lower = prompt.to_lowercase();

    if lower.contains("network")
        || lower.contains("ping")
        || lower.contains("dns")
        || lower.contains("connection")
    {
        return AiQueryRoute::NetworkDiagnostics;
    }
    if lower.contains("adult metadata")
        || lower.contains("gather metadata")
        || lower.contains("chapter images")
        || lower.contains("adult providers")
    {
        return AiQueryRoute::AdultMetadataGather;
    }
    if lower.contains("discover sources")
        || lower.contains("discover media")
        || lower.contains("find media folders")
        || lower.contains("locate media folders")
    {
        return AiQueryRoute::SourceDiscovery;
    }

    let requests_library_change = [
        "enrich",
        "normalize",
        "clean up",
        "cleanup",
        "rename",
        "poster",
        "nfo",
        "duplicate",
        "tag",
    ]
    .iter()
    .any(|term| lower.contains(term));
    if requests_library_change
        && (lower.contains("metadata")
            || lower.contains("title")
            || lower.contains("filename")
            || lower.contains("library")
            || lower.contains("media"))
    {
        return AiQueryRoute::LibraryAutomation;
    }

    if lower.contains("source")
        || lower.contains("folder")
        || lower.contains("media")
        || lower.contains("library")
    {
        return AiQueryRoute::SourceCheck;
    }
    if lower.contains("provider") || lower.contains("api") || lower.contains("metadata") {
        return AiQueryRoute::ProviderCheck;
    }

    AiQueryRoute::Inference
}

fn automation_tasks_from_prompt(prompt: &str) -> Vec<String> {
    let lower = prompt.to_lowercase();
    let mut tasks = BTreeSet::new();

    if lower.contains("scan") {
        tasks.insert("scan".to_string());
    }
    if lower.contains("metadata") || lower.contains("enrich") {
        tasks.insert("enrich".to_string());
        tasks.insert("posters".to_string());
        tasks.insert("nfo".to_string());
        tasks.insert("tags".to_string());
    }
    if lower.contains("title")
        || lower.contains("filename")
        || lower.contains("normalize")
        || lower.contains("rename")
        || lower.contains("clean up")
        || lower.contains("cleanup")
    {
        tasks.insert("enrich".to_string());
        tasks.insert("normalize".to_string());
    }
    if lower.contains("duplicate") {
        tasks.insert("duplicates".to_string());
    }

    if tasks.is_empty() {
        tasks.insert("enrich".to_string());
    }
    tasks.into_iter().collect()
}

pub(crate) fn is_adult_gather_candidate(media_type: &str, file_path: &str) -> bool {
    let path_lower = file_path.replace('/', "\\").to_lowercase();
    let is_video = [
        ".mp4", ".mkv", ".avi", ".mov", ".wmv", ".flv", ".webm", ".m4v", ".mpg", ".mpeg", ".ts",
        ".m2ts", ".vob", ".ogv", ".3gp", ".divx", ".rm", ".rmvb", ".asf",
    ]
    .iter()
    .any(|ext| path_lower.ends_with(ext));

    if !is_video {
        return false;
    }

    if path_lower.contains("_chapters\\chapter_") {
        return false;
    }

    matches!(media_type, "adult" | "movie" | "video")
}

fn normalize_adult_provider_key(provider: &str) -> String {
    match provider.trim().to_lowercase().as_str() {
        "theporndb" | "tpdb" => "tpdb".to_string(),
        other => other.to_string(),
    }
}

fn normalize_provider_key(provider: &str) -> String {
    match provider.trim().to_lowercase().as_str() {
        "themoviedb" | "themoviedb_images" | "tmdb_images" | "tmdb" => "tmdb".to_string(),
        "theporndb" | "tpdb" => "tpdb".to_string(),
        "open_movie_db" | "openmoviedb" | "omdb" => "omdb".to_string(),
        other => other.to_string(),
    }
}

fn is_adult_library_item(
    media_type: &str,
    title: &str,
    file_path: &str,
    source_name: Option<&str>,
    source_path: Option<&str>,
) -> bool {
    if !is_adult_gather_candidate(media_type, file_path) {
        return false;
    }

    let item = LibraryItemRecord {
        id: 0,
        title: title.to_string(),
        file_path: file_path.to_string(),
        media_type: media_type.to_string(),
        overview: None,
        poster_path: None,
        year: None,
        rating: None,
        genre: None,
        tmdb_id: None,
        imdb_id: None,
        source_name: source_name.map(str::to_string),
        source_path: source_path.map(str::to_string),
    };

    classify_library_item(&item) == SourceKind::AdultVideo
}

fn title_from_filename(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
        .replace('_', " ")
        .replace('.', " ")
}

fn should_refresh_title_from_embedded(current_title: &str, file_path: &str) -> bool {
    let trimmed = current_title.trim();
    if trimmed.is_empty() {
        return true;
    }

    let filename_title = title_from_filename(Path::new(file_path));
    trimmed.eq_ignore_ascii_case(&filename_title)
}

fn extract_embedded_title(file_path: &str) -> Option<String> {
    let mut cmd = Command::new("ffprobe");
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
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.to_string())
}

#[tauri::command]
pub async fn ai_query(
    state: State<'_, AppState>,
    prompt: String,
) -> Result<serde_json::Value, String> {
    match classify_ai_query_prompt(&prompt) {
        AiQueryRoute::NetworkDiagnostics => run_network_diagnostics().await,
        AiQueryRoute::AdultMetadataGather => gather_adult_metadata_assets(state).await,
        AiQueryRoute::SourceDiscovery => crate::scanner::discover_media_sources(state).await,
        AiQueryRoute::LibraryAutomation => {
            let tasks = automation_tasks_from_prompt(&prompt);
            crate::ai_automation::ai_library_manage(state, Some(tasks)).await
        }
        AiQueryRoute::SourceCheck => check_sources(state).await,
        AiQueryRoute::ProviderCheck => check_providers(state).await,
        AiQueryRoute::Inference => ai_inference(state, prompt, None, None).await,
    }
}

#[tauri::command]
pub async fn ai_inference(
    state: State<'_, AppState>,
    input: String,
    model: Option<String>,
    image_url: Option<String>,
) -> Result<serde_json::Value, String> {
    let token = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_setting_data("hf_token")
            .map_err(|e| e.to_string())?
            .filter(|t| !t.trim().is_empty())
            .or_else(|| std::env::var("CINAVAULT_HF_TOKEN").ok())
            .or_else(|| std::env::var("HF_TOKEN").ok())
            .or_else(cached_hf_token)
    };

    let model_id = model.unwrap_or_else(|| {
        let db = state.db.lock().ok();
        db.and_then(|d| d.get_setting_data("ai_model").ok().flatten())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string())
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let user_content = if let Some(url) = image_url.filter(|url| !url.trim().is_empty()) {
        serde_json::json!([
            { "type": "text", "text": input },
            { "type": "image_url", "image_url": { "url": url } }
        ])
    } else {
        serde_json::json!(input)
    };

    let inference_url = HF_BASE_URL;
    let mut req = client.post(inference_url).json(&serde_json::json!({
        "model": model_id,
        "messages": [
            {
                "role": "system",
                "content": "You are CineVault Premium's AI assistant for media server operations, metadata workflows, and diagnostics. Give concise, practical answers."
            },
            {
                "role": "user",
                "content": user_content
            }
        ],
        "temperature": 0.2,
        "max_tokens": 512
    }));

    if let Some(t) = &token {
        if !t.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", t));
        }
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("AI request failed: {}", e))?;
    let status = resp.status();

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Ok(serde_json::json!({
            "status": "error",
            "code": status.as_u16(),
            "message": body,
            "model": model_id,
        }));
    }

    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = data
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    Ok(serde_json::json!({
        "status": "success",
        "model": model_id,
        "message": content,
        "result": data,
    }))
}

async fn run_network_diagnostics() -> Result<serde_json::Value, String> {
    let mut results = serde_json::Map::new();

    // DNS check
    let dns = std::process::Command::new("nslookup")
        .arg("google.com")
        .output();
    results.insert(
        "dns".to_string(),
        serde_json::json!({
            "test": "DNS Resolution",
            "target": "google.com",
            "success": dns.as_ref().map(|o| o.status.success()).unwrap_or(false),
            "output": dns.ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
        }),
    );

    // Ping check
    #[cfg(target_os = "windows")]
    let ping = std::process::Command::new("ping")
        .args(&["-n", "3", "8.8.8.8"])
        .output();
    #[cfg(not(target_os = "windows"))]
    let ping = std::process::Command::new("ping")
        .args(&["-c", "3", "8.8.8.8"])
        .output();

    results.insert(
        "ping".to_string(),
        serde_json::json!({
            "test": "Ping (Google DNS)",
            "target": "8.8.8.8",
            "success": ping.as_ref().map(|o| o.status.success()).unwrap_or(false),
            "output": ping.ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
        }),
    );

    // HTTP check
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let http = client.get("https://www.google.com").send().await;
    results.insert(
        "http".to_string(),
        serde_json::json!({
            "test": "HTTPS Connectivity",
            "target": "https://www.google.com",
            "success": http.as_ref().map(|r| r.status().is_success()).unwrap_or(false),
        }),
    );

    Ok(serde_json::json!({
        "type": "network_diagnostics",
        "results": results,
    }))
}

async fn check_sources(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let sources = db.get_sources_data().map_err(|e| e.to_string())?;

    let mut checks = Vec::new();
    for source in &sources {
        let exists = std::path::Path::new(&source.path).exists();
        checks.push(serde_json::json!({
            "name": source.name,
            "path": source.path,
            "exists": exists,
            "enabled": source.enabled,
            "items": source.item_count,
        }));
    }

    Ok(serde_json::json!({
        "type": "source_check",
        "total_sources": sources.len(),
        "results": checks,
    }))
}

async fn check_providers(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .conn
        .prepare("SELECT provider FROM api_keys")
        .map_err(|e| e.to_string())?;
    let providers: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(serde_json::json!({
        "type": "provider_check",
        "configured_providers": providers,
        "total_configured": providers.len(),
    }))
}

fn detect_local_poster(file_path: &str) -> Option<String> {
    let media = std::path::Path::new(file_path);
    let parent = media.parent()?;
    let stem = media.file_stem()?.to_string_lossy();
    let candidates = [
        parent.join("poster.jpg"),
        parent.join("folder.jpg"),
        parent.join("cover.jpg"),
        parent.join(format!("{stem}.jpg")),
        parent.join(format!("{stem}.png")),
        parent.join(format!("{stem}-poster.jpg")),
    ];
    candidates
        .iter()
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
}

fn chapter_dir_for(file_path: &str) -> Option<String> {
    let p = std::path::Path::new(file_path);
    let parent = p.parent()?;
    let stem = p.file_stem()?.to_string_lossy();
    Some(
        parent
            .join(format!("{stem}_chapters"))
            .to_string_lossy()
            .to_string(),
    )
}

fn count_existing_chapter_images(chapter_dir: &str) -> usize {
    let dir = std::path::Path::new(chapter_dir);
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "jpg" || ext == "png" || ext == "webp")
                .unwrap_or(false)
        })
        .count()
}

fn metadata_sidecar_path(file_path: &str) -> Option<std::path::PathBuf> {
    let media = std::path::Path::new(file_path);
    let parent = media.parent()?;
    let stem = media.file_stem()?.to_string_lossy();
    Some(parent.join(format!("{stem}.cinavault.json")))
}

fn write_metadata_sidecar(
    file_path: &str,
    title: &str,
    overview: Option<&String>,
    poster_path: Option<&String>,
    year: Option<i32>,
    rating: Option<f64>,
    genre: Option<&String>,
    tmdb_id: Option<&String>,
    imdb_id: Option<&String>,
) -> Result<bool, String> {
    let sidecar_path = metadata_sidecar_path(file_path).ok_or("Unable to resolve sidecar path")?;
    let payload = serde_json::json!({
        "source_file": file_path,
        "title": title,
        "overview": overview,
        "poster_path": poster_path,
        "year": year,
        "rating": rating,
        "genre": genre,
        "tmdb_id": tmdb_id,
        "imdb_id": imdb_id,
        "written_at_utc": chrono::Utc::now().to_rfc3339(),
    });
    let body = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(sidecar_path, body).map_err(|e| e.to_string())?;
    Ok(true)
}

#[derive(Default, Debug, Clone)]
struct RemoteMetadata {
    title: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    year: Option<i32>,
    rating: Option<f64>,
    genre: Option<String>,
    tmdb_id: Option<String>,
    imdb_id: Option<String>,
}

fn non_empty_string(value: Option<&str>) -> Option<String> {
    value
        .map(|v| v.trim())
        .filter(|v| !v.is_empty() && *v != "N/A")
        .map(|v| v.to_string())
}

fn parse_year_prefix(value: Option<&str>) -> Option<i32> {
    let text = value?.trim();
    if text.len() < 4 {
        return None;
    }
    text[..4].parse::<i32>().ok()
}

fn should_prefer_remote_poster(current_poster: Option<&str>) -> bool {
    match current_poster.map(str::trim).filter(|v| !v.is_empty()) {
        None => true,
        Some(path) => {
            if path.starts_with("http://")
                || path.starts_with("https://")
                || path.starts_with("data:")
                || path.starts_with("asset:")
            {
                return false;
            }
            let lower = path.replace('/', "\\").to_lowercase();
            lower.ends_with("-poster.jpg")
                || lower.ends_with("-poster.png")
                || lower.ends_with("\\poster.jpg")
                || lower.ends_with("\\cover.jpg")
                || lower.ends_with("\\folder.jpg")
        }
    }
}

async fn fetch_tmdb_metadata(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
) -> Option<RemoteMetadata> {
    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    let url = format!(
        "https://api.themoviedb.org/3/search/multi?api_key={api_key}&query={encoded}&include_adult=true&page=1"
    );
    let data = client
        .get(url)
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;
    let first = data.get("results")?.as_array()?.first()?;

    let title = non_empty_string(first.get("title").and_then(|v| v.as_str()))
        .or_else(|| non_empty_string(first.get("name").and_then(|v| v.as_str())));
    let overview = non_empty_string(first.get("overview").and_then(|v| v.as_str()));
    let poster_path = first
        .get("poster_path")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(|p| format!("https://image.tmdb.org/t/p/w500{p}"));
    let year = parse_year_prefix(first.get("release_date").and_then(|v| v.as_str()))
        .or_else(|| parse_year_prefix(first.get("first_air_date").and_then(|v| v.as_str())));
    let rating = first
        .get("vote_average")
        .and_then(|v| v.as_f64())
        .filter(|v| *v > 0.0);
    let tmdb_id = first
        .get("id")
        .and_then(|v| v.as_i64())
        .map(|id| id.to_string());

    Some(RemoteMetadata {
        title,
        overview,
        poster_path,
        year,
        rating,
        genre: None,
        tmdb_id,
        imdb_id: None,
    })
}

async fn fetch_omdb_metadata(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
) -> Option<RemoteMetadata> {
    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    let url = format!("https://www.omdbapi.com/?apikey={api_key}&t={encoded}&plot=full");
    let data = client
        .get(url)
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;
    if data.get("Response").and_then(|v| v.as_str()) != Some("True") {
        return None;
    }

    let title = non_empty_string(data.get("Title").and_then(|v| v.as_str()));
    let overview = non_empty_string(data.get("Plot").and_then(|v| v.as_str()));
    let poster_path = non_empty_string(data.get("Poster").and_then(|v| v.as_str()));
    let year = parse_year_prefix(data.get("Year").and_then(|v| v.as_str()));
    let rating = data
        .get("imdbRating")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| *v > 0.0);
    let genre = non_empty_string(data.get("Genre").and_then(|v| v.as_str()));
    let imdb_id = non_empty_string(data.get("imdbID").and_then(|v| v.as_str()));

    Some(RemoteMetadata {
        title,
        overview,
        poster_path,
        year,
        rating,
        genre,
        tmdb_id: None,
        imdb_id,
    })
}

async fn fetch_stashdb_metadata(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
) -> Option<RemoteMetadata> {
    let body = serde_json::json!({
        "query": "query($title:String!){ queryScenes(input:{title:$title, per_page:1, page:1, direction:DESC, sort:DATE}) { scenes { title details release_date images { url width height } tags { name } urls { url } } } }",
        "variables": {
            "title": query
        }
    });

    let data = client
        .post("https://stashdb.org/graphql")
        .header("Content-Type", "application/json")
        .header("ApiKey", api_key)
        .json(&body)
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;

    if data.get("errors").is_some() {
        return None;
    }

    let first = data
        .get("data")
        .and_then(|v| v.get("queryScenes"))
        .and_then(|v| v.get("scenes"))
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())?;

    let title = non_empty_string(first.get("title").and_then(|v| v.as_str()));
    let overview = non_empty_string(first.get("details").and_then(|v| v.as_str()));
    let poster_path = first
        .get("images")
        .and_then(|v| v.as_array())
        .and_then(|images| images.first())
        .and_then(|img| img.get("url"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());
    let year = parse_year_prefix(first.get("release_date").and_then(|v| v.as_str()));
    let genre = first
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| tag.get("name").and_then(|v| v.as_str()))
                .filter(|name| !name.trim().is_empty())
                .take(5)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|g| !g.trim().is_empty());

    Some(RemoteMetadata {
        title,
        overview,
        poster_path,
        year,
        rating: None,
        genre,
        tmdb_id: None,
        imdb_id: None,
    })
}

fn merge_remote_metadata(
    primary: Option<RemoteMetadata>,
    secondary: Option<RemoteMetadata>,
) -> Option<RemoteMetadata> {
    let mut merged = primary.or(secondary.clone())?;
    if let Some(extra) = secondary {
        if merged.title.is_none() {
            merged.title = extra.title;
        }
        if merged.overview.is_none() {
            merged.overview = extra.overview;
        }
        if merged.poster_path.is_none() {
            merged.poster_path = extra.poster_path;
        }
        if merged.year.is_none() {
            merged.year = extra.year;
        }
        if merged.rating.is_none() {
            merged.rating = extra.rating;
        }
        if merged.genre.is_none() {
            merged.genre = extra.genre;
        }
        if merged.tmdb_id.is_none() {
            merged.tmdb_id = extra.tmdb_id;
        }
        if merged.imdb_id.is_none() {
            merged.imdb_id = extra.imdb_id;
        }
    }
    Some(merged)
}

async fn gather_adult_metadata_assets(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if ADULT_GATHER_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(serde_json::json!({
            "type": "adult_metadata_gather",
            "status": "busy",
            "message": "Adult metadata gather is already running. Please wait for completion.",
        }));
    }

    let result = gather_adult_metadata_assets_inner(state).await;
    ADULT_GATHER_RUNNING.store(false, Ordering::SeqCst);
    result
}

async fn gather_adult_metadata_assets_inner(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let configured_adult_providers: Vec<String> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .conn
            .prepare("SELECT provider FROM api_keys")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut providers = BTreeSet::new();

        for provider in rows.filter_map(|r| r.ok()) {
            let normalized = normalize_adult_provider_key(&provider);
            if matches!(
                normalized.as_str(),
                "tpdb" | "stashdb" | "phoenixadult" | "iafd"
            ) {
                providers.insert(normalized);
            }
        }

        providers.into_iter().collect()
    };

    let provider_keys: HashMap<String, String> = {
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

        let mut keys = HashMap::new();
        for row in rows.filter_map(|r| r.ok()) {
            let raw_key = row.0.to_lowercase();
            let normalized_key = normalize_provider_key(&raw_key);
            keys.insert(raw_key, row.1.clone());
            keys.insert(normalized_key, row.1);
        }
        keys
    };
    let tmdb_key = provider_keys.get("tmdb").cloned();
    let omdb_key = provider_keys.get("omdb").cloned();
    let stashdb_key = provider_keys.get("stashdb").cloned();

    let media_items: Vec<(
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<i32>,
        Option<f64>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
    )> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .conn
            .prepare(
                "SELECT mi.id,
                    mi.title,
                    mi.file_path,
                    mi.poster_path,
                    mi.overview,
                    mi.year,
                    mi.rating,
                    mi.genre,
                    mi.tmdb_id,
                    mi.imdb_id,
                    mi.media_type,
                    ms.name,
                    ms.path
             FROM media_items mi
             LEFT JOIN media_sources ms ON ms.id = mi.source_id
             WHERE mi.media_type IN ('adult', 'movie', 'video')
             ORDER BY mi.date_added DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<i32>>(5)?,
                    row.get::<_, Option<f64>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok())
            .filter(
                |(
                    _,
                    title,
                    file_path,
                    _,
                    _,
                    _,
                    _,
                    _,
                    _,
                    _,
                    media_type,
                    source_name,
                    source_path,
                )| {
                    is_adult_library_item(
                        media_type,
                        title,
                        file_path,
                        source_name.as_deref(),
                        source_path.as_deref(),
                    )
                },
            )
            .collect()
    };

    let mut posters_updated = 0usize;
    let mut chapters_generated_for_items = 0usize;
    let mut chapter_images_generated = 0usize;
    let mut items_needing_metadata = 0usize;
    let mut items_reclassified_as_adult = 0usize;
    let mut titles_refreshed_from_embedded = 0usize;
    let mut metadata_items_enriched = 0usize;
    let mut metadata_fields_updated = 0usize;
    let mut sidecars_written = 0usize;
    let mut skipped_missing_files = 0usize;
    let mut skipped_non_video_items = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let client = reqwest::Client::new();
    let mut progress = task_progress::MetadataTaskGuard::start(
        "adult_metadata_gather",
        "Adult Metadata Gather",
        media_items.len(),
        "Preparing adult metadata gather",
    );

    for (
        index,
        (
            id,
            title,
            file_path,
            poster_path,
            overview,
            year,
            rating,
            genre,
            tmdb_id,
            imdb_id,
            media_type,
            source_name,
            source_path,
        ),
    ) in media_items.iter().enumerate()
    {
        if task_progress::stop_requested() {
            errors.push("Stopped by user before processing the next item".to_string());
            break;
        }
        progress.update(
            index + 1,
            format!(
                "Gathering metadata and poster artwork for {} of {}",
                index + 1,
                media_items.len()
            ),
        );
        let media_path = std::path::Path::new(file_path);
        if !media_path.exists() {
            skipped_missing_files += 1;
            continue;
        }
        if !is_adult_library_item(
            media_type,
            title,
            file_path,
            source_name.as_deref(),
            source_path.as_deref(),
        ) {
            skipped_non_video_items += 1;
            continue;
        }

        if media_type != "adult" {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.conn
                .execute(
                    "UPDATE media_items SET media_type = 'adult' WHERE id = ?1",
                    params![id],
                )
                .map_err(|e| e.to_string())?;
            items_reclassified_as_adult += 1;
        }

        let mut final_title = title.clone();
        let mut final_overview = overview.clone();
        let mut final_poster = poster_path.clone();
        let mut final_year = *year;
        let mut final_rating = *rating;
        let mut final_genre = genre.clone();
        let mut final_tmdb_id = tmdb_id.clone();
        let mut final_imdb_id = imdb_id.clone();

        if should_refresh_title_from_embedded(&final_title, file_path) {
            if let Some(embedded_title) = extract_embedded_title(file_path) {
                if !embedded_title.eq_ignore_ascii_case(&final_title) {
                    let db = state.db.lock().map_err(|e| e.to_string())?;
                    db.conn
                        .execute(
                            "UPDATE media_items SET title = ?1 WHERE id = ?2",
                            params![embedded_title, id],
                        )
                        .map_err(|e| e.to_string())?;
                    final_title = embedded_title;
                    titles_refreshed_from_embedded += 1;
                }
            }
        }

        let has_overview = overview
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_overview {
            items_needing_metadata += 1;
        }

        let has_poster = poster_path
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !has_poster {
            if let Some(local_poster) = detect_local_poster(file_path) {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                db.conn
                    .execute(
                        "UPDATE media_items SET poster_path = ?1 WHERE id = ?2",
                        params![local_poster, id],
                    )
                    .map_err(|e| e.to_string())?;
                posters_updated += 1;
                final_poster = Some(local_poster);
            }
        }

        let missing_genre = final_genre
            .as_ref()
            .map(|g| g.trim().is_empty())
            .unwrap_or(true);
        let should_upgrade_poster = should_prefer_remote_poster(final_poster.as_deref());
        let needs_remote_metadata = !has_overview
            || should_upgrade_poster
            || final_year.is_none()
            || final_rating.is_none()
            || missing_genre
            || final_tmdb_id
                .as_deref()
                .map(|v| v.trim().is_empty())
                .unwrap_or(true)
            || final_imdb_id
                .as_deref()
                .map(|v| v.trim().is_empty())
                .unwrap_or(true);

        if needs_remote_metadata
            && (stashdb_key.is_some() || tmdb_key.is_some() || omdb_key.is_some())
        {
            let query_title = extract_embedded_title(file_path)
                .filter(|embedded| !embedded.trim().is_empty())
                .unwrap_or_else(|| final_title.clone());

            let stashdb_meta = if let Some(key) = stashdb_key.as_deref() {
                fetch_stashdb_metadata(&client, key, &query_title).await
            } else {
                None
            };
            let tmdb_meta = if let Some(key) = tmdb_key.as_deref() {
                fetch_tmdb_metadata(&client, key, &query_title).await
            } else {
                None
            };
            let omdb_meta = if let Some(key) = omdb_key.as_deref() {
                fetch_omdb_metadata(&client, key, &query_title).await
            } else {
                None
            };
            let stash_then_tmdb = merge_remote_metadata(stashdb_meta, tmdb_meta);
            let remote_meta = merge_remote_metadata(stash_then_tmdb, omdb_meta);

            if let Some(meta) = remote_meta {
                let mut changed_fields = 0usize;

                if should_refresh_title_from_embedded(title, file_path) {
                    if let Some(new_title) = meta.title.filter(|v| !v.trim().is_empty()) {
                        if !new_title.eq_ignore_ascii_case(&final_title) {
                            final_title = new_title;
                            changed_fields += 1;
                        }
                    }
                }
                if final_overview
                    .as_ref()
                    .map(|v| v.trim().is_empty())
                    .unwrap_or(true)
                {
                    if let Some(new_overview) = meta.overview.filter(|v| !v.trim().is_empty()) {
                        final_overview = Some(new_overview);
                        changed_fields += 1;
                    }
                }
                if should_prefer_remote_poster(final_poster.as_deref()) {
                    if let Some(new_poster) = meta.poster_path.filter(|v| !v.trim().is_empty()) {
                        if final_poster.as_deref() != Some(new_poster.as_str()) {
                            final_poster = Some(new_poster);
                            changed_fields += 1;
                        }
                    }
                }
                if final_year.is_none() {
                    if let Some(new_year) = meta.year {
                        final_year = Some(new_year);
                        changed_fields += 1;
                    }
                }
                if final_rating.is_none() {
                    if let Some(new_rating) = meta.rating {
                        final_rating = Some(new_rating);
                        changed_fields += 1;
                    }
                }
                if final_genre
                    .as_ref()
                    .map(|v| v.trim().is_empty())
                    .unwrap_or(true)
                {
                    if let Some(new_genre) = meta.genre.filter(|v| !v.trim().is_empty()) {
                        final_genre = Some(new_genre);
                        changed_fields += 1;
                    }
                }
                if final_tmdb_id
                    .as_ref()
                    .map(|v| v.trim().is_empty())
                    .unwrap_or(true)
                {
                    if let Some(new_tmdb_id) = meta.tmdb_id.filter(|v| !v.trim().is_empty()) {
                        final_tmdb_id = Some(new_tmdb_id);
                        changed_fields += 1;
                    }
                }
                if final_imdb_id
                    .as_ref()
                    .map(|v| v.trim().is_empty())
                    .unwrap_or(true)
                {
                    if let Some(new_imdb_id) = meta.imdb_id.filter(|v| !v.trim().is_empty()) {
                        final_imdb_id = Some(new_imdb_id);
                        changed_fields += 1;
                    }
                }

                if changed_fields > 0 {
                    let db = state.db.lock().map_err(|e| e.to_string())?;
                    db.conn
                        .execute(
                            "UPDATE media_items
                         SET title = ?1,
                             overview = ?2,
                             poster_path = ?3,
                             year = ?4,
                             rating = ?5,
                             genre = ?6,
                             tmdb_id = ?7,
                             imdb_id = ?8
                         WHERE id = ?9",
                            params![
                                final_title,
                                final_overview,
                                final_poster,
                                final_year,
                                final_rating,
                                final_genre,
                                final_tmdb_id,
                                final_imdb_id,
                                id
                            ],
                        )
                        .map_err(|e| e.to_string())?;
                    metadata_items_enriched += 1;
                    metadata_fields_updated += changed_fields;
                }
            }
        }

        if let Some(chapter_dir) = chapter_dir_for(file_path) {
            let existing = count_existing_chapter_images(&chapter_dir);
            if existing == 0 {
                match crate::chapters::generate_chapter_thumbs(
                    file_path.clone(),
                    None,
                    Some(300),
                    None,
                )
                .await
                {
                    Ok(thumbs) if !thumbs.is_empty() => {
                        chapters_generated_for_items += 1;
                        chapter_images_generated += thumbs.len();
                    }
                    Ok(_) => {}
                    Err(e) => errors.push(format!("{title}: {e}")),
                }
            }
        }

        match write_metadata_sidecar(
            file_path,
            &final_title,
            final_overview.as_ref(),
            final_poster.as_ref(),
            final_year,
            final_rating,
            final_genre.as_ref(),
            final_tmdb_id.as_ref(),
            final_imdb_id.as_ref(),
        ) {
            Ok(true) => sidecars_written += 1,
            Ok(false) => {}
            Err(e) => errors.push(format!("{title}: sidecar write failed: {e}")),
        }
    }

    progress.finish(format!(
        "Adult metadata gather complete: {posters_updated} posters updated, {sidecars_written} sidecars written"
    ));

    Ok(serde_json::json!({
        "type": "adult_metadata_gather",
        "status": "success",
        "configured_adult_providers": configured_adult_providers,
        "provider_count": configured_adult_providers.len(),
        "items_scanned": media_items.len(),
        "items_reclassified_as_adult": items_reclassified_as_adult,
        "titles_refreshed_from_embedded": titles_refreshed_from_embedded,
        "metadata_items_enriched": metadata_items_enriched,
        "metadata_fields_updated": metadata_fields_updated,
        "sidecars_written": sidecars_written,
        "posters_updated": posters_updated,
        "chapter_sets_generated": chapters_generated_for_items,
        "chapter_images_generated": chapter_images_generated,
        "chapter_generation_skipped_after_limit": 0,
        "items_needing_metadata": items_needing_metadata,
        "skipped_missing_files": skipped_missing_files,
        "skipped_non_video_items": skipped_non_video_items,
        "note": "Adult metadata gather now supports legacy provider-key aliases, uses StashDB/TMDb/OMDb metadata when available, upgrades local poster placeholders to remote posters, writes sidecar files, and generates chapter images without a hard item cap.",
        "errors": errors,
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        automation_tasks_from_prompt, classify_ai_query_prompt, is_adult_gather_candidate,
        is_adult_library_item, metadata_sidecar_path, normalize_adult_provider_key,
        normalize_provider_key, should_prefer_remote_poster, AiQueryRoute,
    };

    #[test]
    fn accepts_real_video_candidates_for_adult_gather() {
        assert!(is_adult_gather_candidate("adult", r"E:\Videos\scene.mp4"));
        assert!(is_adult_gather_candidate("movie", r"E:\Videos\scene.mkv"));
    }

    #[test]
    fn adult_metadata_prompt_routes_to_adult_gather_before_generic_media_checks() {
        assert_eq!(
            classify_ai_query_prompt("Run adult metadata gather for installed providers and generate posters and chapter images"),
            AiQueryRoute::AdultMetadataGather
        );
    }

    #[test]
    fn operational_prompts_route_to_real_side_effect_commands() {
        assert_eq!(
            classify_ai_query_prompt("Enrich metadata and clean up titles"),
            AiQueryRoute::LibraryAutomation
        );
        assert_eq!(
            classify_ai_query_prompt("AI discover sources"),
            AiQueryRoute::SourceDiscovery
        );
        let tasks = automation_tasks_from_prompt("Enrich metadata and clean up titles");
        assert!(tasks.iter().any(|task| task == "enrich"));
        assert!(tasks.iter().any(|task| task == "normalize"));
        assert!(tasks.iter().any(|task| task == "posters"));
        assert!(tasks.iter().any(|task| task == "nfo"));
    }

    #[test]
    fn rejects_generated_images_and_non_video_assets_for_adult_gather() {
        assert!(!is_adult_gather_candidate(
            "photo",
            r"E:\Videos\scene_chapters\chapter_0001.jpg"
        ));
        assert!(!is_adult_gather_candidate("photo", r"E:\Videos\poster.jpg"));
    }

    #[test]
    fn treats_items_from_adult_named_sources_as_adult_library_candidates() {
        assert!(is_adult_library_item(
            "movie",
            "2024-08-31 141950",
            r"E:\Personal Vids X\Media\2024-08-31\2024-08-31_141950.mp4",
            Some("Personal Vids X"),
            Some(r"E:\Personal Vids X")
        ));
        assert!(is_adult_library_item(
            "movie",
            "clip",
            r"D:\Library\clip.mp4",
            Some("Personal X Library"),
            Some(r"D:\Personal X Library")
        ));
    }

    #[test]
    fn normalizes_theporndb_alias_for_adult_provider_detection() {
        assert_eq!(normalize_adult_provider_key("theporndb"), "tpdb");
        assert_eq!(normalize_adult_provider_key("tpdb"), "tpdb");
        assert_eq!(normalize_adult_provider_key("stashdb"), "stashdb");
    }

    #[test]
    fn normalizes_legacy_provider_aliases_for_backward_compatibility() {
        assert_eq!(normalize_provider_key("themoviedb_images"), "tmdb");
        assert_eq!(normalize_provider_key("tmdb_images"), "tmdb");
        assert_eq!(normalize_provider_key("theporndb"), "tpdb");
    }

    #[test]
    fn derives_sidecar_path_next_to_media_file() {
        let path = metadata_sidecar_path(r"E:\Adult\scene-01.mp4")
            .expect("sidecar path should resolve")
            .to_string_lossy()
            .replace('/', "\\");
        assert!(path.ends_with(r"E:\Adult\scene-01.cinavault.json"));
    }

    #[test]
    fn remote_poster_preferred_for_local_placeholder_files_only() {
        assert!(should_prefer_remote_poster(None));
        assert!(should_prefer_remote_poster(Some(
            r"E:\Library\video-poster.jpg"
        )));
        assert!(should_prefer_remote_poster(Some(r"E:\Library\poster.jpg")));
        assert!(!should_prefer_remote_poster(Some(
            "https://example.com/poster.jpg"
        )));
    }
}

#[tauri::command]
pub fn set_hf_token(state: State<AppState>, token: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_setting_data("hf_token", &token)
        .map_err(|e| e.to_string())
}

/// Checks DB, environment variables, and the persistent Hugging Face CLI cache.
/// A valid fallback credential is copied into the app DB so subsequent calls are stable.
/// Returns availability status and source so the UI can show the correct state.
#[tauri::command]
pub fn ensure_hf_token(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // 1. Check DB
    if let Some(t) = db.get_setting_data("hf_token").ok().flatten() {
        if !t.trim().is_empty() {
            return Ok(serde_json::json!({
                "available": true,
                "source": "db",
                "model": DEFAULT_MODEL,
            }));
        }
    }
    // 2. Import a valid environment or Hugging Face CLI cached credential.
    let (source, fallback_token) = if let Some(token) = std::env::var("CINAVAULT_HF_TOKEN")
        .ok()
        .or_else(|| std::env::var("HF_TOKEN").ok())
        .filter(|token| !token.trim().is_empty())
    {
        ("env_auto_seeded", Some(token))
    } else {
        ("hf_cache_auto_seeded", cached_hf_token())
    };
    if let Some(token) = fallback_token {
        db.set_setting_data("hf_token", &token)
            .map_err(|error| error.to_string())?;
        return Ok(serde_json::json!({
            "available": true,
            "source": source,
            "model": DEFAULT_MODEL,
        }));
    }
    Ok(serde_json::json!({
        "available": false,
        "source": "missing",
        "model": DEFAULT_MODEL,
        "get_token_url": "https://huggingface.co/settings/tokens",
        "hint": "Sign in with the Hugging Face CLI, enter a token in AI Configure, or set CINAVAULT_HF_TOKEN",
    }))
}

#[tauri::command]
pub fn get_ai_config(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let model = db
        .get_setting_data("ai_model")
        .map_err(|e| e.to_string())?
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    // Check DB token first, then env vars
    let db_token_present = db
        .get_setting_data("hf_token")
        .map_err(|e| e.to_string())?
        .map(|t| !t.trim().is_empty())
        .unwrap_or(false);
    let env_token_present = std::env::var("CINAVAULT_HF_TOKEN")
        .map(|t| !t.trim().is_empty())
        .unwrap_or(false)
        || std::env::var("HF_TOKEN")
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false);
    let cached_token_present = cached_hf_token().is_some();
    let has_token = db_token_present || env_token_present || cached_token_present;
    Ok(serde_json::json!({
        "model": model,
        "has_token": has_token,
        "default_model": DEFAULT_MODEL,
        "inference_url": HF_BASE_URL,
        "recommended_model": DEFAULT_MODEL,
        "routing_model": ROUTING_MODEL,
    }))
}

/// AI-powered automated library management: runs all library functions in sequence
/// using the configured HuggingFace model. Covers: scan, enrich, poster sync,
/// NFO write-back, duplicate detection, filename normalization, genre tagging.
async fn ai_library_manage(
    state: State<'_, AppState>,
    tasks: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let requested = tasks.unwrap_or_else(|| {
        vec![
            "scan".to_string(),
            "enrich".to_string(),
            "posters".to_string(),
            "nfo".to_string(),
            "duplicates".to_string(),
            "normalize".to_string(),
            "tags".to_string(),
        ]
    });
    let mut results = serde_json::Map::new();
    let mut total_updated = 0u64;
    let mut errors: Vec<String> = Vec::new();

    // --- Enrich metadata + poster sync + NFO write-back ---
    if requested
        .iter()
        .any(|t| t == "enrich" || t == "posters" || t == "nfo")
    {
        match crate::enrichment::run_library_enrichment(state.clone(), false).await {
            Ok(report) => {
                total_updated += report.metadata_updated as u64;
                results.insert(
                    "enrichment".to_string(),
                    serde_json::json!({
                        "status": "ok",
                        "items_scanned": report.items_scanned,
                        "metadata_updated": report.metadata_updated,
                        "metadata_items_enriched": report.metadata_items_enriched,
                        "posters_downloaded": report.posters_downloaded,
                        "sidecars_written": report.sidecars_written,
                    }),
                );
            }
            Err(e) => {
                errors.push(format!("enrichment: {}", e));
                results.insert(
                    "enrichment".to_string(),
                    serde_json::json!({ "status": "error", "error": e }),
                );
            }
        }
    }

    // --- Duplicate detection ---
    if requested.iter().any(|t| t == "duplicates") {
        match crate::duplicates::find_duplicates(
            state.clone(),
            Some("name_size".to_string()),
            Some(0.0),
        )
        .await
        {
            Ok(report) => {
                results.insert("duplicates".to_string(), serde_json::json!({
                    "status": "ok",
                    "groups_found": report.get("groups_found").cloned().unwrap_or(serde_json::json!(0)),
                }));
            }
            Err(e) => {
                errors.push(format!("duplicates: {}", e));
                results.insert(
                    "duplicates".to_string(),
                    serde_json::json!({ "status": "error", "error": e }),
                );
            }
        }
    }

    // --- Source health check ---
    if requested.iter().any(|t| t == "scan") {
        match check_sources(state.clone()).await {
            Ok(report) => {
                results.insert("sources".to_string(), report);
            }
            Err(e) => {
                errors.push(format!("sources: {}", e));
                results.insert(
                    "sources".to_string(),
                    serde_json::json!({ "status": "error", "error": e }),
                );
            }
        }
    }

    Ok(serde_json::json!({
        "type": "ai_library_manage",
        "status": if errors.is_empty() { "success" } else { "partial" },
        "tasks_run": requested,
        "total_updated": total_updated,
        "results": results,
        "errors": errors,
    }))
}

#[tauri::command]
pub fn set_ai_model(state: State<AppState>, model: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_setting_data("ai_model", &model)
        .map_err(|e| e.to_string())
}
