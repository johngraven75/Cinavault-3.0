use regex::Regex;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_ARTWORK_BYTES: usize = 25 * 1024 * 1024;
const USER_AGENT: &str = concat!(
    "CinaVault-Premium/",
    env!("CARGO_PKG_VERSION"),
    " metadata-enrichment"
);
const CINEMETA_BASE_URL: &str = "https://v3-cinemeta.strem.io";

#[derive(Debug, Clone, Default)]
pub struct KeylessMetadataMatch {
    pub provider: String,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub genre: Option<String>,
    pub imdb_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CachedArtwork {
    pub path: String,
    pub mime_type: String,
    pub byte_length: usize,
    pub sha256: String,
}

pub fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|error| error.to_string())
}

fn portable_file_stem(file_path: &str) -> &str {
    let file_name = file_path
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(file_path);
    file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name)
}

pub fn metadata_query(title: &str, file_path: &str) -> String {
    let title_candidate = normalize_media_name(title);
    let file_candidate = normalize_media_name(portable_file_stem(file_path));

    if title_candidate.is_empty()
        || title.eq_ignore_ascii_case("unknown")
        || title.contains('_')
        || title.contains('.')
        || looks_like_release_name(title)
    {
        if !file_candidate.is_empty() {
            return file_candidate;
        }
    }

    if !title_candidate.is_empty() {
        title_candidate
    } else {
        file_candidate
    }
}

pub async fn fetch_keyless_match(
    client: &reqwest::Client,
    query: &str,
    media_type: &str,
) -> Result<Option<KeylessMetadataMatch>, String> {
    match media_type.trim().to_ascii_lowercase().as_str() {
        "episode" | "series" | "show" | "tv" => {
            match fetch_tvmaze_match(client, query).await {
                Ok(Some(result)) => Ok(Some(result)),
                Ok(None) => fetch_cinemeta_match(client, query, "series").await,
                Err(tvmaze_error) => match fetch_cinemeta_match(client, query, "series").await {
                    Ok(Some(result)) => Ok(Some(result)),
                    Ok(None) => Err(tvmaze_error),
                    Err(cinemeta_error) => Err(format!(
                        "TVMaze failed: {tvmaze_error}; Cinemeta series fallback failed: {cinemeta_error}"
                    )),
                },
            }
        }
        "movie" => fetch_cinemeta_match(client, query, "movie").await,
        _ => match fetch_cinemeta_match(client, query, "movie").await {
            Ok(Some(result)) => Ok(Some(result)),
            Ok(None) => fetch_tvmaze_match(client, query).await,
            Err(cinemeta_error) => match fetch_tvmaze_match(client, query).await {
                Ok(Some(result)) => Ok(Some(result)),
                Ok(None) => Err(cinemeta_error),
                Err(tvmaze_error) => Err(format!(
                    "Cinemeta movie lookup failed: {cinemeta_error}; TVMaze fallback failed: {tvmaze_error}"
                )),
            },
        },
    }
}

pub async fn fetch_tvmaze_match(
    client: &reqwest::Client,
    query: &str,
) -> Result<Option<KeylessMetadataMatch>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(None);
    }

    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    let url = format!("https://api.tvmaze.com/search/shows?q={encoded}");
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("TVMaze request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("TVMaze returned an error: {error}"))?;
    let data = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("TVMaze JSON decode failed: {error}"))?;
    let Some(results) = data.as_array() else {
        return Ok(None);
    };

    let selected = results.iter().find_map(|entry| {
        let show = entry.get("show")?;
        let name = show.get("name")?.as_str()?;
        title_matches(query, name).then_some(show)
    });
    let Some(show) = selected else {
        return Ok(None);
    };

    let title = clean_string(show.get("name").and_then(|value| value.as_str()));
    let overview = clean_string(show.get("summary").and_then(|value| value.as_str()))
        .map(|value| strip_html(&value));
    let poster_url = show
        .get("image")
        .and_then(|value| value.as_object())
        .and_then(|image| {
            image
                .get("original")
                .or_else(|| image.get("medium"))
                .and_then(|value| value.as_str())
        })
        .and_then(|value| clean_string(Some(value)))
        .filter(|value| value.starts_with("https://"));
    let year = parse_year(show.get("premiered").and_then(|value| value.as_str()));
    let rating = show
        .get("rating")
        .and_then(|value| value.get("average"))
        .and_then(parse_rating_value)
        .filter(|value| *value > 0.0);
    let genre = join_string_array(show.get("genres"));
    let imdb_id = show
        .get("externals")
        .and_then(|value| value.get("imdb"))
        .and_then(|value| value.as_str())
        .and_then(|value| clean_string(Some(value)))
        .filter(|value| value.starts_with("tt"));

    Ok(Some(KeylessMetadataMatch {
        provider: "tvmaze".to_string(),
        title,
        overview,
        poster_url,
        year,
        rating,
        genre,
        imdb_id,
    }))
}

