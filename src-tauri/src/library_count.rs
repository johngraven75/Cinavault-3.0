use crate::AppState;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryCount {
    pub total: i64,
    pub media_type: Option<String>,
    pub capped: bool,
}

fn normalized_media_type(media_type: Option<String>) -> Option<String> {
    media_type
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty() && value != "all")
}

pub fn query_library_count(
    database: &crate::db::Database,
    media_type: Option<String>,
) -> Result<LibraryCount, String> {
    let media_type = normalized_media_type(media_type);
    let total = match media_type.as_deref() {
        Some(kind) => database
            .conn
            .query_row(
                "SELECT COUNT(*) FROM media_items WHERE media_type <> 'photo' AND lower(media_type) = ?1",
                params![kind],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| error.to_string())?,
        None => database
            .conn
            .query_row(
                "SELECT COUNT(*) FROM media_items WHERE media_type <> 'photo'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| error.to_string())?,
    };

    Ok(LibraryCount {
        total,
        media_type,
        capped: false,
    })
}

#[tauri::command]
pub fn get_library_count(
    state: State<'_, AppState>,
    media_type: Option<String>,
) -> Result<LibraryCount, String> {
    let database = state.db.lock().map_err(|error| error.to_string())?;
    query_library_count(&database, media_type)
}

#[cfg(test)]
mod tests {
    use super::query_library_count;
    use crate::db::{Database, MediaItem};

    fn item(title: &str, media_type: &str) -> MediaItem {
        MediaItem {
            id: None,
            title: title.to_string(),
            file_path: format!("C:/library/{title}.mkv"),
            media_type: media_type.to_string(),
            year: None,
            rating: None,
            overview: None,
            poster_path: None,
            backdrop_path: None,
            genre: None,
            duration: None,
            file_size: None,
            resolution: None,
            codec: None,
            verified: false,
            watched: false,
            favorite: false,
            date_added: "2026-07-26T00:00:00Z".to_string(),
            last_played: None,
            tmdb_id: None,
            imdb_id: None,
            source_id: None,
        }
    }

    #[test]
    fn count_is_uncapped_and_excludes_photo_artifacts() {
        let database = Database::new(":memory:").expect("database should initialize");
        for index in 0..601 {
            database
                .add_media_item_data(&item(&format!("Movie{index}"), "movie"))
                .expect("movie should insert");
        }
        database
            .add_media_item_data(&item("PosterArtifact", "photo"))
            .expect("photo fixture should insert");

        let all = query_library_count(&database, None).expect("count should succeed");
        assert_eq!(all.total, 601);
        assert!(!all.capped);

        let movies = query_library_count(&database, Some("movie".to_string()))
            .expect("typed count should succeed");
        assert_eq!(movies.total, 601);
        assert_eq!(movies.media_type.as_deref(), Some("movie"));
    }
}
