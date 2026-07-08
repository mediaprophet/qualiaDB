# Webizen Desktop — Functionality Audit (Revised)

**Date:** 2026-07-08 (revised)
**Auditor:** GLM-5.2 (Devin)
**Branch:** v0.0.23
**Purpose:** Assess the desktop app's architecture, identify the WASM-in-webview problem, and define the target native architecture.

---

## 0. Critical Architecture Finding — The Studio is WASM, Not Native

### The Problem

The `webizen-studio` frontend is compiled to **WASM** and served inside Tauri's **webview** — it is NOT a native Dioxus desktop app. This is the single biggest architectural issue in the desktop build.

**Evidence:**

| File | Line | What it shows |
|------|------|---------------|
| `webizen-studio/Cargo.toml` | 7 | `dioxus = { features = ["router", "web"] }` — the **web** renderer, not `desktop` |
| `webizen-studio/Cargo.toml` | 11–17 | `wasm-bindgen`, `js-sys`, `web-sys`, `wasm-bindgen-futures`, `serde-wasm-bindgen` — all WASM-only |
| `tauri.conf.json` | 7 | `"frontendDist": "../webizen-studio/dist"` — Tauri serves compiled WASM/HTML |
| `endpoints.rs` | 129–153 | `is_native_host()` checks `window.__TAURI__` — it's in a webview |
| `pane_registry.rs` | — | `q42()` returns `None` on `wasm32` — semantic hash dispatch is dead |

### Consequences

1. **Every UI→backend call crosses the JS↔Rust IPC boundary.** The 330+ Tauri `#[command]` handlers exist *because* of this — every function call must be serialised to JSON, sent over `__TAURI__.invoke()`, deserialised, executed, re-serialised, and deserialised again.

2. **All `cfg(not(target_arch = "wasm32"))` branches are dead code on desktop.** The Cargo.toml has `qualia-core-db` and `webizen-render` as native-only deps (lines 29–32), but they never compile because the desktop always takes the WASM path. The Studio has **zero direct access** to the engine or renderer.

3. **Semantic hash dispatch is dead.** `pane_registry.rs::q42()` returns `None` on wasm32 — the pane registry can never auto-dispatch by RDF type hash because `q_hash()` isn't available.

4. **Direct GPU access is blocked.** The native GPU surface (`native_surface.rs`) can only be invoked via Tauri IPC commands, not directly from the renderer.

5. **The `dx build --release` step is required** to compile the Studio to WASM, then the result must be copied to `dist/`, then Tauri must re-embed it. This three-step build is fragile and the binary can ship with stale assets (which happened — the "Host API v1" screen was the old WASM).

### The Fix

Switch `webizen-studio` to `dioxus = { features = ["router", "desktop"] }` (native rendering). This:
- Activates all `cfg(not(target_arch = "wasm32"))` branches
- Gives the Studio direct in-process access to `qualia-core-db` and `webizen-render`
- Eliminates the IPC boundary for most calls (Tauri commands become optional, not mandatory)
- Enables direct GPU surface wiring
- Removes the WASM build step entirely
- Makes `q_hash()` available — semantic hash dispatch works

**Migration impact:** The `#[cfg(target_arch = "wasm32")]` branches (wasm-bindgen, web-sys, JS interop) become dead code and can be removed. The `tauri_invoke()` pattern is replaced by direct function calls. The 330+ IPC commands remain available for the **web-server path** (see §1 below) but are no longer the primary UI→backend channel.

---

## 1. Target Architecture — Native Desktop + Local Web Server

The desktop app should serve two surfaces simultaneously:

### 1.1 Native Desktop Window (primary)
- **Renderer:** Dioxus `desktop` (native, no webview, no WASM)
- **Backend:** Direct in-process calls to `qualia-core-db`, `webizen-render`, `qualia-client-core`
- **GPU:** Direct `wgpu::Surface` from the Dioxus window's HWND — no IPC, no PNG round-trip
- **Build:** `cargo build --release -p webizen-desktop` — single step, no `dx build`

### 1.2 Local Web Server (secondary, for browser access)
The desktop app should ALSO run a local HTTP server so users can access functionality from any browser on the machine (or LAN). This is NOT the Tauri webview — it's a separate Axum server.

