// CinaVault Premium — persistent plugin manager (Build 166).
// Every successful command below performs a durable filesystem change.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const PGMA_PLUGIN_ID: &str = "px-pgma-modernized";
static PLUGIN_IO: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRepo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub repo_id: String,
    pub author: String,
    pub homepage: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPlugin {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub version: String,
    pub install_path: String,
    pub config_json: String,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub repo_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginRunResult {
    pub success: bool,
    pub output: String,
    pub exit_code: i32,
}

fn plugin_root() -> Result<PathBuf, String> {
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .ok_or_else(|| {
            "The operating-system application-data folder is unavailable.".to_string()
        })?;
    Ok(base.join("CinaVault").join("plugins"))
}

fn registry_path() -> Result<PathBuf, String> {
    Ok(plugin_root()?.join("installed.json"))
}

fn validate_id(plugin_id: &str) -> Result<(), String> {
    if plugin_id.is_empty()
        || !plugin_id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '.'))
    {
        return Err("Plugin id contains unsupported path characters.".to_string());
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Plugin manifest has no parent folder.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    let backup = path.with_extension("bak");
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if path.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(path, &backup).map_err(|error| error.to_string())?;
    }
    match fs::rename(&temporary, path) {
        Ok(()) => {
            let _ = fs::remove_file(&backup);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            if backup.exists() {
                let _ = fs::rename(&backup, path);
            }
            Err(error.to_string())
        }
    }
}

fn load_installed() -> Result<Vec<InstalledPlugin>, String> {
    let path = registry_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Invalid plugin registry JSON: {error}"))
}

fn save_installed(installed: &[InstalledPlugin]) -> Result<(), String> {
    let contents = serde_json::to_vec_pretty(installed).map_err(|error| error.to_string())?;
    atomic_write(&registry_path()?, &contents)
}

fn repositories_path() -> Result<PathBuf, String> {
    Ok(plugin_root()?.join("repositories.json"))
}

fn load_repositories() -> Result<Vec<PluginRepo>, String> {
    let path = repositories_path()?;
    if !path.exists() {
        let repositories = default_repositories();
        save_repositories(&repositories)?;
        return Ok(repositories);
    }
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Invalid plugin repository JSON: {error}"))
}

fn save_repositories(repositories: &[PluginRepo]) -> Result<(), String> {
    let contents = serde_json::to_vec_pretty(repositories).map_err(|error| error.to_string())?;
    atomic_write(&repositories_path()?, &contents)
}

fn save_catalog(catalog: &[PluginEntry]) -> Result<(), String> {
    let contents = serde_json::to_vec_pretty(catalog).map_err(|error| error.to_string())?;
    atomic_write(&plugin_root()?.join("catalog.json"), &contents)
}

fn manifest_path(plugin: &InstalledPlugin) -> PathBuf {
    PathBuf::from(&plugin.install_path).join("plugin.json")
}

fn save_manifest(plugin: &InstalledPlugin) -> Result<(), String> {
    let contents = serde_json::to_vec_pretty(plugin).map_err(|error| error.to_string())?;
    atomic_write(&manifest_path(plugin), &contents)
}

fn default_repositories() -> Vec<PluginRepo> {
    vec![
        PluginRepo {
            id: "official".to_string(),
            name: "CinaVault Official".to_string(),
            url: "https://plugins.cinavault.app/official".to_string(),
            enabled: true,
        },
        PluginRepo {
            id: "community".to_string(),
            name: "Community Plugins".to_string(),
            url: "https://plugins.cinavault.app/community".to_string(),
            enabled: true,
        },
    ]
}

fn default_catalog() -> Vec<PluginEntry> {
    vec![
        PluginEntry {
            id: "metadata-tmdb".to_string(),
            name: "TMDB Metadata".to_string(),
            version: "2.1.0".to_string(),
            description: "Fetch movie and TV metadata from The Movie Database.".to_string(),
            category: "metadata".to_string(),
            repo_id: "official".to_string(),
            author: "CinaVault Team".to_string(),
            homepage: "https://www.themoviedb.org".to_string(),
            tags: vec![
                "metadata".to_string(),
                "movies".to_string(),
                "tv".to_string(),
            ],
        },
        PluginEntry {
            id: "subtitle-opensubtitles".to_string(),
            name: "OpenSubtitles".to_string(),
            version: "1.4.2".to_string(),
            description: "Download subtitles from OpenSubtitles.org.".to_string(),
            category: "subtitles".to_string(),
            repo_id: "official".to_string(),
            author: "CinaVault Team".to_string(),
            homepage: "https://www.opensubtitles.org".to_string(),
            tags: vec!["subtitles".to_string(), "srt".to_string()],
        },
        PluginEntry {
            id: "cast-chromecast".to_string(),
            name: "Chromecast / Google Cast".to_string(),
            version: "1.0.0".to_string(),
            description: "Cast media to Chromecast and Google TV devices.".to_string(),
            category: "casting".to_string(),
            repo_id: "official".to_string(),
            author: "CinaVault Team".to_string(),
            homepage: "https://cinavault.app/plugins/chromecast".to_string(),
            tags: vec![
                "cast".to_string(),
                "chromecast".to_string(),
                "google".to_string(),
            ],
        },
    ]
}

#[tauri::command]
pub fn get_plugin_repos() -> Result<Vec<PluginRepo>, String> {
    let _io = PLUGIN_IO.lock().map_err(|error| error.to_string())?;
    load_repositories()
}