pub async fn fetch_cinemeta_match(
    client: &reqwest::Client,
    query: &str,
    content_type: &str,
) -> Result<Option<KeylessMetadataMatch>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(None);
    }
    let content_type = match content_type {
        "series" => "series",
        _ => "movie",
    };
    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    let url = format!("{CINEMETA_BASE_URL}/catalog/{content_type}/top/search={encoded}.json");
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Cinemeta request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Cinemeta returned an error: {error}"))?;
    let data = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("Cinemeta JSON decode failed: {error}"))?;
    let Some(results) = data.get("metas").and_then(|value| value.as_array()) else {
        return Ok(None);
    };

    let selected = results.iter().find(|entry| {
        entry
            .get("name")
            .and_then(|value| value.as_str())
            .is_some_and(|name| title_matches(query, name))
    });
    let Some(meta) = selected else {
        return Ok(None);
    };

    let title = clean_string(meta.get("name").and_then(|value| value.as_str()));
    let overview = clean_string(
        meta.get("description")
            .or_else(|| meta.get("overview"))
            .and_then(|value| value.as_str()),
    );
    let poster_url = meta
        .get("poster")
        .or_else(|| meta.get("posterShape"))
        .and_then(|value| value.as_str())
        .and_then(|value| clean_string(Some(value)))
        .filter(|value| value.starts_with("https://"));
    let year = meta
        .get("releaseInfo")
        .and_then(|value| value.as_str())
        .and_then(|value| parse_year(Some(value)))
        .or_else(|| {
            meta.get("released")
                .and_then(|value| value.as_str())
                .and_then(|value| parse_year(Some(value)))
        });
    let rating = meta
        .get("imdbRating")
        .or_else(|| meta.get("rating"))
        .and_then(parse_rating_value)
        .filter(|value| *value > 0.0);
    let genre = join_string_array(meta.get("genres"));
    let imdb_id = meta
        .get("id")
        .and_then(|value| value.as_str())
        .and_then(|value| clean_string(Some(value)))
        .filter(|value| value.starts_with("tt"));

    Ok(Some(KeylessMetadataMatch {
        provider: "cinemeta".to_string(),
        title,
        overview,
        poster_url,
        year,
        rating,
        genre,
        imdb_id,
    }))
}

