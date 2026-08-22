use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize)]
pub struct StoredVpnProfile {
    pub name: String,
    pub path: String,
    pub active: bool,
}

pub fn profile_directory(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("unable to resolve app data directory: {error}"))?
        .join("vpn")
        .join("profiles");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("unable to create VPN profile directory: {error}"))?;
    restrict_to_current_user(&directory)?;
    Ok(directory)
}

pub fn validate_profile(content: &str) -> Result<(), String> {
    let normalized = content.replace('\r', "");
    for required in [
        "[Interface]",
        "PrivateKey",
        "[Peer]",
        "PublicKey",
        "Endpoint",
        "AllowedIPs",
    ] {
        if !normalized.contains(required) {
            return Err(format!(
                "WireGuard profile is missing required field: {required}"
            ));
        }
    }
    if normalized.contains("PrivateKey =") || normalized.contains("PrivateKey=") {
        Ok(())
    } else {
        Err("WireGuard profile has no private key value".to_string())
    }
}

pub fn import_profile(app: &AppHandle, source_path: &str) -> Result<StoredVpnProfile, String> {
    let source = Path::new(source_path);
    if !source.is_file() {
        return Err("selected WireGuard profile does not exist or is not a file".to_string());
    }
    if !source
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("conf"))
        .unwrap_or(false)
    {
        return Err("WireGuard profile must use the .conf extension".to_string());
    }

    let content = std::fs::read_to_string(source)
        .map_err(|error| format!("unable to read WireGuard profile: {error}"))?;
    validate_profile(&content)?;

    let raw_name = source
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("WireGuard profile has an invalid filename")?;
    let name = sanitize_profile_name(raw_name)?;
    let destination = profile_directory(app)?.join(format!("{name}.conf"));
    let temporary = destination.with_extension("conf.part");

    std::fs::write(&temporary, content.as_bytes())
        .map_err(|error| format!("unable to stage WireGuard profile: {error}"))?;
    restrict_to_current_user(&temporary)?;
    if destination.exists() {
        std::fs::remove_file(&destination)
            .map_err(|error| format!("unable to replace existing WireGuard profile: {error}"))?;
    }
    std::fs::rename(&temporary, &destination)
        .map_err(|error| format!("unable to store WireGuard profile: {error}"))?;
    restrict_to_current_user(&destination)?;

    Ok(StoredVpnProfile {
        name,
        path: destination.to_string_lossy().to_string(),
        active: false,
    })
}

pub fn list_profiles(
    app: &AppHandle,
    active_name: Option<&str>,
) -> Result<Vec<StoredVpnProfile>, String> {
    let mut profiles = Vec::new();
    for entry in std::fs::read_dir(profile_directory(app)?)
        .map_err(|error| format!("unable to list WireGuard profiles: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("unable to inspect WireGuard profile: {error}"))?;
        let path = entry.path();
        if !path.is_file()
            || !path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case("conf"))
                .unwrap_or(false)
        {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        profiles.push(StoredVpnProfile {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            active: active_name.map(|value| value == name).unwrap_or(false),
        });
    }
    profiles.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(profiles)
}

pub fn profile_path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let name = sanitize_profile_name(name)?;
    let path = profile_directory(app)?.join(format!("{name}.conf"));
    if !path.is_file() {
        return Err(format!("WireGuard profile '{name}' is not stored"));
    }
    Ok(path)
}

fn sanitize_profile_name(name: &str) -> Result<String, String> {
    let sanitized: String = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .take(64)
        .collect();
    if sanitized.is_empty() {
        Err("WireGuard profile filename must contain letters or numbers".to_string())
    } else {
        Ok(sanitized)
    }
}

#[cfg(target_os = "windows")]
fn restrict_to_current_user(path: &Path) -> Result<(), String> {
    let mut command = Command::new("icacls");
    command
        .arg(path)
        .args(["/inheritance:r", "/grant:r", "%USERNAME%:(F)"])
        .creation_flags(CREATE_NO_WINDOW);
    let output = command
        .output()
        .map_err(|error| format!("unable to secure VPN profile permissions: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "unable to secure VPN profile permissions: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(not(target_os = "windows"))]
fn restrict_to_current_user(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{sanitize_profile_name, validate_profile};

    #[test]
    fn validates_complete_wireguard_profile() {
        let profile = "[Interface]\nPrivateKey = secret\n[Peer]\nPublicKey = public\nEndpoint = vpn.example:51820\nAllowedIPs = 0.0.0.0/0";
        assert!(validate_profile(profile).is_ok());
    }

    #[test]
    fn rejects_incomplete_wireguard_profile() {
        assert!(validate_profile("[Interface]\nPrivateKey = secret").is_err());
    }

    #[test]
    fn sanitizes_profile_names() {
        assert_eq!(sanitize_profile_name("Home VPN").unwrap(), "HomeVPN");
        assert!(sanitize_profile_name("...").is_err());
    }
}
