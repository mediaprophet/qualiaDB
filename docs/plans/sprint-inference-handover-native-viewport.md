# Sprint Plan — Inference Handover, Tray Control, Native 10D Viewport

**Date:** 2026-06-17  
**Branch:** `0.0.17-dev`  
**Status:** `PLANNED` — implementation deferred to next sprint  
**Release target:** `0.0.17` docs kit + tray/`:8080` — **not** `webizen-studio`  
**Companion:** [`design-studio-wiring.md`](../design-studio-wiring.md), [`wasm-viewport-migration-plan.md`](wasm-viewport-migration-plan.md), [`phone-console.md`](phone-console.md)

---

## Executive summary

Three problems block a coherent “browser → native” product-design loop:

1. **Duplicate LLM residency** — `online-llm-demo.html` (WASM Gemma) and native `LocalLlmAgent` can both hold ~1 GB+ weights on the same machine with no coordination.
2. **Wrong control surface** — docs and wiring still reference **Flutter LLM Hub**; that shell is **deprecated**. Active desktop control is **Tauri system tray** → **Webizen Studio** + **settings portal `:8080`**.
3. **WASM-first 10D viewer** — phenomenal Qualia Portal (T2 WebGPU, tensor SOA, pick/navigate) ships on **GitHub Pages** (`spatial.html`, `docs/pkg/qualia/`). Native x64 / Metal / DirectML still uses **legacy `webizen-render` offscreen 2D** (`screen.wgsl`) — **PR-C10 desktop parity is open**.

This sprint plan sequences work without assuming Flutter or a finished native viewport.

---

## Deprecation note (locked)

| Surface | Status | LLM / viewport role |
|---------|--------|---------------------|
| `crates/qualia-flutter/` | **Deprecated** — no new FRB or Hub features | Do not add `recommend_assets_for_design()` FRB |
| `crates/webizen-desktop/` (Tauri) | **Active host** — tray, daemon, `:8080` portal | Tray telemetry, inference lease API, job enqueue |
| `crates/webizen-studio/` (Dioxus) | **Out of release scope** | Do not block 0.0.17 on studio wiring |
| `docs/design-studio.html` | **Active tech demo** | Full Qualia Portal WASM (same as `spatial.html`) |
| `docs/` WASM portal | **Active phenomenal path** | T2 viewport; native x64/Metal parity still PR-C10 |

**Tray menu today** (`webizen-desktop/src/main.rs`):

- Open Webizen Studio
- Settings / View Logs
- **Open Settings Portal** (`http://127.0.0.1:8080/`)
- Revoke Sessions
- Daemon Status
- **Toggle Ambient Visualization** (telemetry → `hardware-telemetry` event)
- Quit

**Gap:** no tray items or portal API for *inference lease*, *model load/unload*, or *handover from browser*.

---

## Problem 1 — Inference Handover Protocol (IHP)

### Goal

**One active text-LLM lease per machine.** Switching from browser WASM to native must:

1. Release browser WebGPU weight buffers.
2. Claim native U0 (`ComputeUniverse::LlmInference`) via existing `TaskOrchestrator` + `resident_model`.
3. Optionally resume session (prompt + partial tokens + `AgentIntent`), not re-download GGUF.

### What already exists (reuse, do not rewrite)

| Primitive | Location | Reuse for IHP |
|-----------|----------|---------------|
| Single resident GGUF | `orchestrator.rs` `load_model` / `evict_model` | Native claim after browser release |
| Process-wide mmap | `resident_model.rs` | Same catalog id → mmap from disk |
| Backend preference | `inference_backend.rs` (Local / Remote / Hybrid) | Policy layer |
| Phase-8 bifurcation | `llm_agent.rs` + `compute_universe.rs` | Resume decode on native |
| Orchestration gate | `orchestrator.rs` `validate_intent` / `validate_output` | Handover must carry ratified intent |
| Suspended queue pattern | `crdt.rs` `SuspendedTransactionQueue` | Queue handover if thermal Critical |

