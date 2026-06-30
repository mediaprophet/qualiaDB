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
- 📁 `[commands](commands/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `main.rs`
  - `fn protocol_response`
  - `fn diffusion_frame_response`
  - `fn render_preview_response`
  - `fn webizen_protocol_response`
  - `fn show_main_window`
  - `fn main`
  - `fn test_generate_qapp_credential`
- 📄 `runtime.rs`
  - `struct RuntimeSnapshotRecord`
  - `struct RuntimeLedgerHealth`
  - `struct LedgerHealthFingerprint`
  - `enum RuntimeSignal`
  - `struct LedgerMetrics`
  - `impl LedgerMetrics`
  - `fn snapshot`
  - `fn fingerprint`
  - `fn note_drop`
  - `fn note_gap`
  - `fn note_persisted`
  - `fn note_write_failure`
  - `struct DesktopLedgerSink`
  - `impl DesktopLedgerSink`
  - `fn bounded`
  - *(...and 16 more)*
- 📄 `settings_server.rs`
  - `struct SettingsServerState`
  - `struct HealthResponse`
  - `struct StatusResponse`
  - `struct JobQueueCounts`
  - `fn find_open_port`
  - `fn static_portal_dir`
  - `fn spawn_settings_server`
  - `fn run_settings_server`
  - `fn health_handler`
  - `fn status_handler`
  - `fn probe_graph_daemon`
  - `fn get_config_handler`
  - `fn save_config_handler`
  - `fn get_manifest_handler`
  - `fn post_manifest_handler`
  - *(...and 21 more)*
- 📄 `telemetry_bridge.rs`
  - `struct TelemetryBridge`
  - `impl TelemetryBridge`
  - `fn new`
  - `fn is_enabled`
  - `fn toggle`
  - `fn get_telemetry`
  - `fn set_telemetry`
  - `fn set_memory_pressure`
  - `fn set_network_ripple`
  - `fn set_baking_crystallization`
  - `fn set_logic_flashes`
  - `fn set_llm_heat`
  - `fn set_quantum_activity`
  - `fn set_spectral_shift`
  - `fn set_temporal_pulse`
  - *(...and 14 more)*
- 📄 `telemetry_hooks.rs`
  - `fn get_memory_pressure`
  - `fn get_network_ripple`
  - `fn get_baking_crystallization`
  - `fn get_logic_flashes`
  - `fn get_inference_heat`
  - `fn get_quantum_activity`
  - `fn get_spectral_shift`
  - `fn get_temporal_pulse`
  - `fn get_epistemic_density`
  - `fn get_manifold_pressure`
  - `fn increment_inference_counter`
  - `fn increment_network_io_counter`
  - `fn increment_baking_counter`
  - `fn increment_query_resolve_counter`
  - `fn increment_quantum_activity_counter`
  - *(...and 7 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
