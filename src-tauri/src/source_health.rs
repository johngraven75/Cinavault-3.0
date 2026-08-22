use serde::Serialize;
use std::path::Path;
#[cfg(desktop)]
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct SourcePathHealth {
    pub path: String,
    pub source_type: String,
    pub exists: bool,
    pub readable: bool,
    pub expected_kind: bool,
    pub status: &'static str,
    pub message: String,
}

fn test_directory_read(path: &Path) -> Result<(), String> {
    let mut entries = std::fs::read_dir(path).map_err(|error| error.to_string())?;
    // Force at least one iterator operation so delayed permission/device errors surface.
    if let Some(entry) = entries.next() {
        entry.map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn validate_source_path(path: String, source_type: String) -> SourcePathHealth {
    let trimmed = path.trim().to_string();
    let candidate = Path::new(&trimmed);
    let exists = candidate.exists();
    let expected_kind = match source_type.as_str() {
        "file" => candidate.is_file(),
        _ => candidate.is_dir(),
    };

    let read_result = if !exists {
        Err("path does not exist or the external drive is disconnected".to_string())
    } else if !expected_kind {
        Err(match source_type.as_str() {
            "file" => "source type is File but the selected path is not a file".to_string(),
            _ => "source type is Folder/Drive but the selected path is not a directory".to_string(),
        })
    } else if source_type == "file" {
        std::fs::File::open(candidate)
            .map(|_| ())
            .map_err(|error| error.to_string())
    } else {
        test_directory_read(candidate)
    };

    match read_result {
        Ok(()) => SourcePathHealth {
            path: trimmed,
            source_type,
            exists,
            readable: true,
            expected_kind,
            status: "ready",
            message: "Source is connected, readable, and ready to scan".to_string(),
        },
        Err(message) => SourcePathHealth {
            path: trimmed,
            source_type,
            exists,
            readable: false,
            expected_kind,
            status: "unavailable",
            message,
        },
    }
}

#[tauri::command]
pub fn explore_source_path(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("A source path is required".to_string());
    }

    let candidate = Path::new(trimmed);
    if !candidate.exists() {
        return Err("Source path does not exist or the external drive is disconnected".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer.exe");
        if candidate.is_file() {
            command.arg(format!("/select,{}", candidate.to_string_lossy()));
        } else {
            command.arg(candidate);
        }
        command
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Windows Explorer failed to open: {error}"))
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        if candidate.is_file() {
            command.arg("-R");
        }
        command
            .arg(candidate)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Finder failed to open: {error}"))
    }

    #[cfg(all(unix, not(target_os = "macos"), not(mobile)))]
    {
        let target = if candidate.is_file() {
            candidate.parent().unwrap_or(candidate)
        } else {
            candidate
        };
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("File manager failed to open: {error}"))
    }

    #[cfg(mobile)]
    {
        Err("Explore Source is available on desktop platforms; use the native source picker on mobile".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{explore_source_path, validate_source_path};

    #[test]
    fn missing_external_path_is_reported_as_unavailable() {
        let result = validate_source_path(
            "Z:\\definitely-not-a-real-cinavault-drive".to_string(),
            "drive".to_string(),
        );
        assert_eq!(result.status, "unavailable");
        assert!(!result.readable);
    }

    #[test]
    fn explorer_rejects_missing_paths_without_spawning_a_process() {
        let result = explore_source_path("Z:\\definitely-not-a-real-cinavault-drive".to_string());
        assert!(result.is_err());
    }
}
