# Poet Native & WASM Implementation Plan

**Date:** 2026-10-03  
**Branch:** `rolling-commons/phase0`  
**Status:** In Progress — Full Implementation First  

---

## 1. Architectural Scope & Goals
1. **One Poet, Two Hosts**:
   - **Native Desktop Mode**: First-class citizen inside Webizen Desktop OS Shell, launchable on stage (`/volumes/poet`) or in a dedicated native window (`POST /api/shell/open-window`).
   - **Browser WASM Mode**: Browser-native standalone app hosted under `/poet-app/index.html`, deployable to GitHub Pages and local portal servers.
2. **Zero "Held / Not Yet" Theatre**:
   - Remove the broken dependency on the legacy Studio SPA bundle (`/studio/index.html#/poet`).
   - Elevate Poet into a complete, self-contained Cyber-Semantic HyperCanvas environment.
3. **Backend Integration**:
   - Native VibeScript execution (`POST /api/vibe/eval`).
   - SPARQL / Graph queries (`POST /api/sparql/query`).
   - Q42 mutable journal snapshotting and compaction (`POST /api/q42/compact`).
   - Deterministic Clinical calculations (`POST /api/clinical/framingham`).
   - Local LLM model discovery and inference (`GET /api/llm/models`, `POST /api/llm/infer`).

---

## 2. Implementation Deliverables
- [ ] `crates/webizen-desktop/static/os-shell/volumes/poet.html`: Full Cyber-Semantic HyperCanvas workbench (4-way dock, 15 toolboxes, infinite canvas, containers, wires, 8-sector radial menu, 4D scrubber, dual launch).
- [ ] `crates/webizen-desktop/src/settings_server.rs`:
  - Route `/poet-app` nested service to `portal_support_root.join("poet-app")`.
  - Add `POST /api/vibe/eval` endpoint for server-side VibeScript evaluation.
  - Update `open_poet_window_handler` and `open_poet_window` in `menu.rs` to load `/volumes/poet` natively.
- [ ] `crates/poet/src/lib.rs` & `crates/poet/src/browser/mod.rs`: Add `#[wasm_bindgen(start)]` and `run_poet()` export.
- [ ] `crates/webizen-desktop/static/portal/poet-app/`: Mirror `docs/poet-app/` bundle.
- [ ] `scripts/package-poet-wasm.ps1`: Automated packaging script for Poet WASM.
- [ ] Unit & Route Tests: Verify `/volumes/poet`, `/poet-app/index.html`, `/api/vibe/eval`, and window launching.
