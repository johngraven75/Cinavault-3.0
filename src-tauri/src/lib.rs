// CinaVault Premium — shared Tauri v2 backend for desktop and Android
// Build identity is sourced from build-version.json through build_identity.

mod adult_site_provider;
mod ai;
mod ai_automation;
mod atomic_file;
mod build_identity;
mod casting;
mod chapters;
mod cloud_storage;
mod db;
mod downloads;
mod duplicates;
mod embedded_server;
mod enrichment {
    include!(concat!(env!("OUT_DIR"), "/enrichment_atomic.rs"));
}
mod iptv;
mod jellyfin;
mod library_artifacts;
mod library_count;
mod media_tools;
mod metadata {
    include!(concat!(env!("OUT_DIR"), "/metadata_without_commands.rs"));
}
mod metadata_bridge;
mod metadata_enrichment_runtime;
mod metadata_ext {
    include!(concat!(
        env!("OUT_DIR"),
        "/metadata_ext_without_repaired_commands.rs"
    ));
}
mod metadata_guard {
    include!(concat!(
        env!("OUT_DIR"),
        "/metadata_guard_without_commands.rs"
    ));
}
mod metadata_keyless;
#[cfg(test)]
mod metadata_posting_tests;
mod metadata_provider_config;
mod nas_devices;
mod pgma_bridge;
mod player;
mod plugin_configs;
mod plugins;
mod remote_connectivity;
mod scanner;
mod shared_contracts;
mod source_health;
mod task_progress;
mod vpn;
mod vpn_profile_store;

