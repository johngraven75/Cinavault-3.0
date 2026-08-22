// CinaVault Premium — Downloads Module (yt-dlp + ffmpeg)
use crate::AppState;
use regex::Regex;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::State;

static DOWNLOADING: AtomicBool = AtomicBool::new(false);
static CANCEL_DL: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct DownloadItem {
    pub id: Option<i64>,
    pub url: String,
    pub title: Option<String>,
    pub status: String,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrawlLink {
    pub url: String,
    pub kind: String,
    pub source: String,
}

fn default_download_dir() -> String {
    dirs::download_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Downloads"))
        .to_string_lossy()
        .to_string()
}

fn media_kind(url: &str) -> String {
    let lower = url.to_lowercase();
    if lower.ends_with(".m3u8") || lower.contains(".m3u8?") {
        "hls".into()
    } else if lower.ends_with(".mpd") || lower.contains(".mpd?") {
        "dash".into()
    } else if [
        ".mp4", ".m4v", ".mkv", ".webm", ".mov", ".avi", ".wmv", ".flv", ".ts", ".mts", ".m2ts",
        ".3gp", ".ogv",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext) || lower.contains(&format!("{}?", ext)))
    {
        "video".into()
    } else if [
        ".mp3", ".m4a", ".aac", ".flac", ".wav", ".ogg", ".opus", ".wma", ".aiff", ".alac",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext) || lower.contains(&format!("{}?", ext)))
    {
        "audio".into()
    } else if [".srt", ".vtt", ".ass", ".ssa", ".ttml", ".dfxp"]
        .iter()
        .any(|ext| lower.ends_with(ext) || lower.contains(&format!("{}?", ext)))
    {
        "subtitle".into()
    } else if [
        ".jpg", ".jpeg", ".png", ".webp", ".gif", ".bmp", ".avif", ".heic",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext) || lower.contains(&format!("{}?", ext)))
    {
        "image".into()
    } else {
        "page".into()
    }
}

fn looks_like_captcha_or_challenge(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("captcha")
        || lower.contains("recaptcha")
        || lower.contains("hcaptcha")
        || lower.contains("cf-challenge")
        || lower.contains("cloudflare")
        || lower.contains("verify you are human")
        || lower.contains("are you a human")
        || lower.contains("access denied")
}

fn build_yt_dlp_args(
    url: &str,
    out_dir: &str,
    fmt: &str,
    include_playlist: bool,
    cookies_file: Option<&str>,
) -> Vec<String> {
    let kind = media_kind(url);
    let mut args = vec![
        "-f".into(),
        fmt.into(),
        "--merge-output-format".into(),
        "mp4".into(),
        "--remux-video".into(),
        "mp4/mkv".into(),
        "--embed-metadata".into(),
        "--embed-thumbnail".into(),
        "--write-thumbnail".into(),
        "--write-sub".into(),
        "--write-auto-sub".into(),
        "--sub-langs".into(),
        "all,-live_chat".into(),
        "--convert-subs".into(),
        "srt".into(),
        "--newline".into(),
        "-o".into(),
        format!("{}/%(title)s.%(ext)s", out_dir),
    ];

    if include_playlist {
        args.push("--yes-playlist".into());
    } else {
        args.push("--no-playlist".into());
    }

    if kind == "hls" {
        args.push("--downloader".into());
        args.push("ffmpeg".into());
        args.push("--hls-use-mpegts".into());
        args.push("--hls-prefer-native".into());
    }

    if let Some(cookies) = cookies_file {
        if !cookies.trim().is_empty() {
            args.push("--cookies".into());
            args.push(cookies.into());
        }
    }

    args.push(url.into());
    args
}

