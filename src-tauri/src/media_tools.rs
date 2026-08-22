// CinaVault Premium — startup bootstrap and execution bridge for media tools.
//
// This module performs real executable checks and, on Windows, silently asks
// winget to install missing permanent tools. It never marks a tool ready based
// only on catalog flags.
use serde::Serialize;
use std::collections::HashSet;
use std::env;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy)]
struct MediaTool {
    id: &'static str,
    executable: &'static str,
    version_arg: &'static str,
    winget_package: &'static str,
}

const REQUIRED_MEDIA_TOOLS: &[MediaTool] = &[
    MediaTool {
        id: "ffmpeg",
        executable: "ffmpeg",
        version_arg: "-version",
        winget_package: "Gyan.FFmpeg",
    },
    MediaTool {
        id: "ffprobe",
        executable: "ffprobe",
        version_arg: "-version",
        winget_package: "Gyan.FFmpeg",
    },
    MediaTool {
        id: "yt-dlp",
        executable: "yt-dlp",
        version_arg: "--version",
        winget_package: "yt-dlp.yt-dlp",
    },
    MediaTool {
        id: "mediainfo",
        executable: "mediainfo",
        version_arg: "--Version",
        winget_package: "MediaArea.MediaInfo.CLI",
    },
    MediaTool {
        id: "mkvtoolnix",
        executable: "mkvmerge",
        version_arg: "--version",
        winget_package: "MoritzBunkus.MKVToolNix",
    },
];

#[derive(Debug, Serialize)]
struct ToolStatus {
    id: String,
    installed: bool,
    version: Option<String>,
    auto_install: bool,
    package: String,
}

fn executable_candidates(executable: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = env::var("PATH") {
        for directory in env::split_paths(&path) {
            let candidate = directory.join(executable);
            candidates.push(candidate.clone());
            #[cfg(target_os = "windows")]
            if candidate.extension().is_none() {
                candidates.push(directory.join(format!("{executable}.exe")));
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let program_files = env::var_os("ProgramFiles").map(PathBuf::from);
        let program_files_x86 = env::var_os("ProgramFiles(x86)").map(PathBuf::from);
        let local_app_data = env::var_os("LOCALAPPDATA").map(PathBuf::from);

        match executable {
            "mediainfo" => {
                for root in [program_files.clone(), program_files_x86.clone()]
                    .into_iter()
                    .flatten()
                {
                    candidates.push(root.join("MediaInfo").join("MediaInfo.exe"));
                    candidates.push(root.join("MediaInfo").join("CLI").join("MediaInfo.exe"));
                }
            }
            "mkvmerge" => {
                for root in [program_files.clone(), program_files_x86.clone()]
                    .into_iter()
                    .flatten()
                {
                    candidates.push(root.join("MKVToolNix").join("mkvmerge.exe"));
                }
            }
            "ffmpeg" | "ffprobe" => {
                for root in [program_files, program_files_x86, local_app_data]
                    .into_iter()
                    .flatten()
                {
                    candidates.push(root.join("ffmpeg").join(format!("{executable}.exe")));
                }
            }
            "yt-dlp" => {
                if let Some(root) = local_app_data {
                    candidates.push(root.join("Programs").join("yt-dlp").join("yt-dlp.exe"));
                }
            }
            _ => {}
        }
    }

    candidates
}

fn resolve_executable(executable: &str) -> PathBuf {
    let requested = Path::new(executable);
    if requested.is_absolute() && requested.is_file() {
        return requested.to_path_buf();
    }

    executable_candidates(executable)
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| PathBuf::from(executable))
}

fn command_for(executable: &str) -> Command {
    let mut command = Command::new(resolve_executable(executable));
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

fn executable_status(tool: MediaTool) -> ToolStatus {
    let output = command_for(tool.executable).arg(tool.version_arg).output();
    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version = stdout
                .lines()
                .chain(stderr.lines())
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_string());
            ToolStatus {
                id: tool.id.to_string(),
                installed: true,
                version,
                auto_install: true,
                package: tool.winget_package.to_string(),
            }
        }
        _ => ToolStatus {
            id: tool.id.to_string(),
            installed: false,
            version: None,
            auto_install: true,
            package: tool.winget_package.to_string(),
        },
    }
}