pub async fn cache_remote_artwork(
    client: &reqwest::Client,
    app_data_dir: &Path,
    media_id: i64,
    kind: &str,
    url: &str,
) -> Result<CachedArtwork, String> {
    if !matches!(kind, "poster" | "backdrop") {
        return Err("Artwork kind must be poster or backdrop".to_string());
    }
    if !url.starts_with("https://") {
        return Err("Remote artwork must use HTTPS".to_string());
    }

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Artwork request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Artwork provider returned an error: {error}"))?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_ARTWORK_BYTES as u64)
    {
        return Err("Artwork exceeds the 25 MiB cache limit".to_string());
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .trim()
        .to_ascii_lowercase();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Artwork read failed: {error}"))?;
    if bytes.is_empty() || bytes.len() > MAX_ARTWORK_BYTES {
        return Err("Artwork payload is empty or too large".to_string());
    }

    let (extension, mime_type) = detect_image_type(&content_type, &bytes)
        .ok_or_else(|| "Artwork payload is not a supported image".to_string())?;
    let sha256 = hex_sha256(&bytes);
    let directory = app_data_dir.join("artwork").join(media_id.to_string());
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Artwork cache directory creation failed: {error}"))?;
    let filename = format!("{kind}-{}.{}", &sha256[..16], extension);
    let final_path = directory.join(filename);

    if !final_path.exists() {
        let temporary_path = directory.join(format!(".{kind}-{}.part", &sha256[..16]));
        std::fs::write(&temporary_path, &bytes)
            .map_err(|error| format!("Artwork cache write failed: {error}"))?;
        let written = std::fs::metadata(&temporary_path)
            .map_err(|error| format!("Artwork cache verification failed: {error}"))?
            .len();
        if written != bytes.len() as u64 {
            let _ = std::fs::remove_file(&temporary_path);
            return Err("Artwork cache verification detected a truncated write".to_string());
        }
        std::fs::rename(&temporary_path, &final_path)
            .map_err(|error| format!("Artwork cache finalize failed: {error}"))?;
    }

    remove_superseded_artwork(&directory, kind, &final_path);

    Ok(CachedArtwork {
        path: final_path.to_string_lossy().to_string(),
        mime_type: mime_type.to_string(),
        byte_length: bytes.len(),
        sha256,
    })
}

fn remove_superseded_artwork(directory: &Path, kind: &str, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let prefix = format!("{kind}-");
    for entry in entries.flatten() {
        let path = entry.path();
        if path == keep {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with(&prefix) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn normalize_media_name(value: &str) -> String {
    let mut text = value.replace(['.', '_', '-'], " ");
    for pattern in [
        r"(?i)\bS\d{1,2}E\d{1,3}\b",
        r"(?i)\b\d{1,2}x\d{1,3}\b",
        r"\b(19\d{2}|20\d{2})\b",
        r"(?i)\b(480p|576p|720p|1080p|1440p|2160p|4k|8k|x264|x265|h264|h265|hevc|avc|web\s?dl|webrip|bluray|brrip|dvdrip|hdr|remux|proper|repack)\b",
        r"(?i)\b(mkv|mp4|avi|mov|wmv|webm|m4v|mpg|mpeg|m2ts|ts)\b$",
    ] {
        text = Regex::new(pattern)
            .expect("metadata normalization regex should compile")
            .replace_all(&text, " ")
            .to_string();
    }
    Regex::new(r"\s+")
        .expect("whitespace regex should compile")
        .replace_all(text.trim(), " ")
        .trim()
        .to_string()
}

fn looks_like_release_name(value: &str) -> bool {
    Regex::new(
        r"(?i)\b(S\d{1,2}E\d{1,3}|\d{1,2}x\d{1,3}|480p|720p|1080p|2160p|x264|x265|webrip|bluray)\b",
    )
    .expect("release-name regex should compile")
    .is_match(value)
}

fn title_matches(expected: &str, candidate: &str) -> bool {
    let expected_words = words(expected);
    let candidate_words = words(candidate);
    if expected_words.is_empty() || candidate_words.is_empty() {
        return false;
    }
    if expected_words == candidate_words {
        return true;
    }
    let shared = expected_words
        .iter()
        .filter(|word| candidate_words.contains(word))
        .count();
    shared * 2 >= expected_words.len().min(candidate_words.len()).max(1)
}

fn words(value: &str) -> Vec<String> {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .filter(|word| word.len() > 1)
        .map(str::to_string)
        .collect()
}

fn clean_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("N/A"))
        .map(str::to_string)
}

fn parse_year(value: Option<&str>) -> Option<i32> {
    let value = value?.trim();
    if value.len() < 4 {
        return None;
    }
    value[..4].parse::<i32>().ok()
}

fn parse_rating_value(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().or_else(|| {
        value
            .as_str()
            .and_then(|text| text.trim().parse::<f64>().ok())
    })
}

fn join_string_array(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
}

fn strip_html(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut inside_tag = false;
    for character in value.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => output.push(character),
            _ => {}
        }
    }
    output
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn detect_image_type(content_type: &str, bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(("jpg", "image/jpeg"));
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(("png", "image/png"));
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(("webp", "image/webp"));
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(("gif", "image/gif"));
    }
    match content_type {
        "image/jpeg" | "image/jpg" => Some(("jpg", "image/jpeg")),
        "image/png" => Some(("png", "image/png")),
        "image/webp" => Some(("webp", "image/webp")),
        "image/gif" => Some(("gif", "image/gif")),
        _ => None,
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{metadata_query, strip_html};

    #[test]
    fn release_name_is_reduced_to_show_title() {
        assert_eq!(
            metadata_query(
                "Breaking.Bad.S01E01.1080p",
                r"C:\TV\Breaking.Bad.S01E01.1080p.mkv"
            ),
            "Breaking Bad"
        );
    }

    #[test]
    fn movie_release_name_is_reduced_to_movie_title() {
        assert_eq!(
            metadata_query(
                "Inception.2010.1080p",
                r"C:\Movies\Inception.2010.1080p.mkv"
            ),
            "Inception"
        );
    }

    #[test]
    fn unix_and_windows_paths_normalize_identically() {
        assert_eq!(
            metadata_query("Unknown", "/TV/Breaking.Bad.S01E01.1080p.mkv"),
            metadata_query("Unknown", r"C:\TV\Breaking.Bad.S01E01.1080p.mkv")
        );
    }

    #[test]
    fn tvmaze_html_summary_is_plain_text() {
        assert_eq!(strip_html("<p>A <b>great</b> show.</p>"), "A great show.");
    }
}
