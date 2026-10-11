# VB-00 — VibeScript Scripting Behaviour & Gap Baseline
_Date: 2026-10-11 | Branch: `0.0.40.8-dev` | Status: Complete Baseline_

This document delivers the normative evidence baseline for Packet `VB-00` under the POET Lower-Cost Agent Execution Playbook §19.1 and Implementation Specification ST-N. It classifies every required application scripting behaviour from Storytold applications (most notably ArtCraft) and Poet browser components, establishes test baseline evidence, identifies language/runtime/scheduler gaps, and prepares interface proposals for Review Gate 1 (G1).

---

## 1. Verified Test Baselines

All three core Vibe test suites were executed from the `C:\github\qualiaDB` root (`--locked --offline`):

| Test Suite | Command | Result | Duration | Notes |
|---|---|---|---|---|
| **Conformance** | `cargo test -p vibe --test conformance` | **184 passed**, 0 failed | 27.79s | Covers §12 normative examples, §13 negative fixtures, checked arithmetic, bytecode, cosmic/econ/crypto/emf domains |
| **Features** | `cargo test -p vibe --test vibe_features` | **9 passed**, 0 failed | 1.34s | Covers pipelines, quantity literals, physical units, locales, modal logic blocks, embedded graph query |
| **Catalog Surface** | `cargo test -p vibe --test vibe_catalog_surface` | **24 passed**, 0 failed | 10.40s | Covers `using` leases, catalog method dispatch, present blocks, color literals, quantity units, locale CBOR hashes |
| **Total Baseline** | | **217 passed**, 0 failed | | Clean baseline with zero regressions |

---

## 2. Core Language & Runtime Seam Analysis

### 2.1 Await & Async Execution (`eval/expr.rs:305`)
- **Current Behavior**:
  ```rust
  ExprKind::Await(e) => self.eval_expr(e, env),
  ```
  In `eval/expr.rs`, `await expr` immediately and synchronously evaluates `expr`. In REPL preflight, `= await 42` returns `42` in a Pure cell without error.
- **Identified Gap**:
  Lacks a cooperative asynchronous state machine. When a capability returns an asynchronous job handle (`Pending(job_id)`), the evaluator cannot suspend the task, preserve gas/budgets across frames, yield the execution turn to the host UI thread, or resume upon a completion event.
- **Destination Seam**:
  `VB-02` will establish `vibe::async_runtime` with explicit `Ready(Value)`, `Pending(JobId)`, and `Cancelled` task frames.

### 2.2 Yield & Event Stream Scheduling (`eval/stmt.rs:163-170`)
- **Current Behavior**:
  ```rust
  Stmt::Yield { value, .. } => {
      let v = if let Some(e) = value {
          self.eval_expr(e, env)?
      } else {
          Value::Null
      };
      Ok(Flow::Next(v))
  }
  ```
  `Stmt::Yield` evaluates the inner expression and immediately continues to the next statement via `Flow::Next`.
- **Identified Gap**:
  It does not act as a generator or event-stream emission point. Streams and event queues require deterministic buffered scheduling with backpressure.
- **Destination Seam**: `VB-02` stream runtime.

### 2.3 Module Graph vs Execution Imports (`eval/mod.rs:71-100`)
- **Current Behavior**:
  `populate_import_aliases` validates import paths strictly against `VIBE_0_1_NAMESPACES`:
  `["math", "graph", "aura", "pulse", "time", "quin", "rdf", "capability", "deontic"]`.
  Any other namespace returns `E100: unknown namespace '<ns>'`.
- **Identified Gap**:
  While `module_system.rs` provides `ModuleId`, `ModuleEntry`, `ModuleCatalog`, and `ModuleResolver`, the evaluator does not execute imports from `.vibe` files on disk, packaged HCF modules, or inter-module dependency graphs.
- **Destination Seam**: `VB-03` executable module imports and standard library resolution.

### 2.4 WASM Value Serialization & JS Interchange (`vibe-wasm/src/lib.rs:86-93`)
- **Current Behavior**:
  ```rust
  // For complex domain values, serialise as a tagged record.
  _ => {
      let o = Object::new();
      Reflect::set(&o, &"type".into(), &JsValue::from_str(&format!("{:?}", v))).ok();
      Reflect::set(&o, &"repr".into(), &JsValue::from_str(&format!("{:?}", v))).ok();
      o.into()
  }
  ```
- **Identified Gap**:
  `format!("{:?}", v)` emits Rust `Debug` strings into production JS/WASM objects for complex values (such as nested records, lists, Quin references, and opaque asset/tensor handles). This violates typed production interchange invariants.
- **Destination Seam**: `VB-06` typed native/WASM host I/O and CBOR/JSON tagged interchange.

### 2.5 Poet Browser Cell Execution (`poet/wasm/src/browser/vibe_cell.rs:132-200`)
- **Current Behavior**:
  `VibeCell::evaluate` in `poet/wasm` currently performs ad-hoc string inspections (`expr.starts_with("math.sum([")`, `expr == "sentinel.slg_arena_status()"`, `expr == "triples.count()"`) instead of routing expressions through `vibe::Engine`.
