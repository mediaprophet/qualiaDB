# Vision → 10D Browser Excellence — Progress Log

**Programme:** `vision-10d-browser-excellence-programme-2026.md`  
**Branch:** `0.0.25`

---

## 2026-07-17 — Sprint 5: D4 + F3 + F4 + library migration honesty

**Status:** done (code); MIG-V* **open** (migration not executed this wave)

### Library placement (principal question)

**Not** under `qualia-core-db::specialized_libs` today. Algorithms: **`qualia-vision`**.
Seal/paint/geometry: **core-db** (`compile_10d`, spectral, CG). Host: **client-core**.

Programme §9-B records **MIG-V1…V4** — inventory → promote pure kernels to
`specialized_libs::computer_vision` (or facade) → keep `qualia-vision` as edge surface.
Registry D5.13 honesty notes MIG-V2 open.

### Tracks this wave

| Track | What |
|-------|------|
| **D4** | Gs continuum seals `ProvenanceSidecar` (media digest + model hash metadata) via `compile_mesh_to_10d_vision_with_provenance` |
| **F3** | `temporal_scrub_paint` / `_vec` + Tauri `scrub_vision_10d_paint` |
| **F4** | `vision_10d_rights` barrier (Browse vs CitableRequireProvenance); load path gated |
| **MIG** | Documented only — not migrated this session |

### Measured

```
cargo test -p qualia-client-core --lib vision_10d  →  6 passed
```

### ⚑ Principal

- Confirm MIG-V2 target name (`computer_vision` vs keep dual-crate permanently).
- If permanent dual-crate: mark MIG-V2 cancelled and document as intentional ABI.

### Next

- MIG-V1 inventory (or B2 SR WGSL if principal prioritizes product over migrate)
- Studio UI for load/scrub/citable toggle

---

## 2026-07-17 — Sprint 4 wave: C3 + D3 + F2

**Status:** done

| Track | What was built |
|-------|----------------|
| **C3** | `Compile10dExtras` + `compile_mesh_to_10d_vision` / `_with_extras`: half-edge **Topology** + BVH/kd **SpatialIndex** sections on native/wasm-scientific. Gs continuum + `seal_vision_mesh_with_detections` use vision seal. |
| **D3** | σ → `sigma_to_display_rgb` + `sigma_to_center_frequency_hz` in `vision_10d_load`; volumetric `load_10d_asset` colours mesh by nearest-node σ. |
| **F2** | `load_vision_10d_bytes` / `_path` → `Vision10dLoaded` paint package; Tauri `load_vision_10d`. |

### Measured

```
cargo test -p qualia-core-db --lib vision_seal_includes_topology  → 1 passed
cargo test -p qualia-client-core --lib vision_10d  → 2 passed (browse + load paint)
cargo test -p qualia-vision --lib  → 342 passed
cargo check -p webizen-render  → ok
```

### ⚑ Principal

None. Optional Studio UI button for `load_vision_10d` (command registered).

### Next

- **F3** time scrub using node `t`  
- **F4** rights barrier on unattested vision containers  
- **B2** classical SR WGSL on shared_gpu  
- **D4** ProvenanceSidecar on Gs continuum (media digest + model hash)

---

## 2026-07-17 — Sprint 3 wave: C2 + D1 + F1

**Status:** done

| Track | What was built |
|-------|----------------|
| **C2** | `spatial/mesh_ir_quality` — weld + degenerate drop; host `maybe_decimate_mesh` via QEM (`decimate_qem`) when faces > cap; Gs continuum runs cleanup then optional decimate. |
| **D1** | `compile_mesh_to_10d_with_nodes` / `_and_provenance` + `decode_10d_nodes` in `render/compile_10d.rs`; Gs continuum seals recon marker Tensor10D; `seal_vision_mesh_with_detections` + `node_hint_to_tensor10d`. |
| **F1** | `vision_10d_browse::list_vision_10d_containers` / inspect; Tauri `browse_vision_10d`; general `browse_10d_containers` scans `vision_geometry/` and labels **Vision Reconstruction**. |

