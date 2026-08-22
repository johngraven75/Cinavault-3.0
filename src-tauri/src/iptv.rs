// CinaVault Premium — IPTV / Xtream Codes Module
use crate::AppState;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct XtreamProfile {
    pub id: Option<i64>,
    pub name: String,
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub enabled: bool,
    pub last_synced: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LiveChannel {
    pub id: Option<i64>,
    pub profile_id: i64,
    pub name: String,
    pub stream_url: String,
    pub logo_url: Option<String>,
    pub group_name: Option<String>,
    pub epg_id: Option<String>,
}

#[tauri::command]
pub fn add_xtream_profile(
    state: State<AppState>,
    name: String,
    server_url: String,
    username: String,
    password: String,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn.execute(
        "INSERT INTO xtream_profiles (name, server_url, username, password, enabled) VALUES (?1,?2,?3,?4,1)",
        params![name, server_url, username, password],
    ).map_err(|e| e.to_string())?;
    Ok(db.conn.last_insert_rowid())
}

#[tauri::command]
pub fn get_xtream_profiles(state: State<AppState>) -> Result<Vec<XtreamProfile>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.conn.prepare("SELECT id, name, server_url, username, password, enabled, last_synced FROM xtream_profiles")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(XtreamProfile {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                server_url: row.get(2)?,
                username: row.get(3)?,
                password: row.get(4)?,
                enabled: row.get(5)?,
                last_synced: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_xtream_profile(state: State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn
        .execute(
            "DELETE FROM live_channels WHERE profile_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM xtream_profiles WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn sync_xtream_streams(
    state: State<'_, AppState>,
    profile_id: i64,
) -> Result<serde_json::Value, String> {
    let profile = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .conn
            .prepare("SELECT server_url, username, password FROM xtream_profiles WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        stmt.query_row(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
    };

    let (server_url, username, password) = profile;
    let url = build_player_api_url(&server_url, &username, &password);

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let channels: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn
        .execute(
            "DELETE FROM live_channels WHERE profile_id = ?1",
            params![profile_id],
        )
        .map_err(|e| e.to_string())?;

    let mut count = 0u64;
    for ch in &channels {
        let name = ch.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let stream_id = ch.get("stream_id").and_then(|v| v.as_u64()).unwrap_or(0);
        let stream_url = build_live_stream_url(&server_url, &username, &password, stream_id);
        let logo = ch
            .get("stream_icon")
            .and_then(|v| v.as_str())
            .map(String::from);
        let group = ch
            .get("category_name")
            .and_then(|v| v.as_str())
            .map(String::from);
        let epg = ch
            .get("epg_channel_id")
            .and_then(|v| v.as_str())
            .map(String::from);

        db.conn.execute(
            "INSERT INTO live_channels (profile_id, name, stream_url, logo_url, group_name, epg_id) VALUES (?1,?2,?3,?4,?5,?6)",
            params![profile_id, name, stream_url, logo, group, epg],
        ).map_err(|e| e.to_string())?;
        count += 1;
    }

    let now = chrono::Utc::now().to_rfc3339();
    db.conn
        .execute(
            "UPDATE xtream_profiles SET last_synced = ?1 WHERE id = ?2",
            params![now, profile_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "channels_synced": count }))
}

#[tauri::command]
pub async fn sync_epg(
    state: State<'_, AppState>,
    profile_id: i64,
) -> Result<serde_json::Value, String> {
    let profile = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .conn
            .prepare("SELECT server_url, username, password FROM xtream_profiles WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        stmt.query_row(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
    };

    let (server_url, username, password) = profile;
    let url = build_epg_url(&server_url, &username, &password);

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let _epg_data = resp.text().await.map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "status": "epg_synced" }))
}

#[tauri::command]
pub fn get_live_channels(
    state: State<AppState>,
    profile_id: Option<i64>,
) -> Result<Vec<LiveChannel>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let sql = match profile_id {
        Some(_) => "SELECT id, profile_id, name, stream_url, logo_url, group_name, epg_id FROM live_channels WHERE profile_id = ?1 ORDER BY name",
        None => "SELECT id, profile_id, name, stream_url, logo_url, group_name, epg_id FROM live_channels ORDER BY name",
    };
    let mut stmt = db.conn.prepare(sql).map_err(|e| e.to_string())?;

    if let Some(pid) = profile_id {
        let rows = stmt
            .query_map(params![pid], row_to_live_channel)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    } else {
        let rows = stmt
            .query_map([], row_to_live_channel)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn play_channel(stream_url: String) -> Result<(), String> {
    open::that(&stream_url).map_err(|e| e.to_string())
}

fn row_to_live_channel(row: &rusqlite::Row) -> rusqlite::Result<LiveChannel> {
    Ok(LiveChannel {
        id: Some(row.get(0)?),
        profile_id: row.get(1)?,
        name: row.get(2)?,
        stream_url: row.get(3)?,
        logo_url: row.get(4)?,
        group_name: row.get(5)?,
        epg_id: row.get(6)?,
    })
}

fn build_player_api_url(server_url: &str, username: &str, password: &str) -> String {
    format!(
        "{}/player_api.php?username={}&password={}&action=get_live_streams",
        normalize_server_base(server_url),
        encode_url_component(username),
        encode_url_component(password),
    )
}

fn build_epg_url(server_url: &str, username: &str, password: &str) -> String {
    format!(
        "{}/xmltv.php?username={}&password={}",
        normalize_server_base(server_url),
        encode_url_component(username),
        encode_url_component(password),
    )
}

fn build_live_stream_url(
    server_url: &str,
    username: &str,
    password: &str,
    stream_id: u64,
) -> String {
    format!(
        "{}/live/{}/{}/{}.ts",
        normalize_server_base(server_url),
        encode_url_component(username),
        encode_url_component(password),
        stream_id,
    )
}

fn normalize_server_base(server_url: &str) -> String {
    server_url.trim().trim_end_matches('/').to_string()
}

fn encode_url_component(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

#[cfg(test)]
mod tests {
    use super::{build_live_stream_url, build_player_api_url};

    #[test]
    fn xtream_api_url_encodes_credentials_and_trims_server_slashes() {
        assert_eq!(
            build_player_api_url("http://provider.example.com:8080/", "viewer@example.com", "p&a s"),
            "http://provider.example.com:8080/player_api.php?username=viewer%40example%2Ecom&password=p%26a%20s&action=get_live_streams",
        );
    }

    #[test]
    fn xtream_live_url_encodes_path_credentials() {
        assert_eq!(
            build_live_stream_url("http://provider.example.com:8080/", "view er", "p/a", 42),
            "http://provider.example.com:8080/live/view%20er/p%2Fa/42.ts",
        );
    }
}
