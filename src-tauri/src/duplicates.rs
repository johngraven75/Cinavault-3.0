// CinaVault Premium — Duplicate Finder Module
use crate::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DuplicateGroup {
    pub id: i64,
    pub group_hash: String,
    pub items: Vec<DuplicateItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DuplicateItem {
    pub id: i64,
    pub media_id: i64,
    pub file_path: String,
    pub file_size: Option<i64>,
    pub title: Option<String>,
}

#[tauri::command]
pub async fn find_duplicates(
    state: State<'_, AppState>,
    match_by: Option<String>,
    tolerance_mb: Option<f64>,
) -> Result<serde_json::Value, String> {
    let match_rule = match_by.unwrap_or_else(|| "name_size".to_string());
    let tolerance = tolerance_mb.unwrap_or(0.0) * 1_048_576.0;

    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn
        .execute("DELETE FROM duplicate_items", [])
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM duplicate_groups", [])
        .map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn
        .prepare("SELECT id, title, file_path, file_size FROM media_items ORDER BY title")
        .map_err(|e| e.to_string())?;

    let items: Vec<(i64, String, String, Option<i64>)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut groups: HashMap<String, Vec<(i64, String, Option<i64>)>> = HashMap::new();

    for (id, title, path, size) in &items {
        let key = match match_rule.as_str() {
            "name" => title.to_lowercase().trim().to_string(),
            "size" => {
                if let Some(s) = size {
                    format!("size_{}", s)
                } else {
                    continue;
                }
            }
            "hash" => match partial_hash(path) {
                Ok(h) => h,
                Err(_) => continue,
            },
            _ => {
                let name_key = title.to_lowercase().trim().to_string();
                let size_key = size.unwrap_or(0);
                format!("{}_{}", name_key, size_key)
            }
        };

        groups
            .entry(key)
            .or_default()
            .push((*id, path.clone(), *size));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let mut total_groups = 0u64;
    let mut total_items = 0u64;

    for (key, group_items) in &groups {
        if group_items.len() < 2 {
            continue;
        }

        if (match_rule == "name_size" || match_rule == "name") && tolerance > 0.0 {
            let sizes: Vec<i64> = group_items.iter().filter_map(|(_, _, s)| *s).collect();
            if sizes.len() >= 2 {
                let max = *sizes.iter().max().unwrap_or(&0);
                let min = *sizes.iter().min().unwrap_or(&0);
                if (max - min) as f64 > tolerance {
                    continue;
                }
            }
        }

        let hash = format!("{:x}", Sha256::digest(key.as_bytes()));
        db.conn
            .execute(
                "INSERT INTO duplicate_groups (group_hash, created_at) VALUES (?1, ?2)",
                params![hash, now],
            )
            .map_err(|e| e.to_string())?;
        let group_id = db.conn.last_insert_rowid();
        total_groups += 1;

        for (media_id, path, size) in group_items {
            db.conn
                .execute(
                    "INSERT INTO duplicate_items (group_id, media_id, file_path, file_size) VALUES (?1, ?2, ?3, ?4)",
                    params![group_id, media_id, path, size],
                )
                .map_err(|e| e.to_string())?;
            total_items += 1;
        }
    }

    Ok(serde_json::json!({
        "groups_found": total_groups,
        "total_duplicates": total_items,
        "match_rule": match_rule,
    }))
}

#[tauri::command]
pub fn get_duplicate_groups(state: State<AppState>) -> Result<Vec<DuplicateGroup>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut group_stmt = db
        .conn
        .prepare("SELECT id, group_hash FROM duplicate_groups ORDER BY id")
        .map_err(|e| e.to_string())?;

    let groups: Vec<(i64, String)> = group_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut result = Vec::new();

    for (gid, hash) in groups {
        let mut item_stmt = db
            .conn
            .prepare(
                "SELECT di.id, di.media_id, di.file_path, di.file_size, mi.title \
             FROM duplicate_items di LEFT JOIN media_items mi ON di.media_id = mi.id \
             WHERE di.group_id = ?1",
            )
            .map_err(|e| e.to_string())?;

        let items: Vec<DuplicateItem> = item_stmt
            .query_map(params![gid], |row| {
                Ok(DuplicateItem {
                    id: row.get(0)?,
                    media_id: row.get(1)?,
                    file_path: row.get(2)?,
                    file_size: row.get(3)?,
                    title: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        result.push(DuplicateGroup {
            id: gid,
            group_hash: hash,
            items,
        });
    }

    Ok(result)
}

#[tauri::command]
pub fn remove_duplicate(
    state: State<AppState>,
    item_id: i64,
    delete_file: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    if delete_file {
        let mut stmt = db
            .conn
            .prepare("SELECT file_path FROM duplicate_items WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        if let Ok(path) = stmt.query_row(params![item_id], |row| row.get::<_, String>(0)) {
            let _ = std::fs::remove_file(&path);
        }
    }

    let mut stmt = db
        .conn
        .prepare("SELECT media_id FROM duplicate_items WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    if let Ok(media_id) = stmt.query_row(params![item_id], |row| row.get::<_, i64>(0)) {
        db.conn
            .execute("DELETE FROM media_items WHERE id = ?1", params![media_id])
            .map_err(|e| e.to_string())?;
    }
    db.conn
        .execute(
            "DELETE FROM duplicate_items WHERE id = ?1",
            params![item_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn quarantine(state: State<AppState>, item_id: i64) -> Result<serde_json::Value, String> {
    let source_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.conn
            .query_row(
                "SELECT file_path FROM duplicate_items WHERE id = ?1",
                params![item_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(|e| format!("Duplicate item {item_id} was not found: {e}"))?
    };

    let source = PathBuf::from(&source_path);
    if !source.is_file() {
        return Err(format!(
            "Duplicate file does not exist: {}",
            source.display()
        ));
    }

    let quarantine_dir = state.app_data_dir.join("quarantine");
    std::fs::create_dir_all(&quarantine_dir)
        .map_err(|e| format!("Unable to create quarantine directory: {e}"))?;

    let destination = unique_quarantine_path(&quarantine_dir, &source)?;
    move_without_overwrite(&source, &destination)?;

    let destination_string = destination.to_string_lossy().into_owned();
    let update_result = (|| -> Result<(), String> {
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        let transaction = db.conn.transaction().map_err(|e| e.to_string())?;
        transaction
            .execute(
                "UPDATE duplicate_items SET file_path = ?1 WHERE id = ?2",
                params![destination_string, item_id],
            )
            .map_err(|e| e.to_string())?;
        transaction
            .execute(
                "UPDATE media_items SET file_path = ?1
                 WHERE id = (SELECT media_id FROM duplicate_items WHERE id = ?2)",
                params![destination_string, item_id],
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())
    })();

    if let Err(error) = update_result {
        let rollback_error = std::fs::rename(&destination, &source).err();
        return Err(match rollback_error {
            Some(rollback) => format!(
                "Unable to update quarantine records: {error}; file rollback also failed: {rollback}"
            ),
            None => format!("Unable to update quarantine records: {error}"),
        });
    }

    Ok(serde_json::json!({
        "item_id": item_id,
        "original_path": source_path,
        "quarantine_path": destination_string,
    }))
}

fn unique_quarantine_path(directory: &Path, source: &Path) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .ok_or_else(|| format!("Source path has no file name: {}", source.display()))?;
    let first_candidate = directory.join(file_name);
    if !first_candidate.exists() {
        return Ok(first_candidate);
    }

    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("duplicate");
    let extension = source.extension().and_then(|value| value.to_str());

    for suffix in 1..=10_000u32 {
        let candidate_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{stem} ({suffix}).{ext}"),
            _ => format!("{stem} ({suffix})"),
        };
        let candidate = directory.join(candidate_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("Unable to allocate a unique quarantine file name".to_string())
}

fn move_without_overwrite(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Err(format!(
            "Refusing to overwrite quarantine file: {}",
            destination.display()
        ));
    }

    match std::fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            std::fs::copy(source, destination).map_err(|copy_error| {
                format!(
                    "Unable to move file into quarantine ({rename_error}); copy fallback failed: {copy_error}"
                )
            })?;
            if let Err(remove_error) = std::fs::remove_file(source) {
                let _ = std::fs::remove_file(destination);
                return Err(format!(
                    "Quarantine copy succeeded but source removal failed: {remove_error}"
                ));
            }
            Ok(())
        }
    }
}

fn partial_hash(path: &str) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 1_048_576];
    let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
    buffer.truncate(bytes_read);
    let hash = Sha256::digest(&buffer);
    Ok(format!("{:x}", hash))
}
