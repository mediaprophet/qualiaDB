---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# extensions Index

## Functionality Overview
Comprehensive index of functionality for `extensions`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `extension_bus.rs`
  - `struct ExtensionBus`
  - `impl ExtensionBus`
  - `fn new`
  - `fn scan_and_load_extensions`
  - `fn register_extension_from_path`
  - `fn list_extensions`
  - `fn query_capability`
  - `fn dispatch_task`
  - `fn provision_asset`
  - `struct ExtensionBusState`
  - `struct ChallengePayload`
  - `struct IntentPayload`
  - `fn init_extension_bus`
  - `fn is_connected`
  - `fn send_intent`
- 📄 `extension_manifest.rs`
  - `enum TransportProtocol`
  - `enum SandboxLevel`
  - `struct ExtensionCapability`
  - `struct ExtensionSecurity`
  - `struct ExtensionManifest`
  - `impl ExtensionManifest`
  - `fn from_json`
- 📄 `mod.rs`
- 📄 `resource_catalog.rs`
  - `struct DownloadInfo`
  - `impl DownloadInfo`
  - `fn resolved_url`
  - `fn local_filename`
  - `struct LLMResource`
  - `impl LLMResource`
  - `fn is_multimodal`
  - `fn effective_context_window`
  - `fn to_quins`
  - `fn provenance_quin`
  - `fn source_url_quin`
  - `fn to_capability_profile`
  - `fn to_capability_profile_with_projector`
  - `fn quin`
  - `struct OntologyResource`
  - *(...and 28 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