| Port | Default | Purpose | Current Status |
|------|---------|---------|----------------|
| **8080** | ✅ User-configurable | Settings portal + REST API + SSE telemetry + companion WS + Studio web UI | ✅ Exists (`settings_server.rs`) — needs Studio UI added |
| **4242** | ✅ Auto-finds open port | Graph daemon (SPARQL `/query`, `/health`, chat relay, WebTorrent) | ✅ Exists (`qualia_core_db::daemon`) |
| **4567** | ✅ Auto-finds open port | QApp protocol asset server (`qualia://` content) | ✅ Exists (`qapps_protocol.rs`) |
| **9001** | Planned | External WASM telemetry ingestion | ❌ Not built — needs new endpoint |
| **LAN export** | Dynamic | Static file export for companion devices | ✅ Exists (`ensure_lan_export_server`) |

### 1.3 Port 4242 — External WASM Telemetry Ingestion

The graph daemon on 4242 should additionally support receiving **external WASM telemetry** for local processing. This means:
- External devices (phones, other machines, browser tabs) can POST telemetry data to `localhost:4242/telemetry/ingest`
- The desktop app processes it locally (GPU visualisation, graph storage, anomaly detection)
- This is the "local processing hub" pattern — the desktop is the sovereign compute node

### 1.4 Port 8080 — Web-Accessible Studio

The settings server on 8080 should serve a **web-accessible version of the Studio UI** so users can:
- Open `http://localhost:8080/` in any browser to get the full Studio interface
- This would be the WASM build of the Studio (compiled once, served statically)
- The native desktop window and the web browser window share the same backend
- This is how the desktop app provides "additional functionality via the local browser"

**Architecture diagram:**

```
┌─────────────────────────────────────────────────────────────────────┐
│  webizen-desktop (native x64 binary)                                │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Native Desktop Window (Dioxus desktop renderer)             │   │
│  │  ├─ Direct wgpu::Surface (no IPC, no PNG round-trip)        │   │
│  │  ├─ Direct calls to qualia-core-db / webizen-render         │   │
│  │  ├─ 14 routes, 350+ components, 40+ WellFair panels        │   │
│  │  └─ q_hash() available — semantic dispatch works            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Local Web Server (Axum on :8080, user-configurable)         │   │
│  │  ├─ /             → Studio WASM (web-accessible UI)          │   │
│  │  ├─ /design-studio.html → Qualia Portal WASM                │   │
│  │  ├─ /api/*        → REST endpoints (jobs, telemetry, etc.)  │   │
│  │  ├─ /telemetry    → SSE stream                              │   │
│  │  ├─ /mobile/stream → Companion WS (QR pairing)              │   │
│  │  └─ /manifest, /settings, etc.                              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Graph Daemon (Axum on :4242, auto-port-discovery)           │   │
│  │  ├─ /health       → Daemon health check                      │   │
│  │  ├─ /query        → SPARQL-style graph queries               │   │
│  │  ├─ /chat/*       → Chat relay (publish/pull)                │   │
│  │  ├─ /torrent/*    → WebTorrent seeding                       │   │
│  │  ├─ /telemetry/ingest → External WASM telemetry (NEW)        │   │
│  │  └─ SPARQL endpoints config                                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  QApp Protocol Server (tiny_http on :4567, auto-port)        │   │
│  │  └─ qualia:// protocol handler — serves installed QApps     │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  Background Services:                                               │
│  ├─ Hardware telemetry (sysinfo, 2s poll)                          │
│  ├─ Render preview daemon (750ms poll)                             │
│  ├─ Ambient telemetry bridge (30 FPS)                              │
│  ├─ Medication reminder poller                                     │
│  ├─ Job scheduler (ontology import, seed, reload)                  │
│  └─ WebizenHostApi (vault + policy + Ed25519)                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Protocol Handlers

| Scheme | Purpose | Status |
|--------|---------|--------|
| `qualia://` | Serves installed QApp files from local QApps directory | ✅ Ready |
| `webizen://` | Internal data protocol for renderer/anatomy assets | ✅ Ready |

### `webizen://` Protocol Routes

| Route | Returns | Status |
|-------|---------|--------|
| `/diffusion/frame/{slot}` | Raw RGBA diffusion frame bytes | ✅ Working |
| `/render/preview.png` | PNG snapshot from render loop | ✅ Working |
| `/anatomy/body.png` | PNG of rendered anatomy body | ✅ Working |
| `/anatomy/body.json` | JSON organ percepts + organ keys | ✅ Working |
| `/anatomy/10d/{model}/{organ_key}` | .10d container for individual organs | ✅ Working |

---

## 3. System Tray & Window Management

