# Library Enrichment And Filename Normalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a reusable video-library enrichment workflow that writes back stronger metadata, classifies adult-designated sources correctly, and safely renames low-quality timestamp files to normalized `Title.ext` names only when confidence is high enough.

**Architecture:** The implementation adds a dedicated Rust enrichment module that owns candidate selection, provider routing, metadata merge, confidence scoring, and optional rename execution. The AI tab becomes a thin trigger surface over two backend commands: metadata-only enrichment and enrichment plus filename normalization.

**Tech Stack:** Rust, Tauri v2, rusqlite, reqwest, React, TypeScript, Zustand, cargo test, cargo check, Vite

---

## File Map

- Create: `src-tauri/src/enrichment.rs`
  - Owns enrichment models, candidate selection, title cleanup, confidence scoring, rename safety, provider merge logic, and Tauri commands.
- Modify: `src-tauri/src/main.rs`
  - Registers new enrichment commands and module.
- Modify: `src-tauri/src/ai.rs`
  - Refactors current adult gather path to call shared enrichment helpers and keeps adult-only local asset generation as a sub-flow instead of a separate selection engine.
- Modify: `src-tauri/src/db.rs`
  - Adds DB helpers for metadata writeback and rename-safe `file_path` updates while preserving user-state fields.
- Modify: `src/components/tabs/AIDiagnosticsTab.tsx`
  - Adds `Enrich Library Metadata` and `Enrich + Normalize Filenames` actions and clearer result summaries.
- Modify: `src/store/appStore.ts`
  - Adds any small result-shape typing or provider ID cleanup needed by the UI.

## Task 1: Add Enrichment Core Helpers

**Files:**
- Create: `src-tauri/src/enrichment.rs`
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/enrichment.rs`

- [ ] **Step 1: Write the failing tests for title cleanup, adult-source detection, and rename confidence**

```rust
#[cfg(test)]
mod tests {
    use super::{
        build_query_candidates,
        classify_library_item,
        normalize_filename_title,
        rename_confidence,
        EnrichmentMode,
        LibraryItemRecord,
        ProviderMatch,
        SourceKind,
    };

    fn sample_item(title: &str, file_path: &str, source_name: Option<&str>) -> LibraryItemRecord {
        LibraryItemRecord {
            id: 1,
            title: title.to_string(),
            file_path: file_path.to_string(),
            media_type: "movie".to_string(),
            overview: None,
            poster_path: None,
            year: None,
            rating: None,
            genre: None,
            tmdb_id: None,
            imdb_id: None,
            source_name: source_name.map(str::to_string),
            source_path: None,
        }
    }

    #[test]
    fn normalizes_timestamp_filename_into_searchable_title() {
        assert_eq!(
            normalize_filename_title("2024-08-31_141904.mp4"),
            ""
        );
        assert_eq!(
            normalize_filename_title("My.Movie_1080p.x264.mkv"),
            "My Movie"
        );
    }

