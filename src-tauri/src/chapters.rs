// CinaVault Premium — Chapter Thumbnail Generation
use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn command_output(cmd: &mut Command) -> Result<std::process::Output, std::io::Error> {
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.output()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChapterThumb {
    pub timestamp: f64,
    pub path: String,
    pub label: String,
}

#[tauri::command]
pub async fn generate_chapter_thumbs(
    file_path: String,
    output_dir: Option<String>,
    interval_secs: Option<u64>,
    ffmpeg_path: Option<String>,
) -> Result<Vec<ChapterThumb>, String> {
    let ffmpeg = ffmpeg_path.unwrap_or_else(|| "ffmpeg".to_string());
    let interval = interval_secs.unwrap_or(300); // 5 minutes default

    // Get video duration first
    let ffprobe_out = command_output(Command::new("ffprobe").args(&[
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "csv=p=0",
        &file_path,
    ]))
    .map_err(|e| format!("ffprobe failed: {}", e))?;

    let duration_str = String::from_utf8_lossy(&ffprobe_out.stdout);
    let duration: f64 = duration_str.trim().parse().unwrap_or(0.0);

    if duration <= 0.0 {
        return Err("Could not determine video duration".into());
    }

    let out_dir = output_dir.unwrap_or_else(|| {
        let p = Path::new(&file_path);
        let parent = p.parent().unwrap_or(Path::new("."));
        let stem = p.file_stem().unwrap_or_default().to_string_lossy();
        parent
            .join(format!("{}_chapters", stem))
            .to_string_lossy()
            .to_string()
    });

    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let mut thumbs = Vec::new();
    let mut t = 0.0f64;
    let mut idx = 0;

    while t < duration {
        let out_path = format!("{}/chapter_{:04}.jpg", out_dir, idx);
        let timestamp = format!("{:.2}", t);

        let result = command_output(Command::new(&ffmpeg).args(&[
            "-ss", &timestamp, "-i", &file_path, "-vframes", "1", "-q:v", "3", "-y", &out_path,
        ]));

        match result {
            Ok(output) if output.status.success() => {
                let hours = (t as u64) / 3600;
                let mins = ((t as u64) % 3600) / 60;
                let secs = (t as u64) % 60;
                let label = format!("{:02}:{:02}:{:02}", hours, mins, secs);

                thumbs.push(ChapterThumb {
                    timestamp: t,
                    path: out_path,
                    label,
                });
            }
            _ => {} // skip failed frames
        }

        t += interval as f64;
        idx += 1;
    }

    Ok(thumbs)
}

#[tauri::command]
pub fn get_chapter_thumbs(chapter_dir: String) -> Result<Vec<ChapterThumb>, String> {
    let dir = Path::new(&chapter_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut thumbs = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "jpg" || ext == "png")
                .unwrap_or(false)
        })
        .collect();

    entries.sort_by_key(|e| e.file_name());

    for (i, entry) in entries.iter().enumerate() {
        let timestamp = (i as f64) * 300.0; // Assume 5-min intervals
        let hours = (timestamp as u64) / 3600;
        let mins = ((timestamp as u64) % 3600) / 60;
        let secs = (timestamp as u64) % 60;

        thumbs.push(ChapterThumb {
            timestamp,
            path: entry.path().to_string_lossy().to_string(),
            label: format!("{:02}:{:02}:{:02}", hours, mins, secs),
        });
    }

    Ok(thumbs)
}
