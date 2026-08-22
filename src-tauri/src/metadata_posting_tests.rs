#[cfg(test)]
mod tests {
    use crate::db::{Database, MediaItem};

    fn blank_test_media_item(file_path: &str) -> MediaItem {
        MediaItem {
            id: None,
            title: "raw.file.name".to_string(),
            file_path: file_path.to_string(),
            media_type: "movie".to_string(),
            year: None,
            rating: None,
            overview: None,
            poster_path: None,
            backdrop_path: None,
            genre: None,
            duration: None,
            file_size: Some(1024),
            resolution: None,
            codec: None,
            verified: false,
            watched: false,
            favorite: false,
            date_added: "2026-06-25T00:00:00Z".to_string(),
            last_played: None,
            tmdb_id: None,
            imdb_id: None,
            source_id: None,
        }
    }

    #[test]
    fn metadata_and_poster_are_posted_to_media_file_record() {
        let db = Database::new(":memory:").expect("in-memory database should initialize");
        let file_path = r"C:\CinaVaultTest\Inception.2010.mkv";
        db.add_media_item_data(&blank_test_media_item(file_path))
            .expect("test media item should insert");

        db.update_media_metadata_data(
            file_path,
            Some("Inception"),
            Some("A thief enters dreams to extract and implant secrets."),
            Some("https://image.tmdb.org/t/p/w500/inception-test-poster.jpg"),
            Some(2010),
            Some(8.8),
            Some("Science Fiction, Thriller"),
            Some("27205"),
            Some("tt1375666"),
            None,
        )
        .expect("metadata update should post to the media row");

        let items = db
            .get_media_items_data(None, None, None)
            .expect("media items should load");
        let item = items
            .into_iter()
            .find(|item| item.file_path == file_path)
            .expect("updated media item should exist");

        assert_eq!(item.title, "Inception");
        assert_eq!(
            item.overview.as_deref(),
            Some("A thief enters dreams to extract and implant secrets.")
        );
        assert_eq!(
            item.poster_path.as_deref(),
            Some("https://image.tmdb.org/t/p/w500/inception-test-poster.jpg")
        );
        assert_eq!(item.year, Some(2010));
        assert_eq!(item.rating, Some(8.8));
        assert_eq!(item.genre.as_deref(), Some("Science Fiction, Thriller"));
        assert_eq!(item.tmdb_id.as_deref(), Some("27205"));
        assert_eq!(item.imdb_id.as_deref(), Some("tt1375666"));
    }
}
