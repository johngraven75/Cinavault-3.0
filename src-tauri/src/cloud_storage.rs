// CinaVault Premium — real cloud-folder source integration.
//
// CinaVault consumes provider folders that are synchronized by the provider's
// desktop client. Commands never report success unless a real, readable folder
// was found and (for sync) persisted as an enabled media source.
use crate::db::MediaSource;
use crate::AppState;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::State;
use walkdir::WalkDir;

const MEDIA_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "m4v", "webm", "mpg", "mpeg", "ts", "m2ts", "mp3", "flac",
    "m4a", "aac", "wav", "ogg",
];

fn provider_key(provider: &str) -> Result<&'static str, String> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "onedrive" | "one drive" => Ok("onedrive"),
        "googledrive" | "google_drive" | "google drive" => Ok("googledrive"),
        "dropbox" => Ok("dropbox"),
        _ => Err(format!("Unsupported cloud provider: {provider}")),
    }
}

fn provider_candidates(provider: &str) -> Result<Vec<PathBuf>, String> {
    let provider = provider_key(provider)?;
    let mut candidates = Vec::new();
    if provider == "onedrive" {
        for variable in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
            if let Some(value) = std::env::var_os(variable) {
                candidates.push(PathBuf::from(value));
            }
        }
    }
    if let Some(profile) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let profile = PathBuf::from(profile);
        match provider {
            "onedrive" => candidates.push(profile.join("OneDrive")),
            "googledrive" => {
                candidates.push(profile.join("Google Drive"));
                candidates.push(profile.join("My Drive"));
            }
            "dropbox" => candidates.push(profile.join("Dropbox")),
            _ => {}
        }
    }
    if provider == "googledrive" {
        for drive in ["G:\\My Drive", "G:\\Shared drives"] {
            candidates.push(PathBuf::from(drive));
        }
    }
    Ok(candidates)
}

fn readable_directory(path: &Path) -> bool {
    path.is_dir() && std::fs::read_dir(path).is_ok()
}

fn resolve_provider_path(provider: &str, requested: &str) -> Result<PathBuf, String> {
    if !requested.trim().is_empty() {
        let path = PathBuf::from(requested.trim());
        if readable_directory(&path) {
            return path.canonicalize().map_err(|error| error.to_string());
        }
        return Err(format!(
            "Cloud folder is not readable: {}. Select a folder synchronized by the provider desktop client.",
            path.display()
        ));
    }
    provider_candidates(provider)?
        .into_iter()
        .find(|path| readable_directory(path))
        .and_then(|path| path.canonicalize().ok())
        .ok_or_else(|| {
            format!(
                "No readable {provider} folder was found. Install/sign in to the provider desktop client, then select its synchronized folder."
            )
        })
}

fn count_media_files(root: &Path) -> u64 {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| {
                    MEDIA_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
                })
                .unwrap_or(false)
        })
        .count() as u64
}

fn list_directory_entries(root: &Path) -> Result<Vec<Value>, String> {
    let mut entries = std::fs::read_dir(root)
        .map_err(|error| format!("Unable to read {}: {error}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            let metadata = entry.metadata().ok();
            json!({
                "name": entry.file_name().to_string_lossy(),
                "path": path.to_string_lossy(),
                "is_directory": metadata.as_ref().map(|value| value.is_dir()).unwrap_or(false),
                "size": metadata.as_ref().filter(|value| value.is_file()).map(|value| value.len()).unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .cmp(
                &right["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_ascii_lowercase(),
            )
    });
    Ok(entries)
}

fn setting_key(provider: &str) -> Result<String, String> {
    Ok(format!("cloud_connection_{}", provider_key(provider)?))
}

#[tauri::command]
pub fn cloud_auth_start(
    state: State<AppState>,
    provider: String,
    auth_url: String,
) -> Result<Value, String> {
    let root = resolve_provider_path(&provider, "")?;
    let provider = provider_key(&provider)?;
    let record = json!({
        "provider": provider,
        "root": root.to_string_lossy(),
        "method": "desktop_sync_folder",
        "verified_at": chrono::Utc::now().to_rfc3339(),
    });
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.set_setting_data(&setting_key(provider)?, &record.to_string())
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "success": true,
        "account": format!("{provider} synchronized folder"),
        "method": "desktop_sync_folder",
        "path": root.to_string_lossy(),
        "auth_url_ignored": !auth_url.trim().is_empty(),
    }))
}

