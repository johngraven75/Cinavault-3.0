// CinaVault Premium — bundled WireGuard VPN and Windows Defender integration.
use crate::vpn_profile_store;
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn wireguard_executable(_app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let mut candidates = vec![PathBuf::from(
            r"C:\\Program Files\\WireGuard\\wireguard.exe",
        )];
        for variable in ["PROGRAMW6432", "PROGRAMFILES"] {
            if let Some(root) = std::env::var_os(variable) {
                candidates.push(PathBuf::from(root).join("WireGuard").join("wireguard.exe"));
            }
        }
        candidates
            .into_iter()
            .find(|path| path.is_file())
            .ok_or_else(|| {
                "WireGuard is not installed. Install the official WireGuard for Windows client before connecting a tunnel."
                    .to_string()
            })
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("WireGuard tunnels are currently supported on Windows only".to_string())
    }
}

#[cfg(target_os = "windows")]
fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(not(target_os = "windows"))]
fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    Command::new(program)
}

fn tunnel_service_name(profile_name: &str) -> String {
    format!("WireGuardTunnel${profile_name}")
}

fn service_is_running(profile_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        hidden_command("sc.exe")
            .args(["query", &tunnel_service_name(profile_name)])
            .output()
            .map(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout)
                        .to_ascii_uppercase()
                        .contains("RUNNING")
            })
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = profile_name;
        false
    }
}

#[tauri::command]
pub async fn vpn_import_profile(
    app: AppHandle,
    source_path: String,
) -> Result<vpn_profile_store::StoredVpnProfile, String> {
    vpn_profile_store::import_profile(&app, &source_path)
}

#[tauri::command]
pub async fn vpn_profiles(
    app: AppHandle,
) -> Result<Vec<vpn_profile_store::StoredVpnProfile>, String> {
    let profiles = vpn_profile_store::list_profiles(&app, None)?;
    Ok(profiles
        .into_iter()
        .map(|mut profile| {
            profile.active = service_is_running(&profile.name);
            profile
        })
        .collect())
}

#[tauri::command]
pub async fn vpn_connect(app: AppHandle, profile: String) -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    {
        let executable = wireguard_executable(&app)?;
        let profile_path = vpn_profile_store::profile_path(&app, &profile)?;
        let output = hidden_command(&executable)
            .arg("/installtunnelservice")
            .arg(&profile_path)
            .output()
            .map_err(|error| format!("failed to start the installed WireGuard engine: {error}"))?;
        if !output.status.success() && !service_is_running(&profile) {
            return Err(format!(
                "WireGuard tunnel failed to start: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(serde_json::json!({
            "status": "connected",
            "profile": profile,
            "service": tunnel_service_name(&profile),
            "engine": executable.to_string_lossy(),
        }))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, profile);
        Err("bundled WireGuard tunnels are currently supported on Windows only".to_string())
    }
}

#[tauri::command]
pub async fn vpn_disconnect(app: AppHandle) -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    {
        let executable = wireguard_executable(&app)?;
        let profiles = vpn_profile_store::list_profiles(&app, None)?;
        let active: Vec<String> = profiles
            .into_iter()
            .filter(|profile| service_is_running(&profile.name))
            .map(|profile| profile.name)
            .collect();
        for profile in &active {
            let output = hidden_command(&executable)
                .arg("/uninstalltunnelservice")
                .arg(profile)
                .output()
                .map_err(|error| format!("failed to stop WireGuard tunnel '{profile}': {error}"))?;
            if !output.status.success() && service_is_running(profile) {
                return Err(format!(
                    "WireGuard tunnel '{profile}' failed to stop: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
        }
        Ok(serde_json::json!({
            "status": "disconnected",
            "profiles": active,
        }))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err("bundled WireGuard tunnels are currently supported on Windows only".to_string())
    }
}

#[tauri::command]
pub async fn vpn_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let engine = wireguard_executable(&app).ok();
    let profiles = vpn_profile_store::list_profiles(&app, None).unwrap_or_default();
    let profile_values: Vec<serde_json::Value> = profiles
        .iter()
        .map(|profile| {
            serde_json::json!({
                "name": profile.name,
                "path": profile.path,
                "active": service_is_running(&profile.name),
            })
        })
        .collect();
    let active_profile = profiles
        .iter()
        .find(|profile| service_is_running(&profile.name))
        .map(|profile| profile.name.clone());
    Ok(serde_json::json!({
        "installed": engine.is_some(),
        "engineInstalled": engine.is_some(),
        "connected": active_profile.is_some(),
        "activeProfile": active_profile,
        "profiles": profile_values,
        "details": if engine.is_some() {
                "Installed WireGuard engine ready"
        } else {
                "WireGuard is not installed"
        },
    }))
}

#[tauri::command]
pub async fn run_antivirus_scan() -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    {
        let output = hidden_command("powershell")
            .args(["-NoProfile", "-Command", "Start-MpScan -ScanType QuickScan"])
            .output()
            .map_err(|error| format!("failed to start Windows Defender scan: {error}"))?;
        Ok(serde_json::json!({
            "status": if output.status.success() { "scan_started" } else { "failed" },
            "type": "quick",
            "output": String::from_utf8_lossy(&output.stdout).trim(),
            "error": String::from_utf8_lossy(&output.stderr).trim(),
        }))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(serde_json::json!({
            "status": "unsupported",
            "message": "Windows Defender scan is only available on Windows",
        }))
    }
}

#[tauri::command]
pub async fn update_av_signatures() -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    {
        let output = hidden_command("powershell")
            .args(["-NoProfile", "-Command", "Update-MpSignature"])
            .output()
            .map_err(|error| format!("failed to update Windows Defender signatures: {error}"))?;
        Ok(serde_json::json!({
            "status": if output.status.success() { "updated" } else { "failed" },
            "output": String::from_utf8_lossy(&output.stdout).trim(),
            "error": String::from_utf8_lossy(&output.stderr).trim(),
        }))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(serde_json::json!({
            "status": "unsupported",
            "message": "Windows Defender is only available on Windows",
        }))
    }
}

#[tauri::command]
pub async fn install_security_tools(app: AppHandle) -> Result<serde_json::Value, String> {
    let executable = wireguard_executable(&app)?;
    Ok(serde_json::json!({
        "status": "installed",
        "message": "WireGuard is available from the system installation.",
        "engine": executable.to_string_lossy(),
    }))
}