#[tauri::command]
pub async fn check_download_tools() -> Result<serde_json::Value, String> {
    let ytdlp = Command::new("yt-dlp").arg("--version").output();
    let ffmpeg = Command::new("ffmpeg").arg("-version").output();
    let ffprobe = Command::new("ffprobe").arg("-version").output();

    Ok(serde_json::json!({
        "yt_dlp": {
            "installed": ytdlp.is_ok() && ytdlp.as_ref().unwrap().status.success(),
            "version": ytdlp.ok().map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").to_string()),
        },
        "ffmpeg": {
            "installed": ffmpeg.is_ok() && ffmpeg.as_ref().unwrap().status.success(),
            "version": ffmpeg.ok().map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").to_string()),
        },
        "ffprobe": {
            "installed": ffprobe.is_ok() && ffprobe.as_ref().unwrap().status.success(),
        },
        "supported_media": {
            "streaming": ["HLS .m3u8", "DASH .mpd"],
            "video": ["mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "flv", "ts", "mts", "m2ts", "3gp", "ogv"],
            "audio": ["mp3", "m4a", "aac", "flac", "wav", "ogg", "opus", "wma", "aiff", "alac"],
            "subtitles": ["srt", "vtt", "ass", "ssa", "ttml", "dfxp"],
            "images": ["jpg", "jpeg", "png", "webp", "gif", "bmp", "avif", "heic"]
        }
    }))
}

#[tauri::command]
pub async fn install_download_tools() -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    {
        let mut results = Vec::new();

        let ytdlp = Command::new("winget")
            .args(&[
                "install",
                "--id",
                "yt-dlp.yt-dlp",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ])
            .output();
        results.push(serde_json::json!({
            "tool": "yt-dlp",
            "success": ytdlp.as_ref().map(|o| o.status.success()).unwrap_or(false),
            "output": ytdlp.ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
        }));

        let ffmpeg = Command::new("winget")
            .args(&[
                "install",
                "--id",
                "Gyan.FFmpeg",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ])
            .output();
        results.push(serde_json::json!({
            "tool": "ffmpeg",
            "success": ffmpeg.as_ref().map(|o| o.status.success()).unwrap_or(false),
            "output": ffmpeg.ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
        }));

        Ok(serde_json::json!({ "results": results }))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(serde_json::json!({
            "status": "unsupported",
            "message": "Auto-install via winget is only available on Windows",
        }))
    }
}

#[tauri::command]
pub async fn crawl_media_links(
    url: String,
    max_links: Option<usize>,
) -> Result<serde_json::Value, String> {
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to fetch page: {}", e))?;
    let final_url = response.url().to_string();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read page: {}", e))?;

    if looks_like_captcha_or_challenge(&body) {
        return Ok(serde_json::json!({
            "status": "manual_challenge_required",
            "message": "This page appears to require a CAPTCHA or browser verification. Open it in a browser, complete the challenge manually, then retry with an exported cookies.txt file.",
            "url": final_url,
            "links": []
        }));
    }

    let limit = max_links.unwrap_or(250).min(1000);
    let re = Regex::new(
        r#"(?i)(?:href|src|data-src|source)\s*=\s*[\"']([^\"']+)[\"']|https?://[^\s\"'<>]+"#,
    )
    .map_err(|e| e.to_string())?;
    let mut seen = HashSet::new();
    let mut links: Vec<CrawlLink> = Vec::new();

    for cap in re.captures_iter(&body) {
        let raw = cap
            .get(1)
            .map(|m| m.as_str())
            .or_else(|| cap.get(0).map(|m| m.as_str()))
            .unwrap_or("");
        let cleaned = raw.trim().trim_matches(['\"', '\'', ',', ';']);
        if cleaned.is_empty() || cleaned.starts_with("data:") || cleaned.starts_with("javascript:")
        {
            continue;
        }

        let absolute = if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
            cleaned.to_string()
        } else if let Ok(base) = reqwest::Url::parse(&final_url) {
            match base.join(cleaned) {
                Ok(joined) => joined.to_string(),
                Err(_) => continue,
            }
        } else {
            continue;
        };

        if seen.insert(absolute.clone()) {
            let kind = media_kind(&absolute);
            if kind != "page"
                || absolute.contains("youtube.com")
                || absolute.contains("youtu.be")
                || absolute.contains("vimeo.com")
            {
                links.push(CrawlLink {
                    url: absolute,
                    kind,
                    source: final_url.clone(),
                });
            }
        }

        if links.len() >= limit {
            break;
        }
    }

    Ok(serde_json::json!({
        "status": "ok",
        "url": final_url,
        "links": links,
        "count": links.len()
    }))
}

#[tauri::command]
pub async fn get_supported_media_types() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "streaming": ["m3u8", "mpd"],
        "video": ["mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "flv", "ts", "mts", "m2ts", "3gp", "ogv"],
        "audio": ["mp3", "m4a", "aac", "flac", "wav", "ogg", "opus", "wma", "aiff", "alac"],
        "subtitles": ["srt", "vtt", "ass", "ssa", "ttml", "dfxp"],
        "images": ["jpg", "jpeg", "png", "webp", "gif", "bmp", "avif", "heic"],
        "notes": [
            "HLS and DASH streams are downloaded through yt-dlp/ffmpeg when supported by the source.",
            "CAPTCHA and browser verification are handled by manual user completion plus optional cookies.txt retry. Automated CAPTCHA solving is not included."
        ]
    }))
}

