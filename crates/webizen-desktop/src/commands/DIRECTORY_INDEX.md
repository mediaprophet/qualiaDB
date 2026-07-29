---
created: 2026-06-30
updated: 2026-07-29
update_scope: Minor
---

# commands Index

## Functionality Overview
Comprehensive index of functionality for `commands`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `agent_qa.rs`
  - `struct AgentQaSnapshot`
  - `struct AgentQaModelProbe`
  - `fn agent_qa_snapshot`
  - `fn agent_qa_test_active_model`
  - reversible temporary-session cleanup and bounded inference evidence
- 📄 `binary_registry.rs`
  - `struct BinaryNodeRegistry`
  - `impl BinaryNodeRegistry`
  - `fn new`
  - `fn register`
  - `fn get_index`
  - `fn get_id`
  - `fn len`
  - `fn clear`
  - `impl Default`
  - `fn default`
  - `fn test_binary_node_registry`
  - `fn test_registry_clear`
- 📄 `glb_ingest.rs`
  - `struct GLBMetadata`
  - `struct GLBView`
  - `fn new`
  - `fn header`
  - `fn is_valid_glb`
  - `fn json_chunk_length`
  - `fn json_chunk`
  - `fn binary_chunk`
  - `struct GLBIngestionManager`
  - `impl GLBIngestionManager`
  - `fn load_glb`
  - `fn create_view`
  - `fn get_vh_male_v14_assets`
  - `impl Default`
  - `fn default`
  - *(...and 14 more)*
- 📄 `mod.rs`
  - `struct DiffusionConfigInput`
  - `struct LocalPreviewProbe`
  - `struct QappWasmExport`
  - `fn resolve_web_pkg_src`
  - `fn guess_lan_ipv4`
  - `fn ensure_lan_export_server`
  - `struct TemporalSlice`
  - `impl TemporalSlice`
  - `fn get`
  - `fn set`
  - `fn list_installed_qapps`
  - `fn generate_qapp_credential`
  - `fn verify_and_install_qapp`
  - `fn launch_installed_qapp`
  - `fn get_hardware_status`
  - *(...and 136 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
- **2026-07-29**: Added the 0.0.28 structured diagnostics snapshot and reversible local-model probe.