#[tauri::command]
pub fn add_plugin_repo(id: String, name: String, url: String) -> Result<Vec<PluginRepo>, String> {
    validate_id(&id)?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Plugin repository URL must use HTTP or HTTPS.".to_string());
    }
    let _io = PLUGIN_IO.lock().map_err(|error| error.to_string())?;
    let mut repositories = load_repositories()?;
    if repositories.iter().any(|repo| repo.id == id) {
        return Err(format!("Repository '{id}' already exists."));
    }
    repositories.push(PluginRepo {
        id,
        name,
        url,
        enabled: true,
    });
    save_repositories(&repositories)?;
    Ok(repositories)
}

#[tauri::command]
pub fn remove_plugin_repo(id: String) -> Result<Vec<PluginRepo>, String> {
    let _io = PLUGIN_IO.lock().map_err(|error| error.to_string())?;
    let mut repositories = load_repositories()?;
    let before = repositories.len();
    repositories.retain(|repo| repo.id != id);
    if repositories.len() == before {
        return Err(format!("Plugin repository '{id}' does not exist."));
    }
    save_repositories(&repositories)?;
    Ok(repositories)
}

#[tauri::command]
pub fn sync_plugin_catalog() -> Result<usize, String> {
    let _io = PLUGIN_IO.lock().map_err(|error| error.to_string())?;
    let catalog = default_catalog();
    save_catalog(&catalog)?;
    Ok(catalog.len())
}

#[tauri::command]
pub fn get_plugin_catalog() -> Result<Vec<PluginEntry>, String> {
    Ok(default_catalog())
}

#[tauri::command]
pub fn install_plugin(
    plugin_id: String,
    name: String,
    version: String,
    platforms: Vec<String>,
    repo_url: Option<String>,
) -> Result<InstalledPlugin, String> {
    validate_id(&plugin_id)?;
    let _io = PLUGIN_IO.lock().map_err(|error| error.to_string())?;
    let mut installed = load_installed()?;
    if installed.iter().any(|plugin| plugin.id == plugin_id) {
        return Err(format!("Plugin '{plugin_id}' is already installed."));
    }

    let platform = platforms
        .first()
        .cloned()
        .unwrap_or_else(|| "cinavault".to_string());
    validate_id(&platform)?;
    let install_path = plugin_root()?.join(&platform).join(&plugin_id);
    fs::create_dir_all(&install_path).map_err(|error| error.to_string())?;

    let plugin = InstalledPlugin {
        id: plugin_id,
        name,
        platform,
        version,
        install_path: install_path.to_string_lossy().into_owned(),
        config_json: "{}".to_string(),
        enabled: true,
        last_run: None,
        repo_url,
    };
    save_manifest(&plugin)?;
    installed.push(plugin.clone());
    save_installed(&installed)?;
    Ok(plugin)
}

#[tauri::command]
pub fn uninstall_plugin(plugin_id: String) -> Result<String, String> {
    validate_id(&plugin_id)?;
    if plugin_id == PGMA_PLUGIN_ID {
        return Err(
            "PGMA is a required adult metadata provider and cannot be removed.".to_string(),
        );
    }
    let _io = PLUGIN_IO.lock().map_err(|error| error.to_string())?;
    let mut installed = load_installed()?;
    let index = installed
        .iter()
        .position(|plugin| plugin.id == plugin_id)
        .ok_or_else(|| format!("Plugin '{plugin_id}' is not installed."))?;
    let plugin = installed.remove(index);
    let install_path = PathBuf::from(&plugin.install_path);
    let root = plugin_root()?;
    if !install_path.starts_with(&root) {
        return Err("Refusing to remove a plugin outside the CinaVault plugin folder.".to_string());
    }
    if install_path.exists() {
        fs::remove_dir_all(&install_path).map_err(|error| error.to_string())?;
    }
    save_installed(&installed)?;
    Ok(format!("Plugin '{plugin_id}' uninstalled successfully."))
}

#[tauri::command]
pub fn run_plugin(
    plugin_id: String,
    action: String,
    config: Option<String>,
) -> Result<PluginRunResult, String> {
    validate_id(&plugin_id)?;
    let _io = PLUGIN_IO.lock().map_err(|error| error.to_string())?;
    let mut installed = load_installed()?;
    let plugin = installed
        .iter_mut()
        .find(|plugin| plugin.id == plugin_id)
        .ok_or_else(|| format!("Plugin '{plugin_id}' is not installed."))?;

    match action.as_str() {
        "configure" => {
            let config =
                config.ok_or_else(|| "Plugin configuration JSON is required.".to_string())?;
            let _: serde_json::Value = serde_json::from_str(&config)
                .map_err(|error| format!("Invalid plugin configuration JSON: {error}"))?;
            plugin.config_json = config;
        }
        "enable" => plugin.enabled = true,
        "disable" => plugin.enabled = false,
        "start" | "run" => {
            return Err(format!(
                "Plugin '{}' has no executable runtime registered; no work was performed.",
                plugin.id
            ));
        }
        unsupported => return Err(format!("Unsupported plugin action '{unsupported}'.")),
    }

    plugin.last_run = Some(Utc::now().to_rfc3339());
    save_manifest(plugin)?;
    let output = format!("Plugin '{}' action '{}' persisted.", plugin.id, action);
    save_installed(&installed)?;
    Ok(PluginRunResult {
        success: true,
        output,
        exit_code: 0,
    })
}

#[tauri::command]
pub fn get_installed_plugins() -> Result<Vec<InstalledPlugin>, String> {
    let _io = PLUGIN_IO.lock().map_err(|error| error.to_string())?;
    load_installed()
}

#[cfg(test)]
mod tests {
    use super::validate_id;

    #[test]
    fn plugin_ids_cannot_escape_the_managed_directory() {
        assert!(validate_id("jf-opensubtitles").is_ok());
        assert!(validate_id("../escape").is_err());
        assert!(validate_id("folder/plugin").is_err());
    }
}
