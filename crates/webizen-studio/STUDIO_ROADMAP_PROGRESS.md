# Webizen Studio — 5-Phase Roadmap Progress

_Branch: `0.0.23` | Tracker for Agent 1/2/3 delegation plan_

## Phase 1 — Premium Visual Identity & Theme Engine

| Item | Status | Notes |
|------|--------|-------|
| 4 QPrime presets defined | ✅ | fiduciary-dark, commons-light, sanctuary, infosphere |
| Scoped theming (env/app/page/module) | ✅ | studio_canvas.rs |
| THEMING.md | ✅ | |
| Shoelace wrappers | ✅ | components/shoelace.rs |
| fiduciary-dark default cold start | ✅ | main.rs, dashboard, new_workspace_shell |
| Shoelace ↔ QPrime token bridge | ✅ | shoelace_bridge_css() |
| Shoelace bundled offline | ✅ | assets/vendor/shoelace |
| qprime_elevation_css (breathe, glass) | ✅ | canvas_editor.rs |
| Spring motion wired | ✅ | selection + mode pulse; global/workspace theme class |
| Elevation utility .elevation-0..3 | ✅ | token + utility classes in qprime_elevation_css |
| Theme picker: 4 QPrime foreground | ✅ | dashboard optgroups + settings ordering |
| Sanctuary disables all springs | ✅ | timeline_from_theme + CSS + sanctuary class |

**Acceptance:** picker shows QPrime first; switch updates shell + canvas + Shoelace; sanctuary = no motion; fiduciary-dark on cold start.

## Phase 2 — Interactive QAPP Studio Editor

| Item | Status | Notes |
|------|--------|-------|
| Palette drag-to-add | ✅ | |
| Edit / Preview mode | ✅ | CanvasEditorMode |
| Pane drag + resize | ✅ | PaneInteraction |
| Live inspector x/y/w/h | ✅ | |
| Layer + anchor + data_bindings | ✅ | |
| Presentation mode toggle | ✅ | Grid / Nodes / Spatial |
| Undo/redo (32 deep) | ✅ | WorkspaceHistory |
| WAL append + replay | ✅ | qualia-client-core + settings :8080 |
| Ontology import + job poll | ✅ | ontology_import_wizard.rs |
| Theme binding picker | ✅ | inspector QPrime preset select |
| LLM pane generation | ✅ | pane_generator.rs keyword planner + prompt bar |
| Semantic auto-layout | ◑ | ontology wizard presets only |

## Phase 3 — Multi-Mode Renderers (Agent 3)

| Item | Status | Notes |
|------|--------|-------|
| node_graph SVG edges | ✅ | strength glow |
| spatial_bridge Live portal | ✅ | iframe + offline WASM |
| canvas_graph edge derivation | ✅ | |

## Phase 4 — Deploy, WAL & Provenance

| Item | Status | Notes |
|------|--------|-------|
| WAL append / snapshots / history | ✅ | |
| Replay API + UI | ✅ | wal_inspector + studio sidebar |
| Provenance chips | ✅ | theme_binding_provenance |

## Phase 5 — Integration & E2E

| Item | Status | Notes |
|------|--------|-------|
| wasm32 + desktop check | ✅ | |
| Portal WASM offline bundle | ✅ | static/portal/pkg/qualia |
| pane_generator + theme unit tests | ✅ | |
| Full Tauri E2E smoke | ❌ | manual / future script |
| IHP / PDF / extension stretch | ❌ | separate sprints |

---

## Session log

### 2026-07-01 — Grok (phase 1–2 close-out)

- Added this tracker.
- QPrime-first theme picker (optgroups), elevation utilities, sanctuary→springs.
- Theme binding picker in property inspector.
- `pane_generator.rs` — keyword pane planner for prompt bar.
- App shell theme class + Shoelace bridge on `:root`.
- Verified: `cargo check -p webizen-studio --target wasm32-unknown-unknown`.