fn current_statuses() -> Vec<ToolStatus> {
    REQUIRED_MEDIA_TOOLS
        .iter()
        .copied()
        .map(executable_status)
        .collect()
}

fn validate_media_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("A media file path is required.".to_string());
    }
    let candidate = Path::new(trimmed);
    if !candidate.exists() {
        return Err(format!("Media file does not exist: {trimmed}"));
    }
    if !candidate.is_file() {
        return Err(format!("Media path is not a file: {trimmed}"));
    }
    candidate
        .canonicalize()
        .map_err(|error| format!("Unable to resolve media file path: {error}"))
}

fn run_json_tool(executable: &str, args: &[&str], path: &str) -> Result<serde_json::Value, String> {
    let media_path = validate_media_path(path)?;
    let output = command_for(executable)
        .args(args)
        .arg(&media_path)
        .output()
        .map_err(|error| format!("Failed to start {executable}: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{executable} exited with code {:?}.", output.status.code())
        } else {
            format!("{executable} failed: {stderr}")
        });
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("{executable} returned non-UTF-8 output: {error}"))?;
    serde_json::from_str(&stdout)
        .map_err(|error| format!("{executable} returned invalid JSON: {error}"))
}

#[cfg(target_os = "windows")]
fn install_winget_package(package: &str) -> serde_json::Value {
    let mut command = command_for("winget");
    command.args([
        "install",
        "--id",
        package,
        "--exact",
        "--silent",
        "--disable-interactivity",
        "--accept-package-agreements",
        "--accept-source-agreements",
    ]);
    match command.output() {
        Ok(output) => serde_json::json!({
            "package": package,
            "success": output.status.success(),
            "exit_code": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout).trim(),
            "stderr": String::from_utf8_lossy(&output.stderr).trim(),
        }),
        Err(error) => serde_json::json!({
            "package": package,
            "success": false,
            "error": error.to_string(),
        }),
    }
}

#[tauri::command]
pub fn get_media_tools_status() -> serde_json::Value {
    let tools = current_statuses();
    serde_json::json!({
        "ready": tools.iter().all(|tool| tool.installed),
        "tools": tools,
    })
}

#[tauri::command]
pub fn ensure_media_tools() -> Result<serde_json::Value, String> {
    let before = current_statuses();

    #[cfg(target_os = "windows")]
    let installations = {
        let mut attempted = HashSet::new();
        let mut results = Vec::new();
        for (tool, status) in REQUIRED_MEDIA_TOOLS.iter().zip(before.iter()) {
            if !status.installed && attempted.insert(tool.winget_package) {
                results.push(install_winget_package(tool.winget_package));
            }
        }
        results
    };

    #[cfg(not(target_os = "windows"))]
    let installations: Vec<serde_json::Value> = Vec::new();

    let after = current_statuses();
    let ready = after.iter().all(|tool| tool.installed);
    Ok(serde_json::json!({
        "type": "media_tools_startup",
        "status": if ready { "ready" } else { "missing_tools" },
        "ready": ready,
        "automatic": true,
        "authorization_prompt_required": false,
        "before": before,
        "installations": installations,
        "tools": after,
    }))
}

#[tauri::command]
pub fn inspect_with_mediainfo(path: String) -> Result<serde_json::Value, String> {
    run_json_tool("mediainfo", &["--Output=JSON"], &path)
}

#[tauri::command]
pub fn inspect_with_mkvtoolnix(path: String) -> Result<serde_json::Value, String> {
    run_json_tool(
        "mkvmerge",
        &["--identification-format", "json", "--identify"],
        &path,
    )
}

#[cfg(test)]
mod tests {
    use super::REQUIRED_MEDIA_TOOLS;
    use std::collections::HashSet;

    #[test]
    fn every_permanent_download_tool_has_an_automatic_package() {
        let ids = REQUIRED_MEDIA_TOOLS
            .iter()
            .map(|tool| tool.id)
            .collect::<HashSet<_>>();
        for required in ["ffmpeg", "ffprobe", "yt-dlp", "mediainfo", "mkvtoolnix"] {
            assert!(ids.contains(required));
        }
        assert!(REQUIRED_MEDIA_TOOLS
            .iter()
            .all(|tool| !tool.winget_package.is_empty() && !tool.executable.is_empty()));
    }
}
