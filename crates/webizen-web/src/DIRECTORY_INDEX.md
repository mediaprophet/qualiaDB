---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# src Index

## Functionality Overview
Comprehensive index of functionality for `src`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `lib.rs`
  - `fn log`
  - `fn init_panic_hook`
  - `struct WebEngine`
  - `impl WebEngine`
  - `fn new`
  - `fn load_q42`
  - `fn load_json_scene`
  - `fn render_to_canvas`
  - `fn last_parsed`
  - `fn mount_qapp`
  - `fn create_canvas`
- 📄 `qualia_portal.rs`
  - `enum DisplayMode`
  - `impl DisplayMode`
  - `fn from_str`
  - `struct ProjectedNode`
  - `fn log`
  - `struct QualiaPortal`
  - `impl QualiaPortal`
  - `fn new`
  - `fn tier`
  - `fn operational_mode`
  - `fn resize`
  - `fn tick`
  - `fn set_telemetry`
  - `fn set_display_mode`
  - `fn encode_geometry`
  - *(...and 24 more)*
- 📄 `render_stub.rs`
  - `struct SimpleCanvasRenderer`
  - `impl SimpleCanvasRenderer`
  - `fn new`
  - `fn clear`
  - `fn draw_label`
  - `fn draw_node`
  - `fn draw_edge`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