use base64::Engine;
use db::Database;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Database>,
    pub app_data_dir: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_dir).ok();
            std::fs::create_dir_all(app_dir.join("artwork")).ok();

            let plugin_dirs = ["jellyfin", "emby", "plex", "native"];
            for dir in &plugin_dirs {
                std::fs::create_dir_all(app_dir.join("plugins").join(dir)).ok();
            }

            let db_path = app_dir.join("cinavault.db");
            let database = Database::new(db_path.to_str().unwrap())
                .expect("Failed to initialize database");

            // Recover the persistent Hugging Face credential on every launch if the DB
            // copy is missing. This keeps upgrades/reinstalls stable without embedding a
            // secret in the application binary or repository.
            let db_hf_token = database
                .get_setting_data("hf_token")
                .ok()
                .flatten()
                .filter(|token| !token.trim().is_empty());
            if db_hf_token.is_none() {
                let fallback_hf_token = std::env::var("CINAVAULT_HF_TOKEN")
                    .ok()
                    .or_else(|| std::env::var("HF_TOKEN").ok())
                    .filter(|token| !token.trim().is_empty())
                    .or_else(|| {
                        let token_path = dirs::home_dir()?
                            .join(".cache")
                            .join("huggingface")
                            .join("token");
                        let token = std::fs::read_to_string(token_path).ok()?;
                        let token = token.trim();
                        if token.starts_with("hf_") && token.len() > 20 {
                            Some(token.to_string())
                        } else {
                            None
                        }
                    });
                if let Some(token) = fallback_hf_token {
                    if let Err(error) = database.set_setting_data("hf_token", &token) {
                        log::warn!("Unable to restore persistent Hugging Face token: {error}");
                    } else {
                        log::info!("Persistent Hugging Face token restored at startup");
                    }
                }
            }

            // Initialize the full metadata-provider catalog on every launch. Existing
            // database credentials are retained, supported environment credentials are
            // imported once, and a provider-by-provider readiness report is persisted.
            match metadata_ext::initialize_metadata_providers(&database) {
                Ok(status) => log::info!("Metadata providers initialized at startup: {status}"),
                Err(error) => log::warn!("Metadata provider startup initialization failed: {error}"),
            }

            let resource_dir = app.path().resource_dir().unwrap_or_else(|_| app_dir.clone());
            remote_connectivity::configure(
                resource_dir
                    .join("tools")
                    .join("cloudflared")
                    .join("cloudflared.exe"),
            );

            // Permanent media tools are a startup dependency. Verify and automatically
            // repair yt-dlp/FFmpeg/FFprobe/MediaInfo/MKVToolNix before the UI is ready.
            match media_tools::ensure_media_tools() {
                Ok(status) => {
                    if status.get("ready").and_then(|value| value.as_bool()).unwrap_or(false) {
                        log::info!("Permanent media tools are ready at launch");
                    } else {
                        log::warn!("Permanent media tools startup repair completed with missing tools: {status}");
                    }
                }
                Err(error) => log::warn!("Permanent media tools startup repair failed: {error}"),
            }

            app.manage(AppState {
                db: Mutex::new(database),
                app_data_dir: app_dir,
            });
            log::info!("{} initialized successfully", build_identity::current().display_name);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            db::get_all_settings,
            db::get_setting,
            db::set_setting,
            db::get_feature_settings,
            db::set_feature_setting,
            db::create_remote_access_user,
            db::authenticate_remote_password,
            db::authenticate_remote_access_key,
            db::rotate_remote_access_key,
            db::set_remote_access_user_enabled,
            db::list_remote_access_users,
            db::get_remote_access_security_status,
            db::get_media_items,
            db::get_media_item,
            get_poster_data_url,
            db::add_media_item,
            db::update_media_item,
            db::delete_media_item,
            db::purge_photo_items,
            db::search_media,
            db::get_recent_media,
            db::get_unverified_media,
            library_count::get_library_count,
            db::verify_media_item,
            db::get_sources,
            db::add_source,
            db::remove_source,
            scanner::scan_sources,
            scanner::scan_single_source,
            scanner::discover_media_sources,
            scanner::get_scan_progress,
            scanner::cancel_scan,
            scanner::apply_embedded_titles,
            source_health::validate_source_path,
            source_health::explore_source_path,
            duplicates::find_duplicates,
            duplicates::get_duplicate_groups,
            duplicates::remove_duplicate,
            duplicates::quarantine,
            iptv::add_xtream_profile,
            iptv::get_xtream_profiles,
            iptv::remove_xtream_profile,
            iptv::sync_xtream_streams,
            iptv::sync_epg,
            iptv::get_live_channels,
            iptv::play_channel,
            jellyfin::start_server,
            jellyfin::stop_server,
            jellyfin::get_server_status,
            jellyfin::get_server_info,
            jellyfin::import_libraries,
            jellyfin::check_emby_compat,
            jellyfin::open_admin_page,
            embedded_server::start_embedded_server,
            embedded_server::stop_embedded_server,
            embedded_server::get_embedded_server_status,
            remote_connectivity::start_remote_connectivity,
            remote_connectivity::stop_remote_connectivity,
            remote_connectivity::get_remote_connectivity_status,
            plugins::get_plugin_repos,
            plugins::add_plugin_repo,
            plugins::remove_plugin_repo,
            plugins::sync_plugin_catalog,
            plugins::get_plugin_catalog,
            plugins::install_plugin,
            plugins::uninstall_plugin,
            plugins::run_plugin,
            plugins::get_installed_plugins,
            plugin_configs::ensure_plugin_config_files,
            pgma_bridge::find_local_candidates,
            pgma_bridge::refresh_pgma_library,
            player::play_media,
            player::get_available_players,
            player::set_default_player,
            casting::discover_casting_devices,
            casting::connect_casting_device,
            casting::disconnect_casting_device,
            casting::start_casting,
            casting::update_casting_playback,
            metadata_ext::fetch_metadata,
            metadata_ext::search_metadata,
            metadata_enrichment_runtime::check_media_item_metadata,
            metadata_ext::get_provider_status,
            metadata_ext::test_api_key,
            metadata_ext::set_api_key,
            metadata_ext::get_api_keys,
            metadata_ext::get_metadata_providers,
            chapters::generate_chapter_thumbs,
            chapters::get_chapter_thumbs,
            downloads::start_download,
            downloads::start_media_download,
            downloads::start_playlist_download,
            downloads::crawl_media_links,
            downloads::get_supported_media_types,
            downloads::get_download_progress,
            downloads::cancel_download,
            downloads::install_download_tools,
            downloads::check_download_tools,
            media_tools::get_media_tools_status,
            media_tools::ensure_media_tools,
            media_tools::inspect_with_mediainfo,
            media_tools::inspect_with_mkvtoolnix,
            vpn::vpn_connect,
            vpn::vpn_disconnect,
            vpn::vpn_status,
            vpn::vpn_import_profile,
            vpn::vpn_profiles,
            vpn::run_antivirus_scan,
            vpn::update_av_signatures,
            vpn::install_security_tools,
            ai::ai_query,
            ai::ai_inference,
            ai::set_hf_token,
            ai::ensure_hf_token,
            ai::get_ai_config,
            ai::set_ai_model,
            ai_automation::ai_library_manage,
            metadata_enrichment_runtime::run_library_enrichment,
            enrichment::gather_adult_metadata,
            convert_entire_library_to_adult,
            task_progress::get_metadata_task_progress,
            task_progress::stop_ai_agent,
            cloud_storage::cloud_auth_start,
            cloud_storage::cloud_disconnect,
            cloud_storage::cloud_sync,
            cloud_storage::cloud_browse,
            cloud_storage::cloud_list_files,
            cloud_storage::cloud_get_status,
            nas_devices::synology_connect,
            nas_devices::synology_disconnect,
            nas_devices::synology_get_status,
            nas_devices::list_nas_shares,
            nas_devices::browse_nas_path,
            nas_devices::synology_add_library,
            nas_devices::wd_mycloud_connect,
            nas_devices::wd_mycloud_disconnect,
            nas_devices::wd_mycloud_get_status,
            nas_devices::wd_mycloud_add_library,
            nas_devices::list_nas_shares,
            nas_devices::browse_nas_path,
            build_identity::get_current_build_info,
            open_external_url,
            get_system_info,
            pick_folder,
            pick_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CinaVault Premium");
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    open::that(url).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_system_info() -> serde_json::Value {
    serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "family": std::env::consts::FAMILY
    })
}

