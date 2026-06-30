---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# src Index

## Functionality Overview
Comprehensive index of functionality for `src`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[bin](bin/DIRECTORY_INDEX.md)`
- 📁 `[container](container/DIRECTORY_INDEX.md)`
- 📁 `[ingest](ingest/DIRECTORY_INDEX.md)`
- 📁 `[llm](llm/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `embedding.rs`
  - `fn encode_f32_matrix`
  - `fn decode_f32_matrix`
  - `fn cosine`
  - `fn centroid`
  - `fn matrix_round_trips`
  - `fn cosine_basics`
- 📄 `lib.rs`
- 📄 `library.rs`
  - `struct Entry`
  - `impl Entry`
  - `fn title`
  - `struct Library`
  - `struct Hit`
  - `impl Library`
  - `fn scan`
  - `fn len`
  - `fn is_empty`
  - `fn exact_duplicates`
  - `fn load_vectors`
  - `fn document_centroid`
  - `fn near_duplicates`
  - `fn search`
  - `fn novelty_ranking`
  - *(...and 1 more)*
- 📄 `reorganize.rs`
  - `struct PlacementOp`
  - `enum ApplyMode`
  - `fn plan`
  - `fn apply`
  - `fn category_for`
  - `fn safe_title`
  - `fn safe_component`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