### What is missing

- Cross-process **lease registry** (browser tab ↔ native process).
- WASM export `release_inference_lease()`.
- Portal routes under `:8080`.
- Tray + portal UI for status and “Continue on desktop”.
- `LocalJobKind::LlmCatalogImport` in `local_job_scheduler.rs` (ontology jobs exist; LLM import does not).

### IHP phases (sprint scope)

#### Phase 0 — Status probe (smallest shippable)

**API**

```
GET /api/inference/status
→ {
     lease_holder: "none" | "browser" | "native",
     model_id: string | null,
     lifecycle_state: "discovered" | "active" | "scrubbing" | ...,
     ram_mb: number,
     vram_pressure: number,
     inference_backend: "local" | "remote" | "hybrid"
   }
```

**Browser**

- `online-llm-demo.html`, `design-studio.html`: poll `:8080/api/inference/status` when portal reachable.
- If `lease_holder === "native"` and `lifecycle_state === "active"`: show banner *“Native LLM active — pause browser model to free RAM”*; disable `infer_wasm` start.

**Tray**

- New menu item: **Inference Status** (submenu or native notification).
- Updates `daemon_status` label from `GET /api/inference/status` (same channel as existing tray refresh loop in `main.rs`).

**Exit criteria:** No double-load policy enforced in UI; no new Rust inference code beyond status aggregation.

#### Phase 1 — Explicit handover

**API**

```
POST /api/inference/handover/request
  { source: "browser", catalog_model_id, session_id, partial_token_ids[] }
→ { granted: bool, handover_token, native_model_id }

POST /api/inference/handover/commit
  { handover_token, agent_intent_frame, design?: qualia.design }
→ { resumed: bool, lifecycle_state }

POST /api/inference/release
  { holder: "browser" | "native" }
```

**Native path**

- `qualia-client-core`: new `inference_lease.rs` (thin; calls `model_lifecycle`, `unload_active_model`).
- Handover commit → `load_model` if needed → inject prompt into `AgentIntent`.

**Browser path**

- Before commit: call WASM `release_inference_lease()` (free GPU buffers).
- Design Studio: **“Continue on desktop”** button → handover request when portal live.

**Tray**

- **Load / Unload Model** submenu wired to `qualia resources import llm` + `unload_active_model` via Tauri command (not Flutter).

**Exit criteria:** Measured: browser Gemma tab closed or paused; native `ModelLifecycle::Active` within one handover flow.

#### Phase 2 — Session continuity (optional same sprint if Phase 1 slips)

- Bounded `[u32; N]` partial token buffer in handover payload.
- KV cache shard transfer **only** if same `catalog_model_id` + quantization (else restart decode from tokens).
- Sentinel: handover only at token boundary unless user confirms mid-stream abort.

#### Phase 3 — Multi-device (future)

- ICP relay `INFERENCE_DELEGATE` (phone → desktop claims U0). See [`phone-console.md`](phone-console.md) §LLM.

### Pipeline requirements (normative)

```
Browser infer start
  → [optional] GET /api/inference/status — abort if native holds lease

Handover request
  → validate_intent (browser-held AgentIntent Quins)
  → native unload conflicting resident if different model_id
  → grant handover_token

Browser release
  → WASM release_inference_lease()
  → POST handover/commit

Native resume
  → load_model (mmap catalog GGUF)
  → Phase-8 decode loop (existing infer_local_model)
  → validate_output on commit paths
```

**Thermal:** `ThermalGovernor::Critical` → deny handover; return queued state (mirror `SuspendedTransactionQueue` semantics).

**VRAM:** `VramLedger` Reserve mode may throttle U2 viewport before denying U0 claim — document in tray status string.

---

## Problem 2 — Control surface (tray + portal, not Flutter)

### Target UX map

