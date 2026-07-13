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
- 📁 `[components](components/DIRECTORY_INDEX.md)`
- 📁 `[render](render/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `endpoints.rs`
  - `enum HostSurface`
  - `fn manifest_url`
  - `fn telemetry_url`
  - `fn is_native_host`
  - `fn current_host_surface`
  - `fn supports_browser_pane`
- 📄 `lib.rs`
- 📄 `main.rs`
  - `fn tauri_listen`
  - `fn main`
  - `enum Route`
  - `fn AnatomyTestRoute`
  - `fn DashboardRoute`
  - `fn ContextStudioRoute`
  - `fn QAppsRoute`
  - `fn BrowserRoute`
  - `fn StudioEditRoute`
  - `fn StudioRoute`
  - `fn RenderPreviewRoute`
  - `fn SceneInteractionRoute`
  - `fn NexusRoute`
  - `fn SettingsRoute`
  - `fn AboutRoute`
  - *(...and 2 more)*
- 📄 `pane_registry.rs`
  - `fn q42`
  - `struct PaneDefinition`
  - `enum PaneCategory`
  - `fn builtin_pane_definitions`
  - `fn find_pane`
  - `fn category_label`
- 📄 `studio_canvas.rs`
  - `struct NQuin`
  - `enum LayoutStrategy`
  - `impl Default`
  - `fn default`
  - `enum PresentationMode`
  - `enum CoordinateSpace`
  - `enum UiMode`
  - `enum LayerBehavior`
  - `struct PanePlacement`
  - `struct Page`
  - `struct WebizenWorkspace`
  - `fn p`
  - `fn app_display_name`
  - `fn default_panes_for_app`
  - `fn default_presentation_mode`
  - *(...and 10 more)*
- 📄 `telemetry.rs`
  - `fn use_telemetry`
- 📄 `theme_engine.rs`
  - `struct ThemeDefinition`
  - `struct ThemeBinding`
  - `struct ResolvedTheme`
  - `fn builtin_theme_catalog`
  - `fn resolve_theme`
  - `fn render_scope_tokens`
  - `fn collect_stylesheets`
  - `fn join_theme_classes`
  - `fn push_stylesheet`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