    #[test]
    fn classifies_adult_sources_from_source_name_hints() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Personal Vids X\Media\2024-08-31_141904.mp4",
            Some("Personal Vids X"),
        );
        assert_eq!(classify_library_item(&item), SourceKind::AdultVideo);
    }

    #[test]
    fn builds_query_candidates_from_embedded_title_before_filename() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Videos\2024-08-31_141904.mp4",
            Some("General Video"),
        );
        let queries = build_query_candidates(&item, Some("Actual Scene Title".to_string()));
        assert_eq!(queries.first().map(String::as_str), Some("Actual Scene Title"));
    }

    #[test]
    fn allows_rename_for_balanced_confidence_when_provider_and_embedded_title_agree() {
        let item = sample_item(
            "2024-08-31 141904",
            r"E:\Videos\2024-08-31_141904.mp4",
            Some("General Video"),
        );
        let provider = ProviderMatch {
            title: Some("Actual Scene Title".to_string()),
            overview: None,
            poster_path: None,
            year: None,
            rating: None,
            genre: None,
            tmdb_id: Some("123".to_string()),
            imdb_id: None,
        };
        let confidence = rename_confidence(
            &item,
            Some("Actual Scene Title"),
            &provider,
            EnrichmentMode::MetadataAndRename,
        );
        assert!(confidence.allow_rename);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test enrichment::tests -- --nocapture`

Expected: FAIL with unresolved module or missing helper symbols in `src-tauri/src/enrichment.rs`

- [ ] **Step 3: Write minimal enrichment models and pure helper implementation**

```rust
// src-tauri/src/enrichment.rs
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    StandardVideo,
    AdultVideo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnrichmentMode {
    MetadataOnly,
    MetadataAndRename,
}

#[derive(Debug, Clone)]
pub struct LibraryItemRecord {
    pub id: i64,
    pub title: String,
    pub file_path: String,
    pub media_type: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub genre: Option<String>,
    pub tmdb_id: Option<String>,
    pub imdb_id: Option<String>,
    pub source_name: Option<String>,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderMatch {
    pub title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub genre: Option<String>,
    pub tmdb_id: Option<String>,
    pub imdb_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDecision {
    pub allow_rename: bool,
    pub normalized_title: Option<String>,
}

fn has_adult_hint(text: &str) -> bool {
    let lower = text
        .replace(['\\', '/', '_', '-'], " ")
        .to_lowercase();
    ["adult", "porn", "xxx", "nsfw", "personal x", "x library", "vids x", "videos x"]
        .iter()
        .any(|hint| lower.contains(hint))
}

pub fn normalize_filename_title(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    if stem.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | '_' | ' ')) {
        return String::new();
    }

    stem.replace(['.', '_'], " ")
        .replace("1080p", "")
        .replace("720p", "")
        .replace("x264", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn classify_library_item(item: &LibraryItemRecord) -> SourceKind {
    if item.media_type == "adult"
        || has_adult_hint(&item.title)
        || has_adult_hint(&item.file_path)
        || item.source_name.as_deref().map(has_adult_hint).unwrap_or(false)
        || item.source_path.as_deref().map(has_adult_hint).unwrap_or(false)
    {
        SourceKind::AdultVideo
    } else {
        SourceKind::StandardVideo
    }
}

pub fn build_query_candidates(item: &LibraryItemRecord, embedded_title: Option<String>) -> Vec<String> {
    let mut queries = Vec::new();
    if let Some(title) = embedded_title.filter(|t| !t.trim().is_empty()) {
        queries.push(title);
    }
    if !item.title.trim().is_empty() {
        queries.push(item.title.trim().to_string());
    }
    let normalized = normalize_filename_title(&item.file_path);
    if !normalized.is_empty() && !queries.iter().any(|q| q.eq_ignore_ascii_case(&normalized)) {
        queries.push(normalized);
    }
    queries
}

pub fn rename_confidence(
    item: &LibraryItemRecord,
    embedded_title: Option<&str>,
    provider: &ProviderMatch,
    mode: EnrichmentMode,
) -> RenameDecision {
    if mode != EnrichmentMode::MetadataAndRename {
        return RenameDecision { allow_rename: false, normalized_title: None };
    }

    let provider_title = provider.title.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let embedded = embedded_title.map(str::trim).filter(|t| !t.is_empty());
    let stable_id = provider.tmdb_id.is_some() || provider.imdb_id.is_some();

    let allow_rename = match (provider_title, embedded) {
        (Some(p), Some(e)) if p.eq_ignore_ascii_case(e) => true,
        (Some(_), Some(_)) => false,
        (Some(_), None) if stable_id && normalize_filename_title(&item.file_path).is_empty() => true,
        _ => false,
    };

    RenameDecision {
        allow_rename,
        normalized_title: provider_title.map(str::to_string),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test enrichment::tests -- --nocapture`

Expected: PASS for the new helper-level tests

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/enrichment.rs src-tauri/src/main.rs
git commit -m "feat: add enrichment core helpers"
```

## Task 2: Add Database Writeback And Rename-Safe Persistence Helpers

**Files:**
- Modify: `src-tauri/src/db.rs`
- Test: `src-tauri/src/db.rs`

- [ ] **Step 1: Write the failing DB tests for metadata-only updates and rename-safe path updates**

```rust
#[test]
fn enrichment_update_preserves_user_flags() {
    let db_path = test_db_path("enrichment-update");
    let db = Database::new(&db_path).expect("db should open");

    let item = sample_item("Old Title", r"C:\media\old-title.mp4");
    db.add_media_item_data(&item).expect("insert should succeed");
    db.conn.execute(
        "UPDATE media_items SET watched = 1, favorite = 1 WHERE file_path = ?1",
        params![&item.file_path],
    ).expect("flag update should succeed");

    db.update_media_metadata_data(
        &item.file_path,
        Some("Better Title"),
        Some("Overview text"),
        Some("https://poster"),
        Some(2024),
        Some(8.1),
        Some("Drama"),
        Some("123"),
        Some("tt123"),
        Some("adult"),
    ).expect("metadata update should succeed");

    let row = db.conn.query_row(
        "SELECT title, overview, watched, favorite, media_type FROM media_items WHERE file_path = ?1",
        params![&item.file_path],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, bool>(2)?,
            row.get::<_, bool>(3)?,
            row.get::<_, String>(4)?,
        )),
    ).expect("row should exist");

    assert_eq!(row.0, "Better Title");
    assert_eq!(row.1.as_deref(), Some("Overview text"));
    assert!(row.2);
    assert!(row.3);
    assert_eq!(row.4, "adult");
}

#[test]
fn rename_update_changes_file_path_only_after_success() {
    let db_path = test_db_path("rename-update");
    let db = Database::new(&db_path).expect("db should open");

    let item = sample_item("Old Title", r"C:\media\old-title.mp4");
    db.add_media_item_data(&item).expect("insert should succeed");

    db.update_media_file_path_data(&item.file_path, r"C:\media\New Title.mp4", "New Title")
        .expect("rename update should succeed");

    let row = db.conn.query_row(
        "SELECT title, file_path FROM media_items WHERE file_path = ?1",
        params![r"C:\media\New Title.mp4"],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ).expect("renamed row should exist");

    assert_eq!(row.0, "New Title");
    assert_eq!(row.1, r"C:\media\New Title.mp4");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test db::tests::enrichment_update_preserves_user_flags db::tests::rename_update_changes_file_path_only_after_success -- --nocapture`

Expected: FAIL with missing database helper methods

- [ ] **Step 3: Implement focused DB helper methods**

```rust
pub fn update_media_metadata_data(
    &self,
    file_path: &str,
    title: Option<&str>,
    overview: Option<&str>,
    poster_path: Option<&str>,
    year: Option<i32>,
    rating: Option<f64>,
    genre: Option<&str>,
    tmdb_id: Option<&str>,
    imdb_id: Option<&str>,
    media_type: Option<&str>,
) -> SqlResult<()> {
    self.conn.execute(
        "UPDATE media_items
         SET title = COALESCE(?1, title),
             overview = COALESCE(?2, overview),
             poster_path = COALESCE(?3, poster_path),
             year = COALESCE(?4, year),
             rating = COALESCE(?5, rating),
             genre = COALESCE(?6, genre),
             tmdb_id = COALESCE(?7, tmdb_id),
             imdb_id = COALESCE(?8, imdb_id),
             media_type = COALESCE(?9, media_type)
         WHERE file_path = ?10",
        params![
            title,
            overview,
            poster_path,
            year,
            rating,
            genre,
            tmdb_id,
            imdb_id,
            media_type,
            file_path,
        ],
    )?;
    Ok(())
}

pub fn update_media_file_path_data(
    &self,
    old_file_path: &str,
    new_file_path: &str,
    new_title: &str,
) -> SqlResult<()> {
    self.conn.execute(
        "UPDATE media_items SET file_path = ?1, title = ?2 WHERE file_path = ?3",
        params![new_file_path, new_title, old_file_path],
    )?;
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test db::tests::enrichment_update_preserves_user_flags db::tests::rename_update_changes_file_path_only_after_success -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat: add enrichment database update helpers"
```

## Task 3: Add Provider Routing, Metadata Merge, And Safe Rename Execution

**Files:**
- Modify: `src-tauri/src/enrichment.rs`
- Modify: `src-tauri/src/ai.rs`
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/enrichment.rs`

- [ ] **Step 1: Write the failing tests for metadata-only fallback and rename collision blocking**

```rust
#[test]
fn metadata_only_mode_updates_fields_without_allowing_rename() {
    let item = sample_item(
        "2024-08-31 141904",
        r"E:\Videos\2024-08-31_141904.mp4",
        Some("General Video"),
    );
    let provider = ProviderMatch {
        title: Some("Actual Scene Title".to_string()),
        overview: Some("Summary".to_string()),
        poster_path: None,
        year: Some(2024),
        rating: None,
        genre: None,
        tmdb_id: Some("123".to_string()),
        imdb_id: None,
    };
    let decision = rename_confidence(
        &item,
        Some("Actual Scene Title"),
        &provider,
        EnrichmentMode::MetadataOnly,
    );
    assert!(!decision.allow_rename);
}

#[test]
fn blocks_rename_when_target_file_already_exists() {
    let source = std::env::temp_dir().join("cinavault-source.mp4");
    let target = std::env::temp_dir().join("Actual Title.mp4");
    std::fs::write(&source, b"source").expect("source write should succeed");
    std::fs::write(&target, b"target").expect("target write should succeed");

    let result = safe_rename_target(&source, "Actual Title");
    assert!(matches!(result, RenameTarget::Collision(_)));

    let _ = std::fs::remove_file(source);
    let _ = std::fs::remove_file(target);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test enrichment::tests::metadata_only_mode_updates_fields_without_allowing_rename enrichment::tests::blocks_rename_when_target_file_already_exists -- --nocapture`

Expected: FAIL with missing rename-target helpers or collision logic

- [ ] **Step 3: Implement provider merge, command entry points, and safe rename helpers**

```rust
#[derive(Debug, Clone)]
pub struct EnrichmentRunSummary {
    pub items_scanned: usize,
    pub metadata_items_enriched: usize,
    pub metadata_fields_updated: usize,
    pub titles_improved: usize,
    pub items_reclassified_as_adult: usize,
    pub files_renamed: usize,
    pub rename_collisions_skipped: usize,
    pub low_confidence_metadata_only: usize,
    pub skipped_missing_files: usize,
    pub skipped_non_video_items: usize,
    pub provider_errors: Vec<String>,
}

pub enum RenameTarget {
    Ready(std::path::PathBuf, String),
    Collision(std::path::PathBuf),
    Invalid(String),
}

pub fn safe_rename_target(source: &Path, normalized_title: &str) -> RenameTarget {
    let parent = match source.parent() {
        Some(parent) => parent,
        None => return RenameTarget::Invalid("source file has no parent".to_string()),
    };
    let extension = source
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    let cleaned = sanitize_windows_filename(normalized_title);
    if cleaned.is_empty() {
        return RenameTarget::Invalid("normalized title is empty".to_string());
    }

    let candidate = parent.join(format!("{cleaned}{extension}"));
    if candidate.exists() && candidate != source {
        return RenameTarget::Collision(candidate);
    }

    RenameTarget::Ready(candidate, cleaned)
}

#[tauri::command]
pub async fn run_library_enrichment(
    state: State<'_, AppState>,
    rename_files: bool,
) -> Result<serde_json::Value, String> {
    let mode = if rename_files {
        EnrichmentMode::MetadataAndRename
    } else {
        EnrichmentMode::MetadataOnly
    };
    run_enrichment_pipeline(state, mode).await
}
```

- [ ] **Step 4: Run tests and compile checks**

Run: `cargo test enrichment::tests -- --nocapture`

Expected: PASS

Run: `cargo check`

Expected: PASS with warnings only

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/enrichment.rs src-tauri/src/ai.rs src-tauri/src/main.rs
git commit -m "feat: add enrichment pipeline and safe rename flow"
```

## Task 4: Add AI Tab Actions And Result Rendering

**Files:**
- Modify: `src/components/tabs/AIDiagnosticsTab.tsx`
- Modify: `src/store/appStore.ts`
- Test: `npm run build`

- [ ] **Step 1: Write the failing UI-facing change by adding the new quick actions before the backend command is registered**

```tsx
const quickActions = [
  { label: "Network Diagnostics", icon: Network, q: "Run network diagnostics" },
  { label: "Check Sources", icon: FolderSearch, q: "Check all media sources" },
  { label: "Check Providers", icon: Database, q: "Check metadata providers" },
  {
    label: "Enrich Library Metadata",
    icon: Sparkles,
    runNow: () => invoke("run_library_enrichment", { renameFiles: false }),
  },
  {
    label: "Enrich + Normalize Filenames",
    icon: Sparkles,
    runNow: () => invoke("run_library_enrichment", { renameFiles: true }),
  },
];
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run build`

Expected: FAIL until the backend command name and result formatting code both exist cleanly in the UI flow

- [ ] **Step 3: Add the new AI tab actions and richer result output**

```tsx
const quickActions = [
  { label: "Network Diagnostics", icon: Network, q: "Run network diagnostics" },
  { label: "Check Sources", icon: FolderSearch, q: "Check all media sources" },
  { label: "Check Providers", icon: Database, q: "Check metadata providers" },
  {
    label: "Enrich Library Metadata",
    icon: Sparkles,
    runNow: () => invoke("run_library_enrichment", { renameFiles: false }),
  },
  {
    label: "Enrich + Normalize Filenames",
    icon: Sparkles,
    runNow: () => invoke("run_library_enrichment", { renameFiles: true }),
  },
];

function formatResultSummary(result: any) {
  if (result?.type !== "library_enrichment") return JSON.stringify(result, null, 2);
  return JSON.stringify(
    {
      status: result.status,
      items_scanned: result.items_scanned,
      metadata_items_enriched: result.metadata_items_enriched,
      metadata_fields_updated: result.metadata_fields_updated,
      files_renamed: result.files_renamed,
      rename_collisions_skipped: result.rename_collisions_skipped,
      low_confidence_metadata_only: result.low_confidence_metadata_only,
      provider_errors: result.provider_errors,
    },
    null,
    2,
  );
}
```

- [ ] **Step 4: Run UI build or tests to verify it passes**

Run: `npm run build`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/tabs/AIDiagnosticsTab.tsx src/store/appStore.ts
git commit -m "feat: add library enrichment AI tab actions"
```

## Task 5: End-To-End Verification, Build, And Release Prep

**Files:**
- Modify: `build-summaries/Cinavault-Beta4-Build4-Build-Summary.md`
- Modify: `releases/CinaVault Premium_1.0.0_x64-setup-beta4-build4.exe`
- Modify: `releases/CinaVault Premium_1.0.0_x64_en-US-beta4-build4.msi`
- Test: repository verification commands

- [ ] **Step 1: Run the focused backend test suite**

Run: `cargo test enrichment::tests db::tests -- --nocapture`

Expected: PASS

- [ ] **Step 2: Run full Rust verification**

Run: `cargo test`

Expected: PASS

Run: `cargo check`

Expected: PASS with warnings only

- [ ] **Step 3: Run frontend verification**

Run: `npm run build`

Expected: PASS

- [ ] **Step 4: Build the Windows app bundles**

Run: `npm run tauri build`

Expected: PASS and generate updated NSIS and MSI bundles under `src-tauri/target/release/bundle/`

- [ ] **Step 5: Commit**

```bash
git add src-tauri src build-summaries releases
git commit -m "release: ship library enrichment and filename normalization"
```
