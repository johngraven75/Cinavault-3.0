// CinaVault Premium — NAS Device Integration
// Synology QuickConnect + WD My Cloud Home
use crate::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::process::Command;
use tauri::State;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ════════════════════════════════════════════════════════════
//  Shared types
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NasCredentials {
    pub device_type: String, // "synology" | "wd_mycloud"
    pub host: String,        // QuickConnect ID or IP/hostname
    pub username: String,
    pub password: String,
    pub port: Option<u16>,
    pub use_https: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NasLibrary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub share_name: String,
    pub media_type: String, // "movies" | "tv" | "music" | "photos" | "mixed"
    pub item_count: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NasConnectionResult {
    pub success: bool,
    pub device_name: String,
    pub device_model: String,
    pub firmware: String,
    pub host_resolved: String,
    pub libraries: Vec<NasLibrary>,
    pub error: Option<String>,
}

// ════════════════════════════════════════════════════════════
//  Synology QuickConnect
//  Resolves a QuickConnect ID → real LAN/WAN IP via the
//  Synology relay service, then authenticates via DSM API.
// ════════════════════════════════════════════════════════════

fn resolve_quickconnect(quickconnect_id: &str) -> Result<String, String> {
    // Try direct LAN first (common case when on same network)
    let _relay_url = format!(
        "https://global.quickconnect.to/Serv.php?id={}&serverID=&type=relay",
        quickconnect_id
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    // Query the QuickConnect relay to resolve the real host
    let body = serde_json::json!({
        "version": "1",
        "command": "get_server_info",
        "stop_when_error": false,
        "stop_when_success": false,
        "id": "dsm_portal_https",
        "serverID": quickconnect_id,
        "is_gofile": false
    });

    let resp = client
        .post("https://global.quickconnect.to/Serv.php")
        .json(&body)
        .send()
        .map_err(|e| format!("QuickConnect relay error: {}", e))?;

    let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;

    // Try LAN hosts first, then relay hosts
    if let Some(env) = json.get("env") {
        if let Some(relay_region) = env.get("relay_region") {
            if let Some(hosts) = relay_region.get("hosts") {
                if let Some(arr) = hosts.as_array() {
                    for h in arr {
                        if let Some(host) = h.get("host").and_then(|v| v.as_str()) {
                            return Ok(host.to_string());
                        }
                    }
                }
            }
        }
        // Try direct LAN IP
        if let Some(control_host) = env.get("control_host") {
            if let Some(host) = control_host.as_str() {
                if !host.is_empty() {
                    return Ok(host.to_string());
                }
            }
        }
    }

    // Fallback: treat the ID as a direct hostname
    Ok(format!("{}.quickconnect.to", quickconnect_id))
}

fn synology_login(
    host: &str,
    port: u16,
    use_https: bool,
    username: &str,
    password: &str,
) -> Result<String, String> {
    let scheme = if use_https { "https" } else { "http" };
    let url = format!(
        "{}://{}:{}/webapi/auth.cgi?api=SYNO.API.Auth&version=3&method=login&account={}&passwd={}&session=CinaVault&format=sid",
        scheme, host, port,
        urlencoding_simple(username),
        urlencoding_simple(password)
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("Synology login error: {}", e))?;
    let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;

    if json
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let sid = json["data"]["sid"].as_str().unwrap_or("").to_string();
        Ok(sid)
    } else {
        let code = json["error"]["code"].as_u64().unwrap_or(0);
        Err(format!("Synology auth failed (error code {})", code))
    }
}

fn synology_get_shares(
    host: &str,
    port: u16,
    use_https: bool,
    sid: &str,
) -> Result<Vec<NasLibrary>, String> {
    let scheme = if use_https { "https" } else { "http" };
    let url = format!(
        "{}://{}:{}/webapi/entry.cgi?api=SYNO.FileStation.List&version=2&method=list_share&_sid={}",
        scheme, host, port, sid
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&url).send().map_err(|e| e.to_string())?;
    let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;

    let mut libraries = Vec::new();
    if let Some(shares) = json["data"]["shares"].as_array() {
        for share in shares {
            let name = share["name"].as_str().unwrap_or("").to_string();
            let path = share["path"].as_str().unwrap_or("").to_string();
            let additional = &share["additional"];
            let size_bytes = additional["volume_status"]["totalspace"]
                .as_u64()
                .unwrap_or(0);

            // Infer media type from share name
            let media_type = infer_media_type_from_name(&name);

            libraries.push(NasLibrary {
                id: format!("synology-{}", name.to_lowercase().replace(' ', "-")),
                name: name.clone(),
                path: path.clone(),
                share_name: name,
                media_type,
                item_count: 0,
                size_bytes,
            });
        }
    }
    Ok(libraries)
}

fn synology_get_info(
    host: &str,
    port: u16,
    use_https: bool,
    sid: &str,
) -> (String, String, String) {
    let scheme = if use_https { "https" } else { "http" };
    let url = format!(
        "{}://{}:{}/webapi/entry.cgi?api=SYNO.DSM.Info&version=2&method=getinfo&_sid={}",
        scheme, host, port, sid
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_default();

    if let Ok(resp) = client.get(&url).send() {
        if let Ok(json) = resp.json::<serde_json::Value>() {
            let model = json["data"]["model"]
                .as_str()
                .unwrap_or("Synology NAS")
                .to_string();
            let firmware = json["data"]["version_string"]
                .as_str()
                .unwrap_or("DSM")
                .to_string();
            let hostname = json["data"]["hostname"]
                .as_str()
                .unwrap_or(host)
                .to_string();
            return (hostname, model, firmware);
        }
    }
    (
        host.to_string(),
        "Synology NAS".to_string(),
        "DSM".to_string(),
    )
}

// ════════════════════════════════════════════════════════════
//  WD My Cloud
//  Authenticates via the WD My Cloud REST API (local or cloud)
// ════════════════════════════════════════════════════════════

#[derive(Clone)]
struct WdSession {
    client: reqwest::blocking::Client,
    token: String,
}

fn wd_mycloud_login(
    host: &str,
    port: u16,
    use_https: bool,
    username: &str,
    password: &str,
) -> Result<WdSession, String> {
    let scheme = if use_https { "https" } else { "http" };

    // WD My Cloud uses a cookie-based session
    let login_url = format!(
        "{}://{}:{}/api/2.1/rest/users?method=login",
        scheme, host, port
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .danger_accept_invalid_certs(true)
        .cookie_store(true)
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "username": username,
        "password": password
    });

    let resp = client
        .post(&login_url)
        .json(&body)
        .send()
        .map_err(|e| format!("WD My Cloud login error: {}", e))?;

    let status = resp.status();
    if status.is_success() {
        // Extract session token from response headers or body
        let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        let token = json["token"]
            .as_str()
            .or_else(|| json["data"]["token"].as_str())
            .or_else(|| json["session_id"].as_str())
            .unwrap_or("authenticated")
            .to_string();
        Ok(WdSession { client, token })
    } else {
        Err(format!("WD My Cloud auth failed (HTTP {})", status))
    }
}

fn wd_mycloud_get_shares(
    host: &str,
    port: u16,
    use_https: bool,
    session: &WdSession,
) -> Result<Vec<NasLibrary>, String> {
    let scheme = if use_https { "https" } else { "http" };
    let url = format!("{}://{}:{}/api/2.1/rest/shares", scheme, host, port);

    let resp = session
        .client
        .get(&url)
        .header("Authorization", format!("Bearer {}", session.token))
        .send()
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    let mut libraries = Vec::new();

    let shares_arr = json["shares"]
        .as_array()
        .or_else(|| json["data"].as_array())
        .or_else(|| json.as_array())
        .cloned()
        .unwrap_or_default();

    for share in shares_arr {
        let name = share["name"]
            .as_str()
            .or_else(|| share["share_name"].as_str())
            .unwrap_or("Share")
            .to_string();
        let path = share["path"]
            .as_str()
            .or_else(|| share["mount_path"].as_str())
            .unwrap_or("")
            .to_string();
        let size_bytes = share["total_size"]
            .as_u64()
            .or_else(|| share["size"].as_u64())
            .unwrap_or(0);
        let media_type = infer_media_type_from_name(&name);

        libraries.push(NasLibrary {
            id: format!("wd-{}", name.to_lowercase().replace(' ', "-")),
            name: name.clone(),
            path,
            share_name: name,
            media_type,
            item_count: 0,
            size_bytes,
        });
    }

    // If API returned nothing, provide a default Public share
    if libraries.is_empty() {
        libraries.push(NasLibrary {
            id: "wd-public".to_string(),
            name: "Public".to_string(),
            path: "/Public".to_string(),
            share_name: "Public".to_string(),
            media_type: "mixed".to_string(),
            item_count: 0,
            size_bytes: 0,
        });
    }

    Ok(libraries)
}

fn wd_mycloud_get_info(
    host: &str,
    port: u16,
    use_https: bool,
    session: &WdSession,
) -> (String, String, String) {
    let scheme = if use_https { "https" } else { "http" };
    let url = format!("{}://{}:{}/api/2.1/rest/device", scheme, host, port);

    if let Ok(resp) = session
        .client
        .get(&url)
        .header("Authorization", format!("Bearer {}", session.token))
        .send()
    {
        if let Ok(json) = resp.json::<serde_json::Value>() {
            let name = json["name"]
                .as_str()
                .or_else(|| json["device_name"].as_str())
                .unwrap_or("WD My Cloud")
                .to_string();
            let model = json["model"]
                .as_str()
                .or_else(|| json["device_model"].as_str())
                .unwrap_or("WD My Cloud")
                .to_string();
            let fw = json["firmware"]
                .as_str()
                .or_else(|| json["firmware_version"].as_str())
                .unwrap_or("")
                .to_string();
            return (name, model, fw);
        }
    }
    (
        "WD My Cloud".to_string(),
        "WD My Cloud".to_string(),
        "".to_string(),
    )
}

// ════════════════════════════════════════════════════════════
//  Helpers
// ════════════════════════════════════════════════════════════

fn infer_media_type_from_name(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("movie") || lower.contains("film") || lower.contains("cinema") {
        "movies".to_string()
    } else if lower.contains("tv")
        || lower.contains("series")
        || lower.contains("show")
        || lower.contains("episode")
    {
        "tv".to_string()
    } else if lower.contains("music") || lower.contains("audio") || lower.contains("song") {
        "music".to_string()
    } else if lower.contains("photo") || lower.contains("picture") || lower.contains("image") {
        "photos".to_string()
    } else {
        "mixed".to_string()
    }
}

fn urlencoding_simple(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

fn network_source_path(host: &str, share_name: &str, share_path: &str) -> String {
    let share = if share_name.trim().is_empty() {
        share_path
            .trim_matches(|character| character == '/' || character == '\\')
            .split(|character| character == '/' || character == '\\')
            .next()
            .unwrap_or("Public")
    } else {
        share_name.trim()
    };
    #[cfg(target_os = "windows")]
    {
        format!(r"\\{}\{}", host, share)
    }
    #[cfg(not(target_os = "windows"))]
    {
        format!("smb://{}/{}", host, share)
    }
}

#[cfg(target_os = "windows")]
fn authenticate_windows_shares(
    host: &str,
    libraries: &[NasLibrary],
    username: &str,
    password: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    for library in libraries {
        let remote = network_source_path(host, &library.share_name, &library.path);
        let user_argument = format!("/user:{username}");
        let mut command = Command::new("net");
        command.args([
            "use",
            remote.as_str(),
            password,
            user_argument.as_str(),
            "/persistent:no",
        ]);
        command.creation_flags(CREATE_NO_WINDOW);
        match command.output() {
            Ok(output) if output.status.success() => {}
            Ok(output) => errors.push(format!(
                "{}: {}",
                library.share_name,
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            Err(error) => errors.push(format!("{}: {error}", library.share_name)),
        }
    }
    errors
}

#[cfg(not(target_os = "windows"))]
fn authenticate_windows_shares(
    _host: &str,
    _libraries: &[NasLibrary],
    _username: &str,
    _password: &str,
) -> Vec<String> {
    Vec::new()
}

fn ensure_network_source_reachable(source_path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    if !Path::new(source_path).is_dir() {
        return Err(format!(
            "NAS share is not mounted or reachable: {source_path}. Reconnect with the NAS username and password, then try again."
        ));
    }
    Ok(())
}

fn normalize_nas_device_type(device_type: &str) -> Option<&'static str> {
    let normalized = device_type
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_");
    match normalized.as_str() {
        "synology" => Some("synology_connection"),
        "wd_mycloud" | "wdmycloud" => Some("wd_mycloud_connection"),
        _ => None,
    }
}

fn read_saved_nas_connection(
    db: &crate::db::Database,
    setting_key: &str,
) -> Result<Option<Value>, String> {
    db.get_setting_data(setting_key)
        .map_err(|error| error.to_string())?
        .filter(|value| !value.trim().is_empty())
        .map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn connection_libraries(connection: &Value) -> Vec<NasLibrary> {
    connection["libraries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|library| serde_json::from_value::<NasLibrary>(library.clone()).ok())
        .collect()
}

fn list_directory_entries(root: &Path) -> Result<Vec<Value>, String> {
    let mut entries = std::fs::read_dir(root)
        .map_err(|error| format!("Unable to read {}: {error}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            let metadata = entry.metadata().ok();
            serde_json::json!({
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

fn share_entries(libraries: &[NasLibrary]) -> Vec<Value> {
    libraries
        .iter()
        .map(|library| {
            serde_json::json!({
                "name": library.name,
                "path": library.path,
                "share_name": library.share_name,
                "media_type": library.media_type,
                "is_directory": true,
                "size": library.size_bytes,
            })
        })
        .collect()
}

fn resolve_browse_path(host: &str, requested_path: &str) -> PathBuf {
    let trimmed = requested_path.trim();
    let local = Path::new(trimmed);
    if local.is_dir() {
        return local.to_path_buf();
    }

    #[cfg(target_os = "windows")]
    {
        let relative = trimmed.trim_matches(|character| character == '/' || character == '\\');
        let mut segments = relative
            .split(|character| character == '/' || character == '\\')
            .filter(|segment| !segment.is_empty());
        let share_name = segments.next().unwrap_or("Public");
        let mut path = PathBuf::from(network_source_path(host, share_name, trimmed));
        for segment in segments {
            path.push(segment);
        }
        path
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = host;
        PathBuf::from(trimmed)
    }
}

// ════════════════════════════════════════════════════════════
//  Tauri Commands
// ════════════════════════════════════════════════════════════

/// Connect to a Synology NAS via QuickConnect ID or direct IP.
/// Resolves the QuickConnect ID, authenticates via DSM API,
/// and returns all shared folders as CinaVault libraries.
#[tauri::command]
pub fn synology_connect(
    state: State<AppState>,
    quickconnect_id: String,
    username: String,
    password: String,
    use_https: bool,
    port: Option<u16>,
) -> Result<NasConnectionResult, String> {
    log::info!("Synology connect: id={}", quickconnect_id);

    // Resolve host
    let host =
        if quickconnect_id.contains('.') || quickconnect_id.parse::<std::net::IpAddr>().is_ok() {
            // Direct IP or hostname provided
            quickconnect_id.clone()
        } else {
            // QuickConnect ID — resolve via relay
            resolve_quickconnect(&quickconnect_id)
                .unwrap_or_else(|_| format!("{}.quickconnect.to", quickconnect_id))
        };

    let resolved_port = port.unwrap_or(if use_https { 5001 } else { 5000 });

    // Authenticate
    let sid = synology_login(&host, resolved_port, use_https, &username, &password)?;

    // Get device info
    let (device_name, device_model, firmware) =
        synology_get_info(&host, resolved_port, use_https, &sid);

    // Get shared folders
    let libraries = synology_get_shares(&host, resolved_port, use_https, &sid)?;

    // Establish authenticated SMB sessions so discovered shares are real filesystem sources.
    let share_auth_errors = authenticate_windows_shares(&host, &libraries, &username, &password);
    if !share_auth_errors.is_empty() {
        log::warn!(
            "Some Synology shares were not mounted: {}",
            share_auth_errors.join("; ")
        );
    }

    // Persist the authenticated session without storing the plaintext password.
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let creds = serde_json::json!({
        "device_type": "synology",
        "host": host,
        "port": resolved_port,
        "use_https": use_https,
        "username": username,
        "sid": sid,
        "device_name": device_name,
        "device_model": device_model,
        "firmware": firmware,
        "libraries": libraries,
        "connected_at": chrono::Utc::now().to_rfc3339()
    });
    db.set_setting_data("synology_connection", &creds.to_string())
        .map_err(|e| e.to_string())?;

    Ok(NasConnectionResult {
        success: true,
        device_name,
        device_model,
        firmware,
        host_resolved: host,
        libraries,
        error: None,
    })
}

/// Disconnect from Synology NAS and clear stored credentials.
#[tauri::command]
pub fn synology_disconnect(state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_setting_data("synology_connection", "")
        .map_err(|e| e.to_string())?;
    log::info!("Synology disconnected");
    Ok(())
}

/// Get the current Synology connection status and libraries.
#[tauri::command]
pub fn synology_get_status(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let raw = db
        .get_setting_data("synology_connection")
        .map_err(|e| e.to_string())?;
    match raw {
        Some(s) if !s.is_empty() => {
            let val: serde_json::Value = serde_json::from_str(&s).unwrap_or(serde_json::json!({}));
            Ok(serde_json::json!({ "connected": true, "data": val }))
        }
        _ => Ok(serde_json::json!({ "connected": false })),
    }
}

/// Add a Synology shared folder as a CinaVault media source.
#[tauri::command]
pub fn synology_add_library(
    state: State<AppState>,
    share_name: String,
    share_path: String,
    media_type: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // Get connection info
    let raw = db
        .get_setting_data("synology_connection")
        .map_err(|e| e.to_string())?;
    let conn: serde_json::Value = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let host = conn["host"].as_str().unwrap_or("nas");
    let source_path = network_source_path(host, &share_name, &share_path);
    ensure_network_source_reachable(&source_path)?;
    let source = crate::db::MediaSource {
        id: None,
        path: source_path.clone(),
        source_type: media_type.clone(),
        name: share_name.clone(),
        enabled: true,
        last_scanned: None,
        item_count: 0,
    };
    db.add_source_data(&source).map_err(|e| e.to_string())?;
    log::info!("Synology library added: {} -> {}", share_name, source_path);
    Ok(())
}

/// Connect to a WD My Cloud device via local IP or hostname.
#[tauri::command]
pub fn wd_mycloud_connect(
    state: State<AppState>,
    host: String,
    username: String,
    password: String,
    use_https: bool,
    port: Option<u16>,
) -> Result<NasConnectionResult, String> {
    log::info!("WD My Cloud connect: host={}", host);

    let resolved_port = port.unwrap_or(if use_https { 443 } else { 80 });

    // Authenticate
    let session = wd_mycloud_login(&host, resolved_port, use_https, &username, &password)?;

    // Reuse the authenticated cookie jar and token for device/share requests.
    let (device_name, device_model, firmware) =
        wd_mycloud_get_info(&host, resolved_port, use_https, &session);
    let libraries = wd_mycloud_get_shares(&host, resolved_port, use_https, &session)?;
    let share_auth_errors = authenticate_windows_shares(&host, &libraries, &username, &password);
    if !share_auth_errors.is_empty() {
        log::warn!(
            "Some WD My Cloud shares were not mounted: {}",
            share_auth_errors.join("; ")
        );
    }

    // Persist the authenticated session without storing the plaintext password
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let creds = serde_json::json!({
        "device_type": "wd_mycloud",
        "host": host,
        "port": resolved_port,
        "use_https": use_https,
        "username": username,
        "token": session.token,
        "device_name": device_name,
        "device_model": device_model,
        "firmware": firmware,
        "libraries": libraries,
        "connected_at": chrono::Utc::now().to_rfc3339()
    });
    db.set_setting_data("wd_mycloud_connection", &creds.to_string())
        .map_err(|e| e.to_string())?;

    Ok(NasConnectionResult {
        success: true,
        device_name,
        device_model,
        firmware,
        host_resolved: host,
        libraries,
        error: None,
    })
}

/// Disconnect from WD My Cloud and clear stored credentials.
#[tauri::command]
pub fn wd_mycloud_disconnect(state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_setting_data("wd_mycloud_connection", "")
        .map_err(|e| e.to_string())?;
    log::info!("WD My Cloud disconnected");
    Ok(())
}

/// Get the current WD My Cloud connection status and libraries.
#[tauri::command]
pub fn wd_mycloud_get_status(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let raw = db
        .get_setting_data("wd_mycloud_connection")
        .map_err(|e| e.to_string())?;
    match raw {
        Some(s) if !s.is_empty() => {
            let val: serde_json::Value = serde_json::from_str(&s).unwrap_or(serde_json::json!({}));
            Ok(serde_json::json!({ "connected": true, "data": val }))
        }
        _ => Ok(serde_json::json!({ "connected": false })),
    }
}

/// Add a WD My Cloud share as a CinaVault media source.
#[tauri::command]
pub fn wd_mycloud_add_library(
    state: State<AppState>,
    share_name: String,
    share_path: String,
    media_type: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let raw = db
        .get_setting_data("wd_mycloud_connection")
        .map_err(|e| e.to_string())?;
    let conn: serde_json::Value = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let host = conn["host"].as_str().unwrap_or("wd-mycloud");
    let source_path = network_source_path(host, &share_name, &share_path);
    ensure_network_source_reachable(&source_path)?;
    let source = crate::db::MediaSource {
        id: None,
        path: source_path.clone(),
        source_type: media_type.clone(),
        name: share_name.clone(),
        enabled: true,
        last_scanned: None,
        item_count: 0,
    };
    db.add_source_data(&source).map_err(|e| e.to_string())?;
    log::info!(
        "WD My Cloud library added: {} -> {}",
        share_name,
        source_path
    );
    Ok(())
}

/// List shares exposed by currently connected NAS backends.
#[tauri::command]
pub fn list_nas_shares(
    state: State<AppState>,
    device_type: Option<String>,
) -> Result<Vec<NasLibrary>, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    let mut libraries = Vec::new();

    if let Some(device_type) = device_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let setting_key = normalize_nas_device_type(device_type)
            .ok_or_else(|| format!("Unsupported NAS device type: {device_type}"))?;
        if let Some(connection) = read_saved_nas_connection(&db, setting_key)? {
            libraries.extend(connection_libraries(&connection));
        }
    } else {
        for setting_key in ["synology_connection", "wd_mycloud_connection"] {
            if let Some(connection) = read_saved_nas_connection(&db, setting_key)? {
                libraries.extend(connection_libraries(&connection));
            }
        }
    }

    if libraries.is_empty() {
        return Err("No connected NAS shares were found.".to_string());
    }

    Ok(libraries)
}

/// Browse the root shares or a mounted NAS path for a connected device.
#[tauri::command]
pub fn browse_nas_path(
    state: State<AppState>,
    device_type: String,
    path: Option<String>,
) -> Result<Vec<Value>, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    let setting_key = normalize_nas_device_type(&device_type)
        .ok_or_else(|| format!("Unsupported NAS device type: {device_type}"))?;
    let connection = read_saved_nas_connection(&db, setting_key)?
        .ok_or_else(|| format!("No active NAS connection found for {device_type}"))?;
    let libraries = connection_libraries(&connection);
    let requested_path = path.unwrap_or_default();

    if requested_path.trim().is_empty() || requested_path == "/" || requested_path == "\\" {
        return Ok(share_entries(&libraries));
    }

    let host = connection["host"].as_str().unwrap_or_default();
    let browse_path = resolve_browse_path(host, &requested_path);
    list_directory_entries(&browse_path)
}

#[cfg(test)]
mod tests {
    use super::{
        connection_libraries, network_source_path, normalize_nas_device_type, share_entries,
        urlencoding_simple, NasLibrary,
    };

    #[test]
    fn nas_library_paths_are_scanner_compatible_network_paths() {
        let path = network_source_path("192.168.1.50", "Movies", "/Movies");
        #[cfg(target_os = "windows")]
        assert_eq!(path, r"\\192.168.1.50\Movies");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(path, "smb://192.168.1.50/Movies");
    }

    #[test]
    fn nas_credentials_are_url_encoded_for_synology_authentication() {
        assert_eq!(urlencoding_simple("name+space"), "name%2Bspace");
        assert_eq!(urlencoding_simple("p@ss word"), "p%40ss%20word");
    }

    #[test]
    fn nas_device_type_aliases_map_to_saved_connection_keys() {
        assert_eq!(
            normalize_nas_device_type("Synology"),
            Some("synology_connection")
        );
        assert_eq!(
            normalize_nas_device_type("WD My Cloud"),
            Some("wd_mycloud_connection")
        );
        assert_eq!(
            normalize_nas_device_type("wd-mycloud"),
            Some("wd_mycloud_connection")
        );
    }

    #[test]
    fn connected_nas_libraries_round_trip_from_saved_connection_json() {
        let connection = serde_json::json!({
            "libraries": [
                {
                    "id": "synology-movies",
                    "name": "Movies",
                    "path": "/Movies",
                    "share_name": "Movies",
                    "media_type": "movies",
                    "item_count": 0,
                    "size_bytes": 42
                }
            ]
        });

        assert_eq!(
            connection_libraries(&connection),
            vec![NasLibrary {
                id: "synology-movies".to_string(),
                name: "Movies".to_string(),
                path: "/Movies".to_string(),
                share_name: "Movies".to_string(),
                media_type: "movies".to_string(),
                item_count: 0,
                size_bytes: 42,
            }]
        );
    }

    #[test]
    fn browsing_root_share_entries_preserves_share_metadata() {
        let entries = share_entries(&[NasLibrary {
            id: "wd-public".to_string(),
            name: "Public".to_string(),
            path: "/Public".to_string(),
            share_name: "Public".to_string(),
            media_type: "mixed".to_string(),
            item_count: 0,
            size_bytes: 0,
        }]);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["name"], "Public");
        assert_eq!(entries[0]["path"], "/Public");
        assert_eq!(entries[0]["share_name"], "Public");
        assert_eq!(entries[0]["is_directory"], true);
    }
}