| Submenu | Items | Status |
|---------|-------|--------|
| Sanctuary | Lock / Unlock / Vault Status | ✅ Wired |
| Daemon | Status / Restart / Stop | ✅ Wired |
| Health | Due Med Reminders / Quick Backup / Diagnostics | ✅ Wired |
| Sync | Sync with Relay / View Sync Inbox | ✅ Wired |
| Help | About / Check for Updates / View Logs / Open Settings Portal | ✅ Wired |
| Top-level | Open Studio / Settings / Toggle Ambient Viz / Revoke Sessions / Quit | ✅ Wired |

**⚠️ Note:** The auto-updater endpoint uses a placeholder public key. Must be replaced before any release.

---

## 4. Background Services

| Service | Module | Status |
|---------|--------|--------|
| Local daemon | `qualia_core_db::daemon` (port 4242+) | ✅ Ready |
| Settings + companion server | `settings_server.rs` (port 8080) | ✅ Ready |
| Hardware telemetry loop | `main.rs` (sysinfo, 2s) | ✅ Ready |
| Render preview daemon | `main.rs` (750ms poll) | ✅ Ready |
| Ambient telemetry loop | `main.rs` (30 FPS) | ✅ Ready |
| Med reminder poller | `med_reminder_notifier.rs` | ✅ Ready |
| QApp loopback server | `qualia_client_core::api` (port 4567+) | ✅ Ready |
| Job scheduler | `qualia_client_core::local_job_scheduler` | ✅ Ready |
| WebizenHostApi | `main.rs` (vault + policy + Ed25519) | ✅ Ready |

---

## 5. Tauri Command Catalogue (330+ commands)

### 5.1 Semantic Engine & Knowledge Graph
`execute_sparql_query`, `fetch_domain_ontology`, `validate_shacl_shape`, `evaluate_logic_rules`, `compute_context_hash`, `apply_semantic_handshake`, `save_qlink`, `submit_omnibox_query` — ✅ Ready

### 5.2 QApp Ecosystem
`list_installed_qapps`, `generate_qapp_credential`, `verify_and_install_qapp`, `launch_installed_qapp`, `export_qapp_as_wasm_package`, `qapp_analyze`, `wellfair_publish_qapp_pwa` — ✅ Ready

### 5.3 WellFair Health & Personal Data (60+ commands)
Health records, companion device, policy & consent, conditions & allergies, medication, sanctuary vault, life events & welfare, finance/ledger, projects & work, credentials, agency & delegation, sync & backup, assessments, anatomy/body, physiological state, dead man's switch / incapacity, transparency / disclosure, guardianship, clinical, conduct tracking, decoy retention, wellbeing, live share, diet & sleep, emergency, library/document, owner envelope, accessibility — ✅ Ready

### 5.4 Chora (Spatial Worlds)
`chora_list_worlds`, `chora_get_world`, `chora_save_world`, `chora_delete_world`, `chora_seed_demo`, `chora_navigation`, `chora_set_temporal`, `chora_set_active_world`, `chora_query_region`, `chora_publish_asset`, `chora_pull_assets` — ✅ Ready

### 5.5 Wallet & Economy
`get_wallet_status`, `get_coin_balances`, `get_transaction_history`, `generate_bip39_seed`, `derive_wallets_from_seed`, `import_external_seed`, `get_tokens`, `add_token`, `remove_token`, `get_tax_suite`, `save_tax_suite`, `dispatch_tax_payment`, `build_send_xec`, `confirm_send_xec`, `send_ecash_token`, `fetch_wallet_portfolio`, `mint_semantic_token` — ✅ Ready

### 5.6 Identity & Social
Identity CRUD, profile management, front door system, social connection, identity verification, peer management, chat system — ✅ Ready

### 5.7 Directory & Mail
Directory service, actor management, delegation rules, domain management, purpose-built inboxes, front-door forms, mail transport — ✅ Ready

### 5.8 Cloudflare & QDP
`cf_verify_token`, `cf_list_zones`, `cf_publish_front_door`, `start_qdp_server`, `resolve_qdp_did`, `get_ns_records_for_did`, `evaluate_data_request` — ✅ Ready

### 5.9 Agreements & Governance
`list_agreements`, `agreements_for`, `create_agreement`, `save_agreement`, `set_agreement_consent`, `accept_vault_handshake`, `receive_vault_job` — ✅ Ready

### 5.10 LLM / AI Inference
`discover_models`, `download_and_vectorize`, `download_model`, `cancel_download`, `get_active_model`, `set_active_model`, `get_active_downloads`, `run_agent_inference` — ✅ Ready