#[tauri::command]
pub fn cloud_disconnect(state: State<AppState>, provider: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.set_setting_data(&setting_key(&provider)?, "")
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn cloud_sync(state: State<AppState>, provider: String, path: String) -> Result<Value, String> {
    let root = resolve_provider_path(&provider, &path)?;
    let provider = provider_key(&provider)?;
    let count = count_media_files(&root);
    let name = format!(
        "{} — {}",
        provider,
        root.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Media")
    );
    let source_path = root.to_string_lossy().to_string();

    let db = state.db.lock().map_err(|error| error.to_string())?;
    let already_present = db
        .get_sources_data()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|source| source.path == source_path);
    if !already_present {
        db.add_source_data(&MediaSource {
            id: None,
            path: source_path.clone(),
            source_type: "mixed".to_string(),
            name,
            enabled: true,
            last_scanned: None,
            item_count: count as i64,
        })
        .map_err(|error| error.to_string())?;
    }
    let record = json!({
        "provider": provider,
        "root": source_path,
        "media_files": count,
        "verified_at": chrono::Utc::now().to_rfc3339(),
    });
    db.set_setting_data(&setting_key(provider)?, &record.to_string())
        .map_err(|error| error.to_string())?;

    Ok(json!({
        "success": true,
        "synced": count,
        "source_added": !already_present,
        "path": root.to_string_lossy(),
        "message": format!("Verified {count} media files in {}", root.display()),
    }))
}

#[tauri::command]
pub fn cloud_browse(provider: String, path: String) -> Result<Vec<Value>, String> {
    let root = resolve_provider_path(&provider, &path)?;
    list_directory_entries(&root)
}

#[tauri::command]
pub fn cloud_list_files(provider: String, path: String) -> Result<Vec<Value>, String> {
    cloud_browse(provider, path)
}

#[tauri::command]
pub fn cloud_get_status(state: State<AppState>) -> Result<Value, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    let mut connected = Vec::new();
    for provider in ["onedrive", "googledrive", "dropbox"] {
        if let Some(raw) = db
            .get_setting_data(&setting_key(provider)?)
            .map_err(|error| error.to_string())?
            .filter(|value| !value.is_empty())
        {
            if let Ok(record) = serde_json::from_str::<Value>(&raw) {
                if record["root"]
                    .as_str()
                    .map(Path::new)
                    .map(readable_directory)
                    .unwrap_or(false)
                {
                    connected.push(record);
                }
            }
        }
    }
    Ok(json!({
        "connected": connected,
        "available": ["onedrive", "googledrive", "dropbox"],
    }))
}

#[cfg(test)]
mod tests {
    use super::{count_media_files, list_directory_entries, resolve_provider_path};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cloud_folder_operations_read_real_files_and_never_fake_counts() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cinavault-cloud-test-{stamp}"));
        std::fs::create_dir_all(root.join("Movies")).unwrap();
        std::fs::write(root.join("Movies").join("Feature.mkv"), b"media").unwrap();
        std::fs::write(root.join("notes.txt"), b"not media").unwrap();

        let resolved = resolve_provider_path("dropbox", root.to_string_lossy().as_ref()).unwrap();
        let entries = list_directory_entries(&resolved).unwrap();

        assert_eq!(count_media_files(&resolved), 1);
        assert!(entries.iter().any(|entry| entry["name"] == "Movies"));
        assert!(entries.iter().any(|entry| entry["name"] == "notes.txt"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn unreadable_cloud_folder_returns_an_error_instead_of_success() {
        let missing = std::env::temp_dir().join("cinavault-cloud-folder-that-does-not-exist");
        assert!(resolve_provider_path("onedrive", missing.to_string_lossy().as_ref()).is_err());
    }
}