#[tauri::command]
pub async fn start_download(
    state: State<'_, AppState>,
    url: String,
    output_dir: Option<String>,
    format: Option<String>,
) -> Result<serde_json::Value, String> {
    start_media_download(state, url, output_dir, format, None, Some(false)).await
}

#[tauri::command]
pub async fn start_media_download(
    state: State<'_, AppState>,
    url: String,
    output_dir: Option<String>,
    format: Option<String>,
    cookies_file: Option<String>,
    include_playlist: Option<bool>,
) -> Result<serde_json::Value, String> {
    if DOWNLOADING.load(Ordering::Relaxed) {
        return Err("A download is already in progress".into());
    }
    DOWNLOADING.store(true, Ordering::Relaxed);
    CANCEL_DL.store(false, Ordering::Relaxed);

    let out_dir = output_dir.unwrap_or_else(default_download_dir);
    let fmt = format.unwrap_or_else(|| "bestvideo*+bestaudio/best/bestvideo+bestaudio".to_string());
    let now = chrono::Utc::now().to_rfc3339();
    let db_id = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.conn.execute(
            "INSERT INTO download_history (url, status, started_at) VALUES (?1, 'downloading', ?2)",
            params![url, now],
        ).map_err(|e| e.to_string())?;
        db.conn.last_insert_rowid()
    };

    let args = build_yt_dlp_args(
        &url,
        &out_dir,
        &fmt,
        include_playlist.unwrap_or(false),
        cookies_file.as_deref(),
    );
    let output = Command::new("yt-dlp").args(&args).output().map_err(|e| {
        DOWNLOADING.store(false, Ordering::Relaxed);
        format!("yt-dlp failed: {}", e)
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);
    let success = output.status.success();
    let needs_manual_challenge = !success && looks_like_captcha_or_challenge(&combined);

    let title = stdout
        .lines()
        .find(|l| {
            l.contains("[download] Destination:") || l.contains("[Merger] Merging formats into")
        })
        .map(|l| {
            l.split_once(':')
                .map(|(_, right)| right.trim().to_string())
                .unwrap_or_else(|| l.to_string())
        });

    let completed_at = chrono::Utc::now().to_rfc3339();
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        if success {
            db.conn.execute(
                "UPDATE download_history SET status = 'completed', title = ?1, completed_at = ?2 WHERE id = ?3",
                params![title, completed_at, db_id],
            ).map_err(|e| e.to_string())?;
        } else {
            db.conn.execute(
                "UPDATE download_history SET status = 'failed', error = ?1, completed_at = ?2 WHERE id = ?3",
                params![stderr, completed_at, db_id],
            ).map_err(|e| e.to_string())?;
        }
    }

    DOWNLOADING.store(false, Ordering::Relaxed);

    Ok(serde_json::json!({
        "id": db_id,
        "status": if success { "completed" } else if needs_manual_challenge { "manual_challenge_required" } else { "failed" },
        "media_kind": media_kind(&url),
        "title": title,
        "output_dir": out_dir,
        "yt_dlp_args": args,
        "output": stdout,
        "error": if stderr.is_empty() { None } else { Some(stderr) },
        "manual_challenge_required": needs_manual_challenge,
        "manual_challenge_message": if needs_manual_challenge { Some("Complete the CAPTCHA or browser challenge manually, export cookies.txt, then retry with cookies_file.".to_string()) } else { None }
    }))
}

#[tauri::command]
pub async fn start_playlist_download(
    state: State<'_, AppState>,
    url: String,
    output_dir: Option<String>,
) -> Result<serde_json::Value, String> {
    start_media_download(
        state,
        url,
        output_dir,
        Some("bestvideo*+bestaudio/best/bestvideo+bestaudio".into()),
        None,
        Some(true),
    )
    .await
}

#[tauri::command]
pub fn get_download_progress() -> serde_json::Value {
    serde_json::json!({
        "downloading": DOWNLOADING.load(Ordering::Relaxed),
    })
}

#[tauri::command]
pub fn cancel_download() -> Result<(), String> {
    CANCEL_DL.store(true, Ordering::Relaxed);
    Ok(())
}
