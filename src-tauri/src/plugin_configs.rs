// CinaVault Premium — permanent plugin JSON configuration provisioning.
// Creates a valid default JSON template for every catalog option and keeps a
// physical config.json attached to every installed plugin directory.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfigSeed {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub platform: String,
    pub category: String,
    pub configurable: bool,
    pub default_config: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfigReport {
    pub templates_written: usize,
    pub installed_configs_written: usize,
    pub registry_entries_repaired: usize,
    pub config_root: String,
}

fn plugin_root() -> Result<PathBuf, String> {
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .ok_or_else(|| {
            "The operating-system application-data folder is unavailable.".to_string()
        })?;
    Ok(base.join("CinaVault").join("plugins"))
}

fn validate_id(plugin_id: &str) -> Result<(), String> {
    if plugin_id.is_empty()
        || !plugin_id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '.'))
    {
        return Err(format!(
            "Plugin id contains unsupported path characters: {plugin_id}"
        ));
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Configuration file has no parent folder.".to_string())?;
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

fn normalized_default(seed: &PluginConfigSeed) -> Value {
    let mut object = match seed.default_config.clone() {
        Value::Object(values) => values,
        _ => Map::new(),
    };
    object
        .entry("schemaVersion".to_string())
        .or_insert(Value::from(1));
    object
        .entry("pluginId".to_string())
        .or_insert(Value::from(seed.plugin_id.clone()));
    object
        .entry("name".to_string())
        .or_insert(Value::from(seed.name.clone()));
    object
        .entry("version".to_string())
        .or_insert(Value::from(seed.version.clone()));
    object
        .entry("platform".to_string())
        .or_insert(Value::from(seed.platform.clone()));
    object
        .entry("category".to_string())
        .or_insert(Value::from(seed.category.clone()));
    object
        .entry("configurable".to_string())
        .or_insert(Value::from(seed.configurable));
    object
        .entry("enabled".to_string())
        .or_insert(Value::from(true));
    object
        .entry("source".to_string())
        .or_insert(Value::from("cinavault-default"));
    Value::Object(object)
}

fn merge_objects(defaults: &Value, current: Option<&Value>) -> Value {
    let mut merged = defaults.as_object().cloned().unwrap_or_default();
    if let Some(Value::Object(current)) = current {
        for (key, value) in current {
            merged.insert(key.clone(), value.clone());
        }
    }
    Value::Object(merged)
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    atomic_write(path, &bytes)
}

#[tauri::command]
pub fn ensure_plugin_config_files(
    seeds: Vec<PluginConfigSeed>,
) -> Result<PluginConfigReport, String> {
    let root = plugin_root()?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let defaults_root = root.join("default-configs");
    fs::create_dir_all(&defaults_root).map_err(|error| error.to_string())?;

    let mut defaults_by_id = HashMap::new();
    let mut templates_written = 0usize;
    for seed in &seeds {
        validate_id(&seed.plugin_id)?;
        let defaults = normalized_default(seed);
        write_json(
            &defaults_root.join(format!("{}.json", seed.plugin_id)),
            &defaults,
        )?;
        defaults_by_id.insert(seed.plugin_id.clone(), defaults);
        templates_written += 1;
    }

    let registry_path = root.join("installed.json");
    let mut installed = if registry_path.exists() {
        let text = fs::read_to_string(&registry_path).map_err(|error| error.to_string())?;
        serde_json::from_str::<Vec<Value>>(&text)
            .map_err(|error| format!("Invalid plugin registry JSON: {error}"))?
    } else {
        Vec::new()
    };

    let mut installed_configs_written = 0usize;
    let mut registry_entries_repaired = 0usize;
    for plugin in &mut installed {
        let Some(object) = plugin.as_object_mut() else {
            continue;
        };
        let Some(plugin_id) = object.get("id").and_then(Value::as_str).map(str::to_string) else {
            continue;
        };
        validate_id(&plugin_id)?;

        let fallback = defaults_by_id.get(&plugin_id).cloned().unwrap_or_else(|| {
            let mut generic = Map::new();
            generic.insert("schemaVersion".to_string(), Value::from(1));
            generic.insert("pluginId".to_string(), Value::from(plugin_id.clone()));
            generic.insert("enabled".to_string(), Value::from(true));
            generic.insert("source".to_string(), Value::from("cinavault-generated"));
            Value::Object(generic)
        });

        let current = object
            .get("configJson")
            .and_then(Value::as_str)
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .filter(Value::is_object);
        let merged = merge_objects(&fallback, current.as_ref());
        let pretty = serde_json::to_string_pretty(&merged).map_err(|error| error.to_string())?;
        if object.get("configJson").and_then(Value::as_str) != Some(pretty.as_str()) {
            object.insert("configJson".to_string(), Value::from(pretty));
            registry_entries_repaired += 1;
        }

        let install_path = object
            .get("installPath")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("native").join(&plugin_id));
        fs::create_dir_all(&install_path).map_err(|error| error.to_string())?;
        write_json(&install_path.join("config.json"), &merged)?;
        installed_configs_written += 1;
    }

    write_json(&registry_path, &Value::Array(installed))?;

    Ok(PluginConfigReport {
        templates_written,
        installed_configs_written,
        registry_entries_repaired,
        config_root: root.to_string_lossy().into_owned(),
    })
}