- **Contrast with `vibe_ui.rs`**:
  `vibe_ui.rs` already correctly imports `vibe::{parse_program, Budget, Engine, Env, LocalHost, Program, Value}` and compiles declarative UI node trees into `VibeUiNode`.
- **Destination Seam**: `VB-05` / `VB-08` wiring `vibe_cell.rs` to the real Vibe engine and unifying UI presentation.

---

## 3. Application Scripting Inventory (ST-N3 Classification)

Below is the exhaustive classification of required scripting behaviors across the 11 Storytold applications and Poet browser environment.

| Row ID | Application / Component | Source / Trigger | Input & Result Shape | Effect Class | Current Vibe State | Missing Seam | Migration Packet | Residual Platform Reason |
|---|---|---|---|---|---|---|---|---|
| **SCR-01** | **Poet** Browser Cells | `<q-cell>` input submit / formula edit | `formula: string` → `CellValue` (number, string, NQuin) | Pure | Mock string parsing in `vibe_cell.rs` | Route through `vibe::Engine::eval_expr` with Pure budget | **VB-05** / **VB-08** | None (pure WebAssembly execution) |
| **SCR-02** | **Poet** Declarative Trays | `.vibe` UI source hot-reload | `Record` with `type: "dock_panel" \| "subtray"` → `VibeUiNode` | Pure | Fully functional in `vibe_ui.rs` via `vibe::Engine` | Integration with contextual orbs & command registry | **VB-05** | None (WASM DOM reconciler) |
| **SCR-03** | **Poet** Live Capability Invocation | Command palette / Tool Chest action | `(id: string, params: Record)` → `Receipt` or result value | External | Fully wired via `poet_host::invoke` / `LocalHost` | Strict ST-E envelope validation & receipt binding | **VB-01** | Host transport |
| **SCR-04** | **ArtCraft** Generation Tasks | Pose / Canvas Generate button | `{ prompt, pose_map, mask_ref, model_id }` → `JobHandle` | External | None (previously TypeScript React/Zustand dispatching to SQLite/Tauri) | `App.execute("artcraft::generate", ...)` returning opaque job handle | **VB-01** / **VB-08** | Native/cloud AI provider API client |
| **SCR-05** | **ArtCraft** Task Queue & Polling | Task state watcher / timer hook | `job_id: string` → `{ status: "queued"\|"running"\|"succeeded"\|"failed", progress: f64 }` | External | None (previously React polling loop) | `Job.status(job_id)` & `Job.cancel(job_id)` async capability | **VB-01** / **VB-02** | Transport polling |
| **SCR-06** | **ArtCraft** Scene State Authoring | Pose slider / camera orbit / 3D transform | `{ bone_id, rotation: [f64; 4], translation: [f64; 3] }` → `Record` | Pure | Available via `LinearAlgebra` & `Record` mutations | Declarative scene state struct + hot update hook | **VB-04** / **VB-08** | WebGL / wgpu canvas render |
| **SCR-07** | **ArtCraft** Mask & Layer Composition | Brush stroke commit / mask inverse | `{ mask_layer_id, opacity, blend_mode }` → `Record` | Hot | Available via image capability family | Hot zero-heap tile compositing bridge | **VB-01** / **VB-08** | Graphics pipeline |
| **SCR-08** | **GridCraft** Formula Recalculation | Cell commit in sheet grid | `formula: string` (e.g. `=SUM(A1:A10)*1.15`) → `Value` | Pure | Functional via Pure cell evaluation | Excel formula dialect bridge & dependency graph cycle check | **VB-04** / **VB-08** | Canvas grid virtualization |
| **SCR-09** | **WordCraft** Document Styles & Typography | Contextual menu style apply | `{ span_id, font_family, weight, size_pt }` → `Receipt` | Document Mutation | None (previously egui CommandSpec) | Command dispatch to Word engine via ST-E contract | **VB-01** / **VB-08** | HarfBuzz / font rasterizer |
| **SCR-10** | **DeckCraft** Slide Transitions & Animation | Presentation timeline scrub / play | `{ slide_id, anim_type, duration_ms }` → `TweenExpr` | Pure | Vibe has native `~ over` tween syntax and easing | Timeline tick binding to presentation renderer | **VB-04** / **VB-08** | Display presentation device |
| **SCR-11** | **PdfCraft** Page Automation & Search | Automation script execution | `{ pdf_ref, action: "rotate"\|"extract_text"\|"redact", pages: [u32] }` → `Receipt` | Cold | Automation table exists in engine; no script bridge | Expose bounded automation tools via `App.execute` | **VB-01** / **VB-08** | PDF rasterizer engine |
| **SCR-12** | **VectorCraft** Path Operations & Shapes | Path booleans (union, subtract, intersect) | `{ path_a: PathRef, path_b: PathRef, op: "union" }` → `PathRef` | Cold | `kurbo` path ops exist in engine; needs script handle | Opaque `GeometryRef` handle passing in Vibe `Value` | **VB-06** / **VB-08** | Vector rasterizer |
| **SCR-13** | **PhotoCraft** Adjustment Layer Parameters | Curves / Levels / Color balance sliders | `{ layer_id, adjustment: "curves", points: [[f64; 2]] }` → `Receipt` | Hot | Tiled engine exists; needs parameter script bindings | Hot buffer parameter updates | **VB-01** / **VB-08** | GPU compositing surface |
| **SCR-14** | **LightCraft** RAW Develop Presets | Apply preset to photo asset | `{ asset_id, exposure, contrast, highlights, shadows }` → `Receipt` | Cold | Pipeline exists in engine; needs script macro | Parameter bundle application via `App.execute` | **VB-01** / **VB-08** | RAW decode pipeline |
| **SCR-15** | **FilmCraft** Rational Time & Clip Splitting | Timeline playhead split command | `{ track_id, ticks: u64 }` → `Receipt` | Document Mutation | Exact rational time exists in engine (`254,016,000,000` ticks/s) | Exact decimal-u64 representation in Vibe `Value` | **VB-04** / **VB-06** | Real-time audio/video output |
| **SCR-16** | **SoundCraft** Track Routing & Metering | Mixer fader change / bus route | `{ track_id, gain_db, pan, mute, solo }` → `Receipt` | Hot | Audio graph exists; callback thread zero-heap | Hot control parameters without allocating in callback | **VB-01** / **VB-08** | Realtime audio device callback |
| **SCR-17** | **CADCraft** Interactive Snapping & Constraints | Entity move with geometric constraint | `{ entity_id, new_pt: [f64; 2], snap_mode }` → `Record` | Cold | f64 constraint solver exists in engine | Diagnostic report for underconstrained / conflicting geometry | **VB-01** / **VB-08** | Viewport rendering |