### Measured

```
cargo test -p qualia-core-db --lib mesh_with_nodes_round_trips_sigma  → 1 passed
cargo test -p qualia-vision --lib  → 342 passed
cargo test -p qualia-client-core --lib vision_10d_browse  → 1 passed
```

### Registry

- D5.13 **Present** (mesh+nodes seal + browse)
- D5.01 honesty notes quality cleanup

### ⚑ Principal

None this step. Optional: Studio UI button wired to `browse_vision_10d` (command registered).

### Next

- **C3** Topology/SpatialIndex sections on vision recon  
- **D3** portal paint/scrub σ for vision assets  
- **F2** portal WASM load vision `.10d`  
- **B2** classical SR WGSL when Cool  

---

## 2026-07-17 — Sprint 1 complete + Sprint 2 B1/D2 started

**Status:** done (Sprint 1); partial (Sprint 2)

### Sprint 1 — A1 / B0 / C1 / E1

| Track | What was built |
|-------|----------------|
| **A1** | `qualia-vision/src/gpu/` — `dispatch` (`resize_nearest_nchw_dispatch` with honest Cpu/Unavailable/degraded), `policy` (ThermalHint, VisionVramBudget). Feature `gpu` remains a non-empty gate; no second adapter. |
| **B0** | `cv/sr/{bilinear,bicubic,lanczos3}_u8` + `sr/super_resolve` unified API (`ClassicalKernel`, `SrReport`, generative=false). |
| **C1** | `spatial/mesh_ir_to_export`, `compile_10d_handoff` (`GeometryFor10d`, `pack_geometry_export_for_10d`, `detections_to_node_hints`). Client `vision_pipeline` uses public export path. Host still seals via `compile_mesh_to_10d`. |
| **E1** | `recipes/compile_hr_observation_quins` — deontic `ProcessingAct::Rppg` fail-closed + 3 epistemic VisionQuins. |

### Sprint 2 (this session) — B1 / D2

| Track | What was built |
|-------|----------------|
| **B1** | `sr/tile_plan`, `tile_extract`, `tile_blend`, `super_resolve_tiled` — plan/extract/feather blend; max_tiles fail-closed; flat image parity vs full-frame. |
| **D2** | `spatial/sigma_map` — class_hash × score → σ; wired into `detections_to_node_hints`. |

### Registry honesty

- D1.16 classical_sr **Present** (SR0+B1 tiling)
- D3.15 biosignal_graph **Present** (HR observation quins + Rppg gate)
- D5.13 10d_handoff **Partial** (mesh pack + NodeHint σ stub; full Tensor10DNodes open)

### Measured

```
cargo test -p qualia-vision --lib  →  339 passed, 0 failed
cargo check -p qualia-client-core  →  Finished ok
```

### ⚑ Where I need the human

None this step. Optional later: real MediaPipe `.task` corpus, attested SR weights under `vendor/vision/sr/`, acceptable portal size for vision lite.

### Next

- **A2** deeper: Forge resize when Cool (link path, still degrade without adapter)
- **C2** optional CG remesh after validate (host or thin bridge)
- **D1** host `compile_mesh_to_10d_with_nodes` from NodeHint
- **F1** desktop browse sealed vision `.10d`

---

## 2026-07-17 — Programme plan authored

**Status:** ready to execute (no code this entry)

**What:** Umbrella plan linking CV/SR/bio → GPU/shared_gpu → CG → `.10d`/σ → portal browser tooling; swarm tracks A–G; sprints 1–6.

**Related audits/plans already landed:** gap audit, SR excellence, bio catalogue, vendor vision pack.

**Next:** `execute sprint 1` — tracks A1 (vision gpu feature), B0 (classical SR), C1 (MeshIR→Mesh API), E1 (biosense modalities).

**⚑ Principal:** none for plan authorship.