| User action | Surface |
|-------------|---------|
| See daemon / inference / RAM | Tray **Inference Status** + `:8080` dashboard |
| Install ontology | Design Studio → `POST /api/assets/enqueue` (exists) |
| Install LLM | Tray **Manage Models** or portal **Assets** page → `LocalJobKind::LlmCatalogImport` (new) |
| Load / activate GGUF | Tray or Webizen Studio `llm_harness` → `set_active_model` / `model_lifecycle` |
| Unload GGUF | Tray **Unload Model** → `unload_active_model` |
| Ambient viz during inference | Existing **Toggle Ambient Visualization** + `telemetry_hooks` `INFERENCE_COUNTER` |
| Full chat + graph | Webizen Studio (not Flutter) |

### Tray work items

| ID | Item | Files |
|----|------|-------|
| T-1 | `Inference Status` menu item + dynamic label | `webizen-desktop/src/main.rs` |
| T-2 | `Manage Models` → open `:8080/assets.html` or studio route | tray + `settings_server.rs` static |
| T-3 | `Unload Model` → Tauri command → `qualia_client_core::model_lifecycle::unload_active_model` | `commands/mod.rs` |
| T-4 | Emit `inference-lease-changed` event for studio ambient sync | `telemetry_bridge.rs` |
| T-5 | File logging for inference transitions (match existing telemetry file hook) | `telemetry_hooks.rs` |

### Portal work items (`:8080`)

| ID | Item | Notes |
|----|------|-------|
| P-1 | `GET /api/inference/status` | Aggregate orchestrator + `list_installed_model_ids` |
| P-2 | IHP Phase 1 routes | See above |
| P-3 | `POST /api/assets/enqueue` extend `{ kind: "llm_catalog_import", llm_id }` | Mirror ontology pattern in `local_job_scheduler.rs` |
| P-4 | Assets HTML page: installed vs recommended, enqueue buttons | Link from Design Studio |
| P-5 | Remove all “Flutter LLM Hub” copy | `design-studio-wiring.md`, portal HTML |

### Design Studio + docs kit (0.0.17 — in progress)

| ID | Item | Status |
|----|------|--------|
| D-0 | `design-studio.html` full `QualiaPortal` | ✅ |
| D-1 | `sync-portal-design-kit.ps1` → `:8080` static | ✅ script |
| D-2 | `pages.yml` + `package-qualia-wasm.ps1` version `0.0.17` | ✅ |
| D-3 | IHP Phase 0 status API + tray items | Next sprint |

Webizen Studio (`S-*` items) deferred until a post-0.0.17 studio release.

---

## Problem 3 — Native 10D viewport gap (large; parallel track)

### Current state (honest)

| Path | 10D / phenomenal | Status |
|------|------------------|--------|
| **Pages WASM** (`spatial.html`, `qualia_portal.rs`, `portal_gpu.rs`) | T2 WebGPU, tensor SOA, pick, collapse, U3 audio | **Validating** (Track C ✅ on Pages) |
| **Design Studio** (`design-studio.html`) | JS canvas **fallback** (not full portal) | Demo only |
| **Native desktop** (`webizen-render`, `webizen-studio/render/native.rs`) | Offscreen `WgpuRenderer`, **2D `screen.wgsl`**, PNG readback to webview | **Not** phenomenal T2 |
| **PR-C10** | Desktop Dioxus parity with Pages shaders | **Open** per [`wasm-viewport-migration-plan.md`](wasm-viewport-migration-plan.md) |

**Implication:** A user who handovers from browser to native **loses** the full 10D Qualia Portal experience until PR-C10 lands. Design Studio native preview stays JS/tensor-fallback or portal-in-webview (WASM embed), not Metal/DirectML native shaders.

### Native viewport sprint track (PR-C10 dependency chain)

```
PR-C10a  Migrate viewport WGSL from qualia-core-db/shaders/viewport → webizen-render re-export
PR-C10b  Shared GpuContext (gpu_context::shared_gpu) in desktop host — not per-frame new_offscreen
PR-C10c  TensorBufferHeader SOA feed from design_encode / daemon graph (same contract as Pages)
PR-C10d  Embed qualia_bg.wasm in studio webview OR native wgpu surface in Tauri window
PR-C10e  design-studio :8080 opens spatial parity tab (not separate JS canvas long-term)
```