---

## 4. Proposed v1 Host Capability Extensions for Gate 1 (G1)

In accordance with playbook §19.2 and §18.5, the following typed capability family is proposed for review at Review Gate 1. These are **not** ad-hoc additions; they map directly to ST-E command and lifecycle contracts:

### 4.1 Family `App.*`
- `App.commands(session_id: string) -> List<CommandDescriptor>`: Enumerate valid command descriptors for the session.
- `App.availability(session_id: string, command_id: string) -> Availability`: Check if command is available given current target/selection.
- `App.inspect(session_id: string, target_id: string) -> Record`: Retrieve inspect snapshot of an entity or selection.
- `App.execute(request: CommandRequest) -> CommandReceipt`: Execute an authorized command through the unified dispatcher.

### 4.2 Family `Job.*`
- `Job.status(job_id: string) -> JobDescriptor`: Poll or inspect the state of an asynchronous/background domain job.
- `Job.cancel(job_id: string) -> Receipt`: Cancel an active job and release staged resources.

### 4.3 Family `UI.*`
- `UI.describe(context_id: string) -> InteractionContext`: Inspect current pointer, focus, and selection context.
- `UI.propose(proposal: Record) -> Record`: Generate an isolated agent presentation proposal without automatic execution.

---

## 5. Summary of JavaScript/TypeScript Deprecations & Bridges

1. **ArtCraft Frontend (React / Zustand / Tauri)**:
   - Full migration of application state and task orchestration to `.vibe` scripts feeding shared Dioxus UI.
   - Complete removal of nonconsented analytics and telemetry libraries.
   - Prohibition of browser-session cookie scraping; all model provider APIs require explicit credential handles.
2. **Platform Bridges (Preserved as Minimal Transport Glue Only)**:
   - `wasm-bindgen` loader & initialization glue in browser entry points.
   - Dedicated Web Worker message pump & transferable `ArrayBuffer` transport.
   - Web Audio API worklet audio-output callback bridge.
   - Canvas / WebGL2 / WebGPU viewport resize and raw pointer/touch event forwarding.
   - *Invariant*: No business logic, validation rules, state persistence, or command decisions reside inside these platform shims.

---

## 6. Acceptance & Gate Closeout Recommendation

- **Acceptance Status**: Complete for `VB-00`.
- **Pre-G1 Requirement Satisfied**: The required application scripting behaviors across all 11 applications and Poet are inventoried, test baselines are verified (217/217 passed), runtime gaps are pinpointed with code citations, and the `App`/`Job`/`UI` capability family is formally defined for Review Gate 1.
- **Recommended Next Packets**:
  - `ST-01`: Local app fork initialization for the first app subunit (e.g. `ST-01-GRID` or `ST-01-WORD`) under `C:\github\poet-<slug>`.
  - `ST-02`: Creation of the `qualia-app-contract` crate with canonical ST-E DTOs and schema validators for G1 review.
