// CinaVault Premium — Jellyfin/Emby Server Management
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub url: String,
    pub running: bool,
}

#[tauri::command]
pub async fn start_server(server_type: String) -> Result<serde_json::Value, String> {
    let exe_name = match server_type.as_str() {
        "jellyfin" => "jellyfin.exe",
        "emby" => "EmbyServer.exe",
        _ => return Err("Unknown server type".into()),
    };

    // Try common install paths
    let paths = vec![
        format!(
            "C:\\Program Files\\{}",
            if server_type == "jellyfin" {
                "Jellyfin\\Server"
            } else {
                "Emby-Server"
            }
        ),
        format!(
            "C:\\Program Files (x86)\\{}",
            if server_type == "jellyfin" {
                "Jellyfin\\Server"
            } else {
                "Emby-Server"
            }
        ),
    ];

    for path in &paths {
        let exe_path = format!("{}\\{}", path, exe_name);
        if std::path::Path::new(&exe_path).exists() {
            Command::new(&exe_path).spawn().map_err(|e| e.to_string())?;
            return Ok(serde_json::json!({
                "status": "started",
                "server": server_type,
                "path": exe_path,
            }));
        }
    }

    Err(format!(
        "{} server executable not found in standard paths",
        server_type
    ))
}

#[tauri::command]
pub async fn stop_server(server_type: String) -> Result<serde_json::Value, String> {
    let process_name = match server_type.as_str() {
        "jellyfin" => "jellyfin",
        "emby" => "EmbyServer",
        _ => return Err("Unknown server type".into()),
    };

    #[cfg(target_os = "windows")]
    {
        Command::new("taskkill")
            .args(&["/IM", &format!("{}.exe", process_name), "/F"])
            .output()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("pkill")
            .arg("-f")
            .arg(process_name)
            .output()
            .map_err(|e| e.to_string())?;
    }

    Ok(serde_json::json!({
        "status": "stopped",
        "server": server_type,
    }))
}

#[tauri::command]
pub async fn get_server_status(
    server_type: String,
    base_url: Option<String>,
) -> Result<serde_json::Value, String> {
    let url = base_url.unwrap_or_else(|| match server_type.as_str() {
        "jellyfin" => "http://localhost:8096".to_string(),
        "emby" => "http://localhost:8096".to_string(),
        _ => "http://localhost:8096".to_string(),
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let info_url = format!("{}/System/Info/Public", url);
    match client.get(&info_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                Ok(serde_json::json!({
                    "running": true,
                    "server_name": data.get("ServerName").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                    "version": data.get("Version").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                    "url": url,
                    "id": data.get("Id").and_then(|v| v.as_str()),
                }))
            } else {
                Ok(serde_json::json!({ "running": false, "url": url }))
            }
        }
        Err(_) => Ok(serde_json::json!({ "running": false, "url": url })),
    }
}

#[tauri::command]
pub async fn get_server_info(
    base_url: String,
    api_key: Option<String>,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/System/Info", base_url));
    if let Some(key) = &api_key {
        req = req.header("X-Emby-Token", key);
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(data)
}

#[tauri::command]
pub async fn import_libraries(
    base_url: String,
    api_key: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/Library/VirtualFolders", base_url))
        .header("X-Emby-Token", &api_key)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let libraries: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut imported = 0u64;

    for lib in &libraries {
        if let Some(locations) = lib.get("Locations").and_then(|v| v.as_array()) {
            let lib_name = lib
                .get("Name")
                .and_then(|v| v.as_str())
                .unwrap_or("Library");
            for loc in locations {
                if let Some(path) = loc.as_str() {
                    let _ = db.conn.execute(
                        "INSERT OR IGNORE INTO media_sources (path, source_type, name, enabled, item_count) VALUES (?1, 'folder', ?2, 1, 0)",
                        rusqlite::params![path, format!("{} (Imported)", lib_name)],
                    );
                    imported += 1;
                }
            }
        }
    }

    Ok(serde_json::json!({
        "libraries_found": libraries.len(),
        "sources_imported": imported,
    }))
}

#[tauri::command]
pub async fn check_emby_compat(base_url: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    match client
        .get(format!("{}/System/Info/Public", base_url))
        .send()
        .await
    {
        Ok(resp) => {
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            let version = data.get("Version").and_then(|v| v.as_str()).unwrap_or("");
            let product = data
                .get("ProductName")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(serde_json::json!({
                "compatible": true,
                "product": product,
                "version": version,
                "emby_api": product.to_lowercase().contains("emby"),
                "jellyfin_api": product.to_lowercase().contains("jellyfin"),
            }))
        }
        Err(e) => Ok(serde_json::json!({
            "compatible": false,
            "error": e.to_string(),
        })),
    }
}

#[tauri::command]
pub async fn open_admin_page(base_url: String, page: String) -> Result<(), String> {
    let url = match page.as_str() {
        "dashboard" => format!("{}/web/index.html#!/dashboard", base_url),
        "libraries" => format!("{}/web/index.html#!/libraries", base_url),
        "users" => format!("{}/web/index.html#!/users", base_url),
        "plugins" => format!("{}/web/index.html#!/plugins", base_url),
        "tasks" => format!("{}/web/index.html#!/scheduledtasks", base_url),
        "logs" => format!("{}/web/index.html#!/log", base_url),
        "sessions" => format!("{}/Sessions", base_url),
        "devices" => format!("{}/Devices", base_url),
        _ => format!("{}/web/index.html", base_url),
    };
    open::that(&url).map_err(|e| e.to_string())
}