**Recommended interim:** Design Studio on desktop embeds **same WASM portal** as `spatial.html` in an iframe/webview panel until PR-C10c native path is proven — avoids building a third renderer.

### Design → viewport data contract (unchanged)

- `qualia.design` JSON → `design_encode.rs` → `Tensor10D` SOA + NQuins.
- WASM: `design_encode_wasm(json)`.
- Native commit path still requires `validate_output` provenance Quins.

---

## Sprint PR stack (suggested order)

| PR | Scope | Depends | Exit |
|----|-------|---------|------|
| **PR-H0** | `GET /api/inference/status` + browser/tray banners | — | Status visible; browser defers if native active |
| **PR-H1** | `inference_lease.rs` + handover request/commit/release | H0 | One machine, one LLM lease enforced |
| **PR-H2** | `LocalJobKind::LlmCatalogImport` + portal enqueue | `local_job_scheduler` | Design Studio can queue LLM download |
| **PR-T1** | Tray: Inference Status, Unload Model, Manage Models | H0 | No Flutter references in UX |
| **PR-T2** | Studio `model_lifecycle` + `llm_harness` wired to real API | T1 | Activate/unload from studio |
| **PR-D1** | Design Studio “Continue on desktop” + handover button | H1 | End-to-end browser→native session |
| **PR-C10** | Native viewport parity (separate track; may span sprints) | wasm-viewport plan | `spatial.html` equivalent in desktop |

**Explicitly out of this sprint**

- Flutter FRB exports.
- KV cache cross-process transfer (Phase 2).
- Phone `INFERENCE_DELEGATE` (Phase 3).
- Full native Metal shader path without WASM embed fallback.

---

## Design Studio checklist (updated)

- [x] `design_encode.rs` + WASM export
- [x] Design Studio UI (docs + `:8080`)
- [x] SPARQL proxy on `:8080`
- [x] Asset recommend API + client-side fallback
- [x] Ontology enqueue via `/api/assets/enqueue`
- [ ] LLM job enqueue via `/api/assets/enqueue` + `local_job_scheduler` (**tray/portal**, not Flutter)
- [ ] IHP Phase 0 status probe in docs pages
- [ ] IHP Phase 1 handover from Design Studio
- [ ] Online LLM → `qualia.design` emit in `online-llm-demo.html`
- [ ] `spatial.html` import saved jobs from `localStorage`
- [ ] Native 10D preview: WASM embed interim **or** PR-C10 completion
- [ ] Studio `model_lifecycle` UI wired to real orchestrator state

---

## Testing plan

| Test | Command / surface |
|------|-------------------|
| Asset recommendations (Rust) | `cargo test -p qualia-client-core asset_recommendations` |
| Model lifecycle | `cargo test -p qualia-client-core model_lifecycle` |
| Orchestrator resident guard | `cargo test -p qualia-core-db orchestrator` |
| IHP integration (future) | Script: start native model → open `online-llm-demo.html` → assert WASM infer blocked |
| Handover E2E (future) | Browser partial decode → handover commit → native completes same prompt |
| Tray smoke | Manual: Unload Model → `ModelLifecycle::Discovered` + tray label update |

---

## Open decisions (resolve at sprint kickoff)

1. **Native 10D interim:** iframe WASM portal in studio vs wait for PR-C10?
2. **Handover default:** always unload browser weights on native claim, or user opt-in?
3. **LLM install UX:** tray-only vs portal Assets page vs both?
4. **Remote/Hybrid during handover:** does `inference_backend.json` Remote block native claim?

---

## Session handoff note

Flutter LLM Hub is **out of scope**. All new inference UX goes through **system tray** (`webizen-desktop`), **settings portal `:8080`**, and **Webizen Studio** components. The 10D phenomenal viewer is **WASM-complete, native-incomplete** — plan for WASM embed or PR-C10 before promising native Metal/DirectML design preview parity.