#[tauri::command]
fn get_poster_data_url(state: tauri::State<AppState>, path: String) -> Result<String, String> {
    const MAX_POSTER_BYTES: u64 = 25 * 1024 * 1024;
    let requested = path.trim();
    if requested.is_empty() || requested.starts_with("http://") || requested.starts_with("https://")
    {
        return Err("A local poster path is required".to_string());
    }

    let authorized = {
        let db = state.db.lock().map_err(|error| error.to_string())?;
        db.conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM media_items WHERE poster_path = ?1 OR backdrop_path = ?1)",
                rusqlite::params![requested],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| error.to_string())?
            == 1
    };
    if !authorized {
        return Err("Poster is not attached to a library item".to_string());
    }

    let candidate = std::path::Path::new(requested);
    let metadata =
        std::fs::metadata(candidate).map_err(|error| format!("Poster unavailable: {error}"))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_POSTER_BYTES {
        return Err("Poster file is empty, oversized, or not a regular file".to_string());
    }
    let bytes = std::fs::read(candidate).map_err(|error| format!("Poster read failed: {error}"))?;
    let extension = candidate
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mime = match extension.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    };
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[tauri::command]
async fn convert_entire_library_to_adult(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let labeling = {
        let mut db = state.db.lock().map_err(|error| error.to_string())?;
        db.mark_current_library_adult()
            .map_err(|error| error.to_string())?
    };

    let enrichment = enrichment::gather_adult_metadata(state).await?;
    Ok(serde_json::json!({
        "type": "entire_library_adult_conversion",
        "status": "success",
        "inventory_items": labeling.inventory_items,
        "items_labeled_adult": labeling.items_labeled_adult,
        "items_already_adult": labeling.items_already_adult,
        "poster_references_preserved": true,
        "poster_files_deleted": 0,
        "future_imports_affected": false,
        "adult_enrichment": enrichment,
    }))
}

#[tauri::command]
fn pick_folder() -> Result<Option<String>, String> {
    Ok(None)
}

#[tauri::command]
fn pick_file() -> Result<Option<String>, String> {
    Ok(None)
}
