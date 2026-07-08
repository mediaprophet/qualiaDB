# Webizen Desktop — Proper Application Architecture

**Date:** 2026-07-08
**Status:** Active
**Decisions:** Native Tauri shell + Separate WASM QApp modules + Parallel fix/build

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Native Tauri Shell (Rust)                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Menu Bar: File | Edit | View | QApps | Tools | Help   │  │
│  ├───────────────────────────────────────────────────────┤  │
│  │ Tab Strip: [Dashboard] [WellFair] [Chora] [Browser+] │  │
│  ├───────────────────────────────────────────────────────┤  │
│  │ Address Bar: qualia://wellfair/health                 │  │
│  ├───────────────┬───────────────────────────────────────┤  │
│  │  Native GPU    │  WASM QApp Canvas (WebView)          │  │
│  │  Surface       │  ┌─────────────────────────────┐     │  │
│  │  (wgpu direct  │  │ Active QApp WASM Module     │     │  │
│  │   to swapchain)│  │ (loaded on demand per tab)  │     │  │
│  │                │  └─────────────────────────────┘     │  │
│  ├───────────────┴───────────────────────────────────────┤  │
│  │ Status Bar: daemon ● | vault 🔒 | sync ⟳ | GPU 60fps │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  System Tray: background runner with full menu              │
└─────────────────────────────────────────────────────────────┘
```

## Track A: Fix Critical Bugs (parallel)

### A1: WellFair use_effect infinite loop
**Problem:** 36 panels use `use_effect(move || { reload(); })` where `reload` spawns a task that sets signals, which triggers re-render, which re-runs the effect.
**Fix:** Replace with a `use_once` pattern — use `use_signal` for a `loaded` flag, check it in the effect, set it before spawning.
**Files:** All 36 panels in `crates/webizen-studio/src/components/wellfair/`

### A2: Disabled solvers
**Problem:** calculus, linear_algebra, optimization, quantum_optimizers, symbolic_logic have build errors (ExecutionError/SolverState refs)
**Fix:** Update references to current API
**Files:** `crates/qualia-core-db/src/solvers/`

### A3: Mocked LLM in WASM path
**Problem:** WASM path uses mock ring buffer; native path uses real GGUF inference
**Fix:** Document that WASM path is intentionally limited (no GGUF in browser); ensure native desktop uses real inference path
**Status:** This is by design — WASM can't load GGUF files. The native desktop should use the real path.

## Track B: Build Native Desktop Application

### B1: Native menu bar + tab strip
**What:** Tauri native menu (File, Edit, View, QApps, Tools, Help) + tab strip component
**Files:** `crates/webizen-desktop/src/main.rs`, new `crates/webizen-desktop/src/shell/`

### B2: QApp loader infrastructure
**What:** Load separate WASM modules on demand per tab. Each QApp is a self-contained WASM file served by the QApp protocol server (port 4567).
**Mechanism:**
- Each QApp has a `qapp.json` manifest (already exists in the spec)
- The native shell loads `qualia://{qapp_id}/index.html` into a WebView per tab
- The WebView loads the QApp's WASM module
- QApps call Tauri commands via `window.__TAURI__.core.invoke()`
**Files:** `crates/webizen-desktop/src/qapp_loader.rs`, `crates/webizen-desktop/src/shell/`

### B3: Native GPU surface integration
**What:** The native GPU surface (already built) renders 10D content directly to the window, composited alongside the WebView content
**Status:** Partially done — `native_surface.rs` exists and works. Needs integration with tab system.
**Files:** `crates/webizen-desktop/src/native_surface.rs`, `crates/webizen-desktop/src/shell/`

### B4: System tray (background runner)
**What:** Full system tray with all submenus (already built, needs polish)
**Status:** Done — needs testing and integration with new shell
**Files:** `crates/webizen-desktop/src/main.rs`

### B5: Status bar
**What:** Native status bar showing daemon status, vault state, sync state, GPU FPS
**Files:** `crates/webizen-desktop/src/shell/status_bar.rs`

### B6: Split monolithic WASM into QApp modules
**What:** Break the current monolithic `webizen-studio` WASM into separate QApp modules:
  - `wellfair` — all 43 WellFair panels
  - `chora` — spatio-temporal commons
  - `browser` — web browser functionality
  - `dashboard` — main dashboard
  - `10d-browser` — 10D container browser
  - `qapp-studio` — QApp development environment
  - Each academic discipline QApp (150+) as separate modules
**Files:** New crate structure or feature-gated builds within `webizen-studio`

### B7: Cross-platform (macOS)
**What:** Ensure GPU surface works on macOS (Metal via wgpu), menu bar uses macOS native menu
**Files:** `crates/webizen-desktop/src/native_surface.rs` (add macOS path)

## Execution Order

1. **A1** (WellFair fix) — parallel with B1
2. **B1** (native menu + tabs) — foundation for everything else
3. **B2** (QApp loader) — depends on B1
4. **B3** (GPU surface integration) — depends on B1
5. **A2** (solvers fix) — parallel
6. **B4+B5** (tray + status bar) — after B1
7. **B6** (split WASM) — after B2
8. **B7** (macOS) — after B3
