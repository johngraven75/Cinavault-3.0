# Library Enrichment And Filename Normalization Design

Date: 2026-05-06
Repo: CinaVault-Premium
Branch: beta-4-retry
Status: Draft for review

## Goal

Add a reusable enrichment workflow for all video libraries that:

- enriches missing metadata in the library database
- uses adult providers only for adult-designated sources
- normalizes timestamp-style or poor filenames into cleaner titles
- renames files on disk only when confidence is strong enough
- keeps metadata-only improvements when rename confidence is not strong enough

The workflow should make the library visibly better without causing unsafe file moves.

## User Decisions Captured

- Apply to all video libraries.
- Use adult providers only for adult-designated sources.
- Default normalized filename format is `Title.ext`.
- If confidence is too low, enrich metadata only and do not rename.
- Rename confidence mode is balanced:
  - rename when title matching is strong and either provider metadata or embedded tags support the match

## Current Problems

1. Metadata gathering is fragmented.
   The current AI tab flow is focused on diagnostics and adult-specific local asset gathering instead of a reusable library enrichment pipeline.

2. Real library rows can be missed.
   Adult libraries are currently inferred from partial hints instead of a first-class source-aware selection pass.

3. Metadata writeback is incomplete.
   The app can detect missing metadata, but provider-backed enrichment and library-wide writeback are limited.

4. Timestamp filenames reduce match quality.
   Files named like `2024-08-31_141904.mp4` provide weak matching signals and produce poor library presentation.

5. File rename safety is not formalized.
   There is no current pipeline for safe on-disk rename plus DB path update.

## Recommended Approach

Build a dedicated Rust enrichment pipeline and expose it through new backend commands. The AI tab can trigger the workflow, but the core logic should live outside the tab-specific handler so it can later be reused by scans, scheduled tasks, and source actions.

This is preferred over embedding more logic directly into the existing AI diagnostics handler because:

- it keeps business logic out of UI-trigger code
- it creates a reusable service for future automation
- it lets us test enrichment, confidence scoring, and rename safety independently

## Scope

### In Scope

- all video library items
- source-aware provider routing
- metadata enrichment for title, overview, poster, year, rating, genre, and external IDs
- display-title improvement
- confidence-based filename normalization
- safe on-disk rename within the same folder
- DB updates after successful rename
- AI tab actions and result reporting
- regression tests for matching, confidence, rename safety, and DB updates

### Out Of Scope

- music and photo enrichment
- moving files across directories
- creating a manual review queue
- aggressive renaming with low-confidence matches
- bulk folder restructuring
- chapter image redesign

## High-Level Architecture

Add a new backend enrichment module centered around a pipeline service.

### Proposed Backend Units

1. `enrichment` orchestration layer
   Selects candidates, routes providers, merges metadata, scores confidence, applies DB updates, and performs safe rename operations.

2. `provider matching` helpers
   Build query ladders and fetch provider metadata for standard and adult sources.

3. `title normalization` helpers
   Clean timestamp-based titles, sanitize invalid filename characters, and generate final `Title.ext` targets.

4. `rename safety` helpers
   Verify existence, keep operations inside the original folder, block collisions, and preserve extension.

5. `result reporting` layer
   Produces a structured summary for UI display and logs.

## UI Entry Points

Add new AI-tab actions:

- `Enrich Library Metadata`
- `Enrich + Normalize Filenames`

The second action runs the same enrichment pass but enables rename execution. The first action updates metadata only.

Both actions should return a structured result object that the AI Activity Log can show directly.

## Candidate Selection

The enrichment pass should select from all video items in the database.

### Standard Source Detection

Items from normal movie or general video libraries use standard providers first, such as TMDb and OMDb.

### Adult Source Detection

Items are treated as adult-library candidates when one or more of the following are true:

- `media_type` is already `adult`
- source name contains adult-designated hints
- source path contains adult-designated hints
- filename or existing title contains adult-designated hints

Adult-specific provider routing should happen only for these adult-designated candidates.

## Query Ladder

Each item should build search candidates in this order:

1. embedded title tags
2. current saved library title
3. normalized filename text
4. reduced filename text with timestamp-only patterns stripped or deprioritized

The pipeline should prefer better title evidence without throwing away the original filename context.

## Metadata Enrichment Fields

The enrichment pass can update:

- `title`
- `overview`
- `poster_path`
- `year`
- `rating`
- `genre`
- `tmdb_id`
- `imdb_id`
- `media_type` when adult-source classification is confidently determined

