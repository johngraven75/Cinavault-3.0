use std::path::{Path, PathBuf};

const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "svg", "avif", "heic",
];
const POSTER_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "avif", "heic"];

fn extension_lower(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

fn file_stem_lower(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.trim().to_ascii_lowercase())
}

pub fn is_generated_chapter_image_path(path: &Path) -> bool {
    let path_lower = path.to_string_lossy().replace('/', "\\").to_lowercase();
    path_lower.contains("_chapters\\chapter_")
}

pub fn is_supported_image_path(path: &Path) -> bool {
    extension_lower(path)
        .map(|ext| IMAGE_EXTS.contains(&ext.as_str()))
        .unwrap_or(false)
}

pub fn is_sidecar_artwork_image(path: &Path) -> bool {
    if !is_supported_image_path(path) {
        return false;
    }

    let Some(stem) = file_stem_lower(path) else {
        return false;
    };

    let generic_artwork_names = [
        "poster",
        "cover",
        "folder",
        "fanart",
        "backdrop",
        "landscape",
        "banner",
    ];
    if generic_artwork_names.contains(&stem.as_str()) {
        return true;
    }

    let artwork_suffixes = [
        "-poster",
        "_poster",
        ".poster",
        "-cover",
        "_cover",
        ".cover",
        "-folder",
        "_folder",
        ".folder",
        "-fanart",
        "_fanart",
        ".fanart",
        "-backdrop",
        "_backdrop",
        ".backdrop",
        "-landscape",
        "_landscape",
        ".landscape",
        "-banner",
        "_banner",
        ".banner",
    ];

    artwork_suffixes.iter().any(|suffix| stem.ends_with(suffix))
}

pub fn sidecar_poster_path_for_video(video_path: &Path) -> Option<PathBuf> {
    let parent = video_path.parent()?;
    let stem = video_path.file_stem()?.to_str()?.trim();
    if stem.is_empty() {
        return None;
    }

    let stem_candidates = [
        "-poster",
        "_poster",
        ".poster",
        "-cover",
        "_cover",
        ".cover",
        "-folder",
        "_folder",
        ".folder",
        "-fanart",
        "_fanart",
        ".fanart",
        "-backdrop",
        "_backdrop",
        ".backdrop",
        "-landscape",
        "_landscape",
        ".landscape",
        "-banner",
        "_banner",
        ".banner",
    ];
    for suffix in stem_candidates {
        for ext in POSTER_EXTS {
            let candidate = parent.join(format!("{stem}{suffix}.{ext}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    let generic_candidates = [
        "poster",
        "cover",
        "folder",
        "fanart",
        "backdrop",
        "landscape",
        "banner",
    ];
    for name in generic_candidates {
        for ext in POSTER_EXTS {
            let candidate = parent.join(format!("{name}.{ext}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}
