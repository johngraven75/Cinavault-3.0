use crate::scanner;
use crate::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn find_local_candidates(library_path: String) -> Result<Vec<String>, String> {
    let base_path = PathBuf::from(&library_path);
    if !base_path.exists() || !base_path.is_dir() {
        return Err("Invalid library path provided".to_string());
    }

    let mut candidates: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                candidates.push(path.to_string_lossy().to_string());
            }
        }
    }
    Ok(candidates)
}

#[tauri::command]
pub async fn refresh_pgma_library(state: State<'_, AppState>) -> Result<(), String> {
    // Triggers a full library scan to refresh metadata and posters
    scanner::scan_sources(state)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
