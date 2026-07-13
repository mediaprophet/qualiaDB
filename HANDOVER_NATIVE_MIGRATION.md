# Hand-Over: Webizen Desktop Native Migration (2026-07-08)

## Session Goal
Migrate the Webizen Desktop from a WASM-in-webview architecture to a native x64 architecture, with a local web server for browser access.

## What Was Done (All Complete)

### 1. Architecture Audit (DESKTOP_AUDIT.md)
- Created comprehensive audit at `DESKTOP_AUDIT.md` documenting the WASM-in-webview problem
- Identified that `webizen-studio/Cargo.toml` used `dioxus = { features = ["router", "web"] }` (WASM) instead of `desktop` (native)
- Documented that all `cfg(not(target_arch = "wasm32"))` branches were dead code on desktop
- Documented the 330+ Tauri IPC commands that exist because of the WASM architecture
- Mapped all ports: 8080 (settings), 4242 (graph daemon), 4567 (QApp protocol)

### 2. Studio Migration to Native Dioxus Desktop
- **Cargo.toml**: Changed from `dioxus = { features = ["router", "web"] }` to platform-specific:
  - `cfg(not(target_arch = "wasm32"))`: `dioxus = { features = ["router", "desktop"] }` + native deps (`qualia-core-db`, `webizen-render`, `qualia-client-core`, `tokio`, `dirs-next`)
  - `cfg(target_arch = "wasm32")`: `dioxus = { features = ["router", "web"] }` + wasm deps (`wasm-bindgen`, `web-sys`, etc.)
- **Dioxus upgraded** from 0.7.9 to 0.8.0-alpha.0 to resolve `wry` version conflict (0.53 vs 0.55)
- **18 component files updated** to use dual-mode `invoke_json()` instead of `wasm_bindgen`-based `tauri_invoke()`:
  - `qapp_engine.rs` — new dual-mode `invoke_json()` helper (wasm: JS bridge, native: REST to :8080)
  - `anatomy_test.rs`, `browser_panes.rs`, `chemistry_modeler.rs`, `clinical_risk_scorer.rs`, `comorbidity_analyzer.rs`, `dicom_viewer.rs`, `hardware_configurator.rs`, `native_gpu_viewport.rs`, `portfolio_analyzer.rs`, `provenance_graph.rs`, `qaoa_explorer.rs`, `quantum_dft.rs`, `risk_engine.rs`, `solid_ldp_browser.rs`, `sparql_explorer.rs`, `ten_d_browser.rs`
  - `telemetry.rs` — split into wasm32 (web_sys::window/local_storage) and native (file-based) versions
- **Both builds compile**: `cargo check -p webizen-studio` (native) and `cargo check -p webizen-studio --target wasm32-unknown-unknown` (wasm) both pass

### 3. Native GPU Surface Viewport
- Created `native_gpu_viewport.rs` component with `NativeGpuViewport` and `NativeGpuViewportPage`
- Added `/gpu-viewport` route to `main.rs` with nav item
- Component calls `mount_gpu_surface` / `set_gpu_camera` / `unmount_gpu_surface` via `invoke_json`
- Mouse orbit controls (yaw/pitch) + wheel zoom
- Status overlay showing GPU surface state

### 4. External WASM Telemetry Ingestion (port 4242)
- Added `POST /telemetry/ingest` endpoint to `qualia-core-db/src/services/webizen_server.rs`
- Accepts `{ "source": "device-id", "telemetry": { ...SystemTelemetry fields } }`
- Broadcasts ingested telemetry to all connected subscribers (ambient viz, render bridge)
- Added `Serialize`/`Deserialize` derives to `qualia-core-db/src/render/telemetry.rs::SystemTelemetry`

### 5. Studio WASM Served from :8080
- Added `/studio` route to settings server (`webizen-desktop/src/settings_server.rs`)
- Serves the Studio WASM build from `target/dx/webizen-studio/release/web/public/` (or `QUALIA_STUDIO_WASM_DIR` env var override)
- Browser-accessible Studio UI at `http://localhost:8080/studio/`

### 6. REST Invoke Proxy (settings server → Tauri commands)
- Added `POST /api/invoke/{cmd}` endpoint to settings server
- Dispatches through the Tauri webview's `on_message` IPC handler
- Allows the native Studio (and any browser client) to call all 330+ Tauri commands via REST
- `APP_HANDLE` static stores the Tauri app handle for the proxy