### 5.11 Ingestion & Data Import
`ingest_pdf`, `ingest_literature`, `upsert_cmld_definition`, `ingest_ontology`, `ingest_image`, `ingest_image_async`, `export_to_solid`, `sync_to_solid_pod` — ✅ Ready

### 5.12 3D Rendering & Spatial Interaction
`update_render_preview`, `toggle_render_loop`, `navigate_to_node`, `select_node_at`, `collapse_wavefunction`, `collapse_wavefunction_legacy`, `set_temporal_slice`, `register_browser_capabilities`, `get_latest_diffusion_snapshot`, `reconfigure_diffusion`, `get_diffusion_frame_rgba`, `get_diffusion_ledger_health` — ✅ Ready

### 5.13 Native GPU Surface (Windows HWND)
`mount_gpu_surface`, `set_gpu_scene`, `set_gpu_camera`, `upload_gpu_mesh`, `upload_gpu_mesh_colored`, `upload_gpu_10d_mesh`, `load_gpu_10d_asset`, `unmount_gpu_surface` — ✅ Implemented, Windows-only

**⚠️ NOTE:** These commands exist but the Studio UI (being WASM) calls them via IPC. In the native architecture, these would be direct function calls.

### 5.14 10D Container Browser
`browse_10d_containers`, `inspect_10d_container`, `open_10d_file_picker` — ✅ Ready

### 5.15 Mesh & Asset Commands
`load_ccf_asset`, `list_ccf_assets`, `test_ccf_ipc_handshake`, `test_larynx_smoke`, `test_vasculature_stress`, `mesh_dialability`, `mesh::mesh_start`, `mesh::mesh_stop`, `mesh::mesh_status` — ✅ Ready

### 5.16 Computational & Scientific
`run_computational_geometry`, `get_qualia_compute_profile`, `certify_forge_physics`, `run_forge_compute_probe`, `calculate_chemistry_properties`, `calculate_framingham_risk`, `calculate_quantum_dft`, `calculate_monte_carlo_var` — ✅ Ready

### 5.17 QPU (Quantum Processing Unit)
`get_qpu_settings`, `save_qpu_settings`, `enable_qpu_feature`, `disable_qpu_feature`, `activate_advanced_capabilities`, `get_advanced_activation_status`, `get_commitment_prompt` — ✅ Ready

### 5.18 Privacy & Network
`toggle_nym_relay`, `toggle_stark_prover`, `update_solar_input`, `fetch_torrent_telemetry`, `fetch_remote_manifest`, `load_imported_accounts`, `save_imported_accounts`, `submit_record`, `probe_localhost_preview` — ✅ Ready

---

## 6. Studio UI Routes

| Route | Component | Status |
|-------|-----------|--------|
| `/` | Dashboard | ✅ |
| `/anatomy-test` | AnatomyTest | ✅ |
| `/qapps` | QApps | ✅ |
| `/browser` | WebBrowserPane | ⚠️ Conditional |
| `/settings` | SettingsPage | ✅ |
| `/about` | AboutPage | ✅ |
| `/context-studio` | ContextualWorkspace | ✅ |
| `/qapp-studio` | DynamicPage (template picker) | ✅ Fixed |
| `/qapp-studio/:app_id` | DynamicPage (edit) | ✅ |
| `/render-preview` | RenderPreview | ✅ |
| `/scene-interaction` | SceneInteraction | ✅ |
| `/nexus` | Nexus | ✅ |
| `/wellfair` | WellfairShell | ✅ Fixed (vault unlock flow) |
| `/chora` | WellfairChoraPanel | ✅ New top-level route |
| `/10d-browser` | TenDBrowser | ✅ New |
| `/gpu-viewport` | NativeGpuViewportPage | ✅ New |
| `/:..path` | DynamicPage (catch-all) | ✅ |

### WellFair Sub-Panels (35+)
Accountability, Agency/delegation, Anatomy 3D, Anatomy scores, Assessment, Audit trail, Chora, Clinical, Communications, Consent, Credentials, Decoy retention, Disclosure/duty of inquiry, Finance/ledger, Guardianship, Health records, Library/documents, Life events, Medication, Pairing, Personal info, Projects/work, QApp publishing, Receipts, Safeguards, Sanctuary vault, Scorecard, Sleep, Social book, Sync & backup, Tools, Welfare support, Wellbeing, Work board — ✅ All implemented

---

## 7. Rendering Stack

### 7.1 Render Paths

