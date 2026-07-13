---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# container Index

## Functionality Overview
Comprehensive index of functionality for `container`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `manifest.rs`
  - `enum AssetKind`
  - `impl AssetKind`
  - `fn dir`
  - `struct AssetEntry`
  - `struct SourceInfo`
  - `struct PipelineInfo`
  - `struct StatusFlags`
  - `struct HmcManifest`
  - `impl HmcManifest`
  - `fn new`
  - `fn asset_of`
- 📄 `mod.rs`
  - `enum HmcError`
  - `fn blake3_hex`
  - `struct HmcWriter`
  - `impl HmcWriter`
  - `fn new`
  - `fn push_asset`
  - `fn add_derived`
  - `fn manifest_mut`
  - `fn to_bytes`
  - `fn member_mime`
  - `fn write_to_dir`
  - `fn manifest`
  - `fn reopen`
  - `struct HmcContainer`
  - `impl HmcContainer`
  - *(...and 7 more)*
- 📄 `tests.rs`
  - `fn sample_source`
  - `fn round_trips_manifest_and_assets`
  - `fn add_derived_is_idempotent`
  - `fn filename_is_sanitized_no_traversal`
  - `fn rejects_non_hmc_archive`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
