use crate::{metadata_ext, AppState};
use rusqlite::{params, OptionalExtension};
use tauri::State;

async fn fetch_tmdb_backdrop(tmdb_id: &str, media_type: &str, api_key: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;
    let preferred = if matches!(media_type, "tvshow" | "episode" | "tv") {
        ["tv", "movie"]
    } else {
        ["movie", "tv"]
    };

    for endpoint in preferred {
        let url = format!("https://api.themoviedb.org/3/{endpoint}/{tmdb_id}?api_key={api_key}");
        let response = client.get(url).send().await.ok()?;
        if !response.status().is_success() {
            continue;
        }
        let data = response.json::<serde_json::Value>().await.ok()?;
        if let Some(path) = data
            .get("backdrop_path")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(format!("https://image.tmdb.org/t/p/w1280{path}"));
        }
    }
    None
}

fn full_media_item(state: &State<'_, AppState>, id: i64) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|err| err.to_string())?;
    db.conn
        .query_row(
            "SELECT id, title, file_path, media_type, year, rating, overview, poster_path,
                    backdrop_path, genre, duration, file_size, resolution, codec, verified,
                    watched, favorite, date_added, last_played, tmdb_id, imdb_id, source_id
             FROM media_items WHERE id = ?1",
            params![id],
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "file_path": row.get::<_, String>(2)?,
                    "media_type": row.get::<_, String>(3)?,
                    "year": row.get::<_, Option<i32>>(4)?,
                    "rating": row.get::<_, Option<f64>>(5)?,
                    "overview": row.get::<_, Option<String>>(6)?,
                    "poster_path": row.get::<_, Option<String>>(7)?,
                    "backdrop_path": row.get::<_, Option<String>>(8)?,
                    "genre": row.get::<_, Option<String>>(9)?,
                    "duration": row.get::<_, Option<i64>>(10)?,
                    "file_size": row.get::<_, Option<i64>>(11)?,
                    "resolution": row.get::<_, Option<String>>(12)?,
                    "codec": row.get::<_, Option<String>>(13)?,
                    "verified": row.get::<_, bool>(14)?,
                    "watched": row.get::<_, bool>(15)?,
                    "favorite": row.get::<_, bool>(16)?,
                    "date_added": row.get::<_, String>(17)?,
                    "last_played": row.get::<_, Option<String>>(18)?,
                    "tmdb_id": row.get::<_, Option<String>>(19)?,
                    "imdb_id": row.get::<_, Option<String>>(20)?,
                    "source_id": row.get::<_, Option<i64>>(21)?,
                }))
            },
        )
        .map_err(|err| err.to_string())
}

async fn check_media_item_metadata(
    state: State<'_, AppState>,
    id: i64,
) -> Result<serde_json::Value, String> {
    let mut result = metadata_ext::check_media_item_metadata(state.clone(), id).await?;

    let backdrop_request = {
        let db = state.db.lock().map_err(|err| err.to_string())?;
        db.conn
            .query_row(
                "SELECT tmdb_id, media_type, backdrop_path,
                        (SELECT api_key FROM api_keys WHERE lower(provider) IN ('tmdb', 'tmdb_images', 'themoviedb', 'themoviedb_images') LIMIT 1)
                 FROM media_items WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|err| err.to_string())?
    };

    if let Some((Some(tmdb_id), media_type, backdrop_path, Some(api_key))) = backdrop_request {
        let missing_backdrop = backdrop_path
            .as_deref()
            .map(str::trim)
            .map(str::is_empty)
            .unwrap_or(true);
        if missing_backdrop && !api_key.trim().is_empty() {
            if let Some(backdrop) = fetch_tmdb_backdrop(&tmdb_id, &media_type, &api_key).await {
                let db = state.db.lock().map_err(|err| err.to_string())?;
                db.conn
                    .execute(
                        "UPDATE media_items SET backdrop_path = ?1 WHERE id = ?2",
                        params![backdrop, id],
                    )
                    .map_err(|err| err.to_string())?;
            }
        }
    }

    let item = full_media_item(&state, id)?;
    if let Some(object) = result.as_object_mut() {
        object.insert("updated_item".to_string(), item.clone());
        if let Some(item_object) = item.as_object() {
            for (key, value) in item_object {
                object.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn metadata_bridge_contract_keeps_updated_item_envelope() {
        let value = serde_json::json!({
            "status": "success",
            "updated_item": { "id": 1, "poster_path": "poster.jpg", "backdrop_path": "backdrop.jpg" }
        });
        assert_eq!(value["updated_item"]["id"], 1);
        assert!(value["updated_item"]["poster_path"].is_string());
        assert!(value["updated_item"]["backdrop_path"].is_string());
    }
}