| Path | Where | How | Status |
|------|-------|-----|--------|
| **Native GPU surface** | `native_surface.rs` | Child HWND → wgpu surface → direct swapchain | ✅ Implemented, Windows-only. **Needs native Studio to wire directly** |
| PNG offscreen | `commands/render_pipeline.rs` | Scene → VolumetricRenderer → PNG → `<img>` | ✅ Working (current fallback) |
| WASM Canvas2D | `webizen-web/qualia_portal.rs` | Tensor buffer → 2D projection → fillRect | ✅ Working (web portal only) |
| WASM WebGPU (tier 2) | `webizen-web/qualia_portal.rs` | wgpu surface in browser | ⚠️ Compile-verified only |

### 7.2 webizen-render (shared crate)
- PGA Motor transforms (64-byte) — ✅
- WGSL shaders (7 files: projector, epistemic, ambient, screen) — ✅
- wgpu 29 renderer: offscreen + surface modes — ✅
- VolumetricRenderer SDK (desktop facade) — ✅
- RenderScene contract — ✅

---

## 8. Companion & Phone Support

| Feature | Status |
|---------|--------|
| LAN WebSocket companion gateway | ✅ |
| QR code generation (SVG) | ✅ |
| Ed25519 pairing authentication | ✅ |
| Samsung Health folder import | ✅ |
| Companion health bundle ingest | ✅ |
| Live section sharing | ✅ |
| LAN export server | ✅ |
| PWA scaffold generation | ✅ (scaffold only — secure origin delivery not solved) |

---

## 9. WASM Builds

| Crate | Purpose | Status |
|-------|---------|--------|
| `webizen-web` | QualiaPortal — browser surface for the engine | ✅ Builds |
| `webizen-lite-wasm` | Minimal MCP JSON-RPC WASM bridge | ✅ Builds |
| `webizen-studio` | Dioxus frontend → WASM for Tauri webview | ✅ Builds **but should be native** |

---

## 10. Readiness Assessment

### ✅ Ready (Core)
- Tauri shell + window + tray
- 330+ Tauri IPC commands
- Settings portal (:8080)
- Local daemon (:4242)
- WebizenHostApi (vault + policy + Ed25519)
- Companion pairing
- Hardware telemetry
- Render preview (PNG path)
- Job scheduler

### ⚠️ Needs Architecture Fix
| Issue | Impact | Fix |
|-------|--------|-----|
| **Studio is WASM, not native** | All `cfg(not(wasm32))` branches dead; IPC tax on every call; `q_hash()` unavailable; GPU only via IPC | Switch to `dioxus = { features = ["desktop"] }` |
| **Native GPU surface not wired to UI** | Backend exists but UI can't call it directly | Native Studio gets direct `wgpu::Surface` access |
| **Build is 3-step (dx build → copy dist → cargo build)** | Fragile, ships stale assets | Native build is single `cargo build` |
| **External WASM telemetry ingestion** | No endpoint on 4242 for external telemetry | Add `/telemetry/ingest` to graph daemon |
| **Web-accessible Studio on :8080** | Settings portal exists but doesn't serve Studio UI | Serve WASM Studio build from Axum on :8080 |
| **Updater signing key** | Placeholder key | Replace before release |
| **PWA secure-origin delivery** | LAN HTTP not secure origin | WebRTC data channel (designed, not built) |

### ❌ Not Present
| Feature | Status |
|---------|--------|
| `webizen-browser` (separate repo) | Separate repository, not incorporated |
| Flutter WebView (phone console) | Designed, not implemented |
| STUN/TURN relay | Excluded by design (LAN-only) |
| wasm64 target | Decided, not built |

---

## 11. Priority Work Order

1. **Migrate Studio from WASM to native Dioxus desktop** — switch `features = ["web"]` to `features = ["desktop"]`, remove wasm-bindgen/web-sys deps, activate `cfg(not(wasm32))` branches, replace `tauri_invoke()` with direct calls
2. **Wire native GPU surface directly** — `NativeGpuViewport` component calls `wgpu::Surface` creation directly (no IPC)
3. **Add external WASM telemetry ingestion** — new endpoint on port 4242 for receiving telemetry from external devices
4. **Serve Studio WASM from :8080** — keep the WASM build for the web-server path (browser access), serve it from the existing Axum settings server
5. **Make port 8080 user-configurable** — settings UI for changing the web-server port
6. **Replace updater placeholder key** — required for distribution
7. **Finish PWA secure-origin delivery** — WebRTC data channel for LAN phone install
