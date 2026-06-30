---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# ingest Index

## Functionality Overview
Comprehensive index of functionality for `ingest`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `acquire.rs`
  - `enum SourceKind`
  - `impl SourceKind`
  - `fn mime`
  - `struct Acquired`
  - `fn detect_kind`
  - `fn acquire`
- 📄 `chunk.rs`
  - `struct Chunk`
  - `fn chunk_text`
  - `fn chunks_to_jsonl`
  - `fn split_blocks`
  - `fn heading_level`
  - `fn update_heading_path`
  - `fn floor_char_boundary`
  - `fn chunks_respect_headings_and_size`
  - `fn jsonl_round_trips`
- 📄 `cml.rs`
  - `fn build_cml`
  - `fn detect_method`
  - `fn truncate`
  - `fn ttl_str`
- 📄 `extract.rs`
  - `struct Extracted`
  - `fn extract`
  - `fn extract_pdf`
  - `fn pdf_page_count`
  - `fn text_to_html`
  - `fn looks_like_heading`
  - `fn html_to_text`
- 📄 `mod.rs`
  - `struct IngestOptions`
  - `impl Default`
  - `fn default`
  - `struct IngestResult`
  - `fn ingest_file`
  - `fn ingest_path`
  - `fn is_ingestible`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
