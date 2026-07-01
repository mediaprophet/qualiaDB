# Webizen Studio — 5-Phase Roadmap Progress

_Branch: `0.0.23` | Tracker for Agent 1/2/3 delegation plan_

Use this file as the **session reminder**: what is done, what is next, and where the code lives.

---

## Quick reference — where things live

| Concern | Primary files |
|---------|---------------|
| Theme engine / QPrime | `theme_engine.rs`, `canvas_editor.rs`, `main.rs` |
| Canvas editor | `studio_canvas.rs`, `canvas_editor.rs` |
| Pane generation (API) | `qualia-client-core/src/studio_pane_generator.rs` |
| Pane generation (wasm fallback) | `pane_generator.rs` |
| Settings portal routes | `webizen-desktop/src/settings_server.rs` |
| WAL replay | `qualia-client-core/src/studio_workspace_wal.rs`, `wal_inspector.rs` |
| Ontology import + layouts | `ontology_import_wizard.rs` |
| Portal WASM (offline spatial) | `static/portal/pkg/qualia/`, `scripts/bundle-desktop-deps.ps1` |
| E2E smoke | `scripts/studio-portal-smoke.ps1` |

---

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

**Remaining:** manual cold-start verify in running Tauri app (Timothy or next session).

---

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
| LLM pane generation | ✅ | `POST /generate_pane` + wasm fallback |
| Semantic auto-layout | ✅ | domain presets in studio_pane_generator (legal/health/commons/semantics) |

**Remaining (optional):** quin-encoded undo chain; real LLM infer hook (beyond keyword planner).

---

## Phase 3 — Multi-Mode Renderers (Agent 3)

| Item | Status | Notes |
|------|--------|-------|
| node_graph SVG edges | ✅ | strength glow |
| spatial_bridge Live portal | ✅ | iframe + offline WASM |
| canvas_graph edge derivation | ✅ | |
| Native PortalGpu parity (PR-C10) | ✅ | workspace panes → GPU scene via `merge_workspace_panes` |

---

## Phase 4 — Deploy, WAL & Provenance

| Item | Status | Notes |
|------|--------|-------|
| WAL append / snapshots / history | ✅ | |
| Replay API + UI | ✅ | wal_inspector + studio sidebar |
| Provenance chips | ✅ | theme_binding_provenance |

---

## Phase 5 — Integration & E2E

| Item | Status | Notes |
|------|--------|-------|
| wasm32 + desktop check | ✅ | |
| Portal WASM offline bundle | ✅ | static/portal/pkg/qualia |
| pane_generator + theme unit tests | ✅ | |
| Settings portal HTTP smoke | ✅ | scripts/studio-portal-smoke.ps1 |
| Full Tauri GUI E2E | ❌ | cold start → edit → save → WAL restore (manual) |
| IHP / PDF / extension stretch | ❌ | separate sprints |

---

## Next actions (pick up here)

1. **Run smoke** with desktop open: `.\scripts\studio-portal-smoke.ps1`
2. **Phase 3 (stretch):** feed live QualiaDB graph into spatial scene (beyond mock neighborhood)
3. **Phase 5:** Tauri GUI walkthrough script (optional Playwright/webdriver)
4. **Stretch:** wire real local LLM infer into `/generate_pane` (orchestrator path)
5. **Push** branch `0.0.23` after each verified slice

---

## Session log

### 2026-07-01 — Grok (phase 1–2 close-out)

- Added this tracker.
- QPrime-first theme picker (optgroups), elevation utilities, sanctuary→springs.
- Theme binding picker in property inspector.
- `pane_generator.rs` — keyword pane planner for prompt bar.
- App shell theme class + Shoelace bridge on `:root`.
- Verified: `cargo check -p webizen-studio --target wasm32-unknown-unknown`.

### 2026-07-01 — Grok (generate_pane API + smoke)

- `qualia-client-core/src/studio_pane_generator.rs` — shared planner + ontology domain presets.
- `POST /generate_pane` on settings portal (`settings_server.rs`).
- Studio prompt bar calls API on desktop; local fallback for GitHub Pages demo.
- `scripts/studio-portal-smoke.ps1` — HTTP smoke (health, generate, manifest, history, spatial shell).

### 2026-07-01 — Grok (PR-C10 workspace → PortalGpu)

- `render_pipeline::merge_workspace_panes` — maps studio pane grid to manifold nodes + binding edges.
- `update_render_preview` accepts optional `panes`; spatial_bridge passes current page layout.
- Tests: `workspace_panes_merge_into_scene` + wasm32 check green.