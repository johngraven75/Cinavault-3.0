pub const PORN_SITE_NUXT_DEFAULT_BASE_URL: &str = "http://localhost:42069/";

pub fn is_porn_site_nuxt_alias(provider: &str) -> bool {
    matches!(
        provider.trim().to_lowercase().as_str(),
        "porn_site_nuxt"
            | "porn-site-nuxt"
            | "porn site nuxt"
            | "pornsite_nuxt"
            | "pornsite"
            | "pornhub-irene"
            | "pornhub_irene"
            | "irenehub"
            | "irene_hub"
            | "nuxt_porn_site"
    )
}

pub fn porn_site_nuxt_base_url(configured: Option<&str>) -> String {
    configured
        .map(str::trim)
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
        .unwrap_or(PORN_SITE_NUXT_DEFAULT_BASE_URL)
        .trim_end_matches('/')
        .to_string()
}

pub fn porn_site_nuxt_search_url(base_url: &str, query: &str) -> String {
    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    format!("{}/search?q={}", base_url.trim_end_matches('/'), encoded)
}

pub fn porn_site_nuxt_entries(data: &serde_json::Value) -> Vec<&serde_json::Value> {
    data.get("content")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter(|entry| {
            entry
                .get("key")
                .and_then(|key| key.get("kind"))
                .and_then(|value| value.as_str())
                .map(|kind| kind == "PornEntry")
                .unwrap_or(true)
        })
        .collect()
}

pub fn entry_string(entry: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| entry.get(*key).and_then(|value| value.as_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn porn_site_nuxt_entry_id(entry: &serde_json::Value) -> Option<String> {
    entry
        .get("key")
        .and_then(|key| key.get("id"))
        .and_then(|value| value.as_str())
        .or_else(|| entry.get("id").and_then(|value| value.as_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn porn_site_nuxt_entry_title(entry: &serde_json::Value) -> Option<String> {
    entry_string(entry, &["name", "title", "sceneTitle"])
}

pub fn porn_site_nuxt_entry_source_url(entry: &serde_json::Value) -> Option<String> {
    entry_string(entry, &["sourceUrl", "source_url", "url", "href", "link"])
}

pub fn porn_site_nuxt_entry_image(entry: &serde_json::Value) -> Option<String> {
    entry_string(
        entry,
        &[
            "poster",
            "poster_path",
            "thumb",
            "thumbnail",
            "preview",
            "previewUrl",
            "image",
            "image_url",
        ],
    )
    .or_else(|| {
        entry
            .get("images")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|image| {
                image
                    .get("url")
                    .and_then(|value| value.as_str())
                    .or_else(|| image.as_str())
            })
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

pub fn porn_site_nuxt_entry_rating(entry: &serde_json::Value) -> Option<f64> {
    let raw = entry
        .get("rating")
        .and_then(|value| value.as_f64())
        .or_else(|| {
            entry
                .get("rating")
                .and_then(|value| value.as_str())
                .and_then(|value| value.trim_end_matches('%').parse::<f64>().ok())
        })?;
    if raw > 10.0 {
        Some((raw / 10.0).clamp(0.0, 10.0))
    } else {
        Some(raw.clamp(0.0, 10.0))
    }
}

pub fn porn_site_nuxt_entry_overview(entry: &serde_json::Value) -> Option<String> {
    let description = entry_string(entry, &["description", "details", "overview"]);
    let source_url = porn_site_nuxt_entry_source_url(entry);
    let duration = entry
        .get("duration")
        .and_then(|value| value.as_i64())
        .filter(|value| *value > 0)
        .map(|seconds| format!("Duration: {}:{:02}", seconds / 60, seconds % 60));

    let parts: Vec<String> = [
        description,
        source_url.map(|url| format!("Source: {url}")),
        duration,
    ]
    .into_iter()
    .flatten()
    .collect();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}