### 7. User-Configurable Port 8080
- Added `settings_port: u16` field to `AgentConfig` in `qualia-client-core/src/state.rs`
- Default: 8080. If 0, auto-finds open port.
- Settings server checks user-configured port first, falls back to auto-find if taken
- `save_config` Tauri command already exists to change it from the UI

## Build Status

| Target | Status | Notes |
|--------|--------|-------|
| `cargo check -p webizen-studio` (native) | ✅ Passes | Zero errors, warnings only |
| `cargo check -p webizen-studio --target wasm32-unknown-unknown` | ✅ Passes | Warnings only (dead code) |
| `cargo check -p webizen-desktop` | ✅ Passes | One warning (fixed) |
| `cargo check -p qualia-core-db` | ✅ Passes | |
| `cargo check -p qualia-client-core` | ✅ Passes | |
| `dx build --release --web` (Studio WASM) | ✅ Built | Output at `target/dx/webizen-studio/release/web/public/` |
| `cargo build --release -p webizen-desktop` | ✅ Built | 53.2 MB binary at `target/release/webizen-desktop.exe` |
| `cargo build --release -p webizen-studio` (native binary) | ❌ Not built | Windows file lock issue (os error 32) interrupted the build |

## What's Left To Do

### Immediate (next session)
1. **Build the Studio native binary** — `cargo build --release -p webizen-studio` was interrupted by Windows file locking (os error 32). Just needs a retry with `CARGO_BUILD_JOBS=1` after clearing the locked files. The code compiles; it's purely a Windows I/O issue.
2. **Verify the desktop launches** — run `webizen-desktop.exe` and confirm:
   - The new UI loads (no "Host API v1" screen)
   - The `/gpu-viewport` route works
   - The settings server on :8080 serves the Studio WASM at `/studio/`
   - The `/telemetry/ingest` endpoint accepts POSTs on :4242
3. **Commit the changes** — 111 files changed, 5714 insertions, 4915 deletions. This is a major architecture change that should be committed as a single atomic commit.

### Follow-up (future sessions)
4. **Replace updater placeholder key** — `tauri.conf.json` has `PLACEHOLDER_REPLACE_WITH_TAURI_SIGN_PUBLIC_KEY`
5. **Finish PWA secure-origin delivery** — WebRTC data channel for LAN phone install
6. **Wire PGA projector.wgsl to the native surface** — connect the semantic scene graph to the desktop renderer
7. **Audit the separate `C:\Projects\webizen-browser` repo** — determine what should be merged vs deprecated
8. **Dioxus 0.8 API migration** — some components may use deprecated 0.7 patterns that still compile but could use 0.8 features (incremental rendering, custom elements)

## Key Files Changed

### Architecture-critical
- `crates/webizen-studio/Cargo.toml` — platform-specific dioxus features + deps
- `crates/webizen-studio/src/components/qapp_engine.rs` — dual-mode `invoke_json()` helper
- `crates/webizen-studio/src/telemetry.rs` — wasm32 vs native split
- `crates/webizen-desktop/src/settings_server.rs` — REST invoke proxy + Studio WASM serving + configurable port
- `crates/qualia-client-core/src/state.rs` — `settings_port` field added to `AgentConfig`
- `crates/qualia-core-db/src/services/webizen_server.rs` — `/telemetry/ingest` endpoint
- `crates/qualia-core-db/src/render/telemetry.rs` — serde derives on `SystemTelemetry`

### New files
- `DESKTOP_AUDIT.md` — comprehensive architecture audit
- `crates/webizen-studio/src/components/native_gpu_viewport.rs` — native GPU surface component

### Component files updated (wasm_bindgen → invoke_json)
- 18 files in `crates/webizen-studio/src/components/` — all use `invoke_json` now

## Important Notes
- The `dioxus` upgrade from 0.7.9 to 0.8.0-alpha.0 was necessary to resolve a `wry` version conflict between `dioxus-desktop` (wry 0.53) and `tauri` (wry 0.55). Dioxus 0.8.0-alpha.0 was published 29 days ago (2026-05-19), well past the 7-day vetting window.
- The WASM build path is still maintained — the Studio can be built as either native (desktop) or WASM (web server). The `dx build --release --web` flag selects the WASM target.
- The `pane_registry.rs::q42()` function now returns real hashes on native builds (via `qualia_core_db::q_hash()`), enabling semantic hash dispatch that was previously dead code.
