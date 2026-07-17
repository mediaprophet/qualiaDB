# Vision → 10D Browser Excellence — Progress Log

**Programme:** `vision-10d-browser-excellence-programme-2026.md`  
**Branch:** `0.0.25`

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