The pass must preserve user-state fields:

- `watched`
- `favorite`
- `verified`
- `last_played`

## Provider Routing

### Standard Video Sources

- TMDb first
- OMDb as fallback and gap filler

### Adult Sources

- adult providers first
- optional weak fallback to general providers only if needed and if the source is not ambiguous

### Merge Rules

Merge the best available fields into a single enrichment result rather than treating one provider as authoritative for all fields.

Examples:

- use one provider for title and IDs
- use another for poster or overview if the primary response is sparse

## Confidence Model

The rename decision is separate from metadata writeback.

### High Confidence

Rename allowed when:

- the provider title strongly matches normalized filename text or embedded title
- and either embedded title support exists or provider metadata returns stable identifiers

### Medium Confidence

- metadata writeback allowed
- rename skipped

### Low Confidence

- no rename
- only clearly additive metadata should be written

### Signals Used

- title similarity between provider title and local signals
- embedded-title agreement
- provider agreement across multiple sources
- presence of stable IDs
- whether the current title is obviously timestamp-style or otherwise low-quality

## Filename Normalization Rules

When rename is approved, target format is:

- `Title.ext`

### Normalization Rules

- preserve the original extension
- remove invalid Windows filename characters
- trim trailing spaces and periods
- collapse repeated whitespace
- normalize obvious punctuation noise
- block reserved device names

### Collision Rules

Recommended initial behavior:

- try exact `Title.ext`
- if the destination already exists and is not the same file, skip rename and report collision

This is safer than auto-increment naming for the first implementation.

## Filesystem Safety Rules

Before rename:

- confirm source file still exists
- confirm destination stays within the same directory
- confirm destination does not already exist as a different file

During rename:

- rename on disk first
- update DB `file_path` and `title` only after rename succeeds

On failure:

- keep DB unchanged for path/title fields tied to the rename
- return a detailed skip or error reason

## Database Behavior

The enrichment path should update metadata fields independently from rename.

### Metadata-Only Success

If enrichment succeeds but rename confidence is too low:

- save improved metadata
- leave `file_path` unchanged

### Rename Success

If rename succeeds:

- update `file_path`
- update `title`
- keep other metadata updates from the same pass

## Result Payload

Return a structured summary including:

- `items_scanned`
- `metadata_items_enriched`
- `metadata_fields_updated`
- `titles_improved`
- `items_reclassified_as_adult`
- `files_renamed`
- `rename_collisions_skipped`
- `low_confidence_metadata_only`
- `skipped_missing_files`
- `skipped_non_video_items`
- `provider_errors`

The payload should be shaped for direct display in the AI tab log.

## Error Handling

- provider failures should not abort the whole run
- file rename failures should be isolated per item
- collision skips are expected and should not count as fatal
- missing files should be counted and skipped cleanly
- DB writes should happen only after a valid item-level decision has been made

## Testing Strategy

### Rust Unit Tests

- title normalization
- timestamp-style detection
- confidence scoring
- adult-source classification
- provider ID normalization
- rename safety checks
- collision handling

### DB Tests

- metadata-only enrichment preserves watched/favorite state
- successful rename updates `file_path` and `title`
- failed rename leaves DB path unchanged

### Integration-Style Tests

- timestamp filename becomes normalized title when confidence is high
- low-confidence item receives metadata updates without rename

## Rollout Plan

### Phase 1

- backend enrichment helpers
- candidate selection
- provider routing
- metadata merge and writeback

### Phase 2

- confidence scoring
- safe rename execution
- DB path updates

### Phase 3

- AI-tab actions
- result rendering improvements

### Phase 4

- verification
- packaging
- GitHub upload

## Risks

1. Bad provider matches can create incorrect filenames.
   Mitigation: balanced confidence gate and metadata-only fallback.

2. Rename collisions may be common in dense libraries.
   Mitigation: skip and report rather than inventing alternate names automatically.

3. Timestamp-only libraries may still have weak provider matches.
   Mitigation: use embedded tags when available and save metadata even when rename is skipped.

4. Library-specific provider IDs may drift across UI and backend.
   Mitigation: normalize provider IDs in a shared path and keep source-aware routing centralized.

## Open Implementation Notes

- The existing adult gather flow should be refactored to call into the new enrichment service rather than remain a separate special-case path.
- Existing AI-tab result display can be reused, but it should receive richer structured summaries.
- Future scheduled tasks can reuse the same backend command once the core service is stable.
