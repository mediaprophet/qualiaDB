# Vision → GPU → 10D Browser Excellence Programme (2026)

**Branch:** `0.0.25`  
**Canonical tree:** `C:\Projects\qualia-27062026` only  
**Status:** Ready to execute with multi-agent swarms  
**Purpose:** Close the loop from **pixels / biosense / SR** through **domains · modalities · solvers · computational geometry · shared GPU** into **`.10d` tooling** that powers **portal / 10D browser capabilities** (native desktop first, WASM portal second).

This is the **umbrella programme**. Specialist plans remain the detailed specs; this file is the orchestrator’s DAG, collision map, excellence bar, and browser deliverable.

---

## 0. Why this programme exists

| Already true in-tree | Still false for product CV |
|----------------------|----------------------------|
| `shared_gpu`, Forge, PortalGpu, CUDA/DML lanes | `qualia-vision` is CPU-only (`gpu = []`) |
| `render::compile_10d`, spectral/acoustic **σ**, Tensor10D | Vision recon seals **mesh only** — no σ-nodes |
| `computational_geometry` (Delaunay, CSG, remesh, Poisson, …) | Vision never calls it |
| `domains/*`, `modalities/*`, `solvers/*` | CV results often stop at pixels / MeshIR |
| WASM **portal** WebGPU viewport + LLM | **No** `qualia-vision` on any wasm profile |

**Principal goal:** tooling so a human can capture/enhance/analyse imagery, ground it in rights + domain science, seal **manifold geometry** (`.10d` + quins), and **browse / scrub / hear** it in the 10D portal — without a second GPU stack, OpenCV product ABI, or Python.

---

## 1. Non-negotiables (excellence)

1. **One GPU story:** `gpu_context::shared_gpu()` — Vulkan / DX12+DirectML / Metal / WebGPU via wgpu; optional CUDA as EP/lane when Cool. No second adapter; no NCNN product dep.  
2. **Native algorithms in Rust** under `qualia-vision` (`cv/`, `bio/`, `sr/`, …); external projects = **weights + algorithm lineage** only.  
3. **PermissiveReady defaults** (MIT/Apache); TrainingDeferred does not block inference.  
4. **Modalities for rights/certainty** — deontic/epistemic/spatio-temporal; never silent biometrics.  
5. **Render owns projection** — `compile_10d`, σ → spectral + acoustic; don’t invent a parallel EMF path.  
6. **Solvers own numerics** — SVD/FFT/stats/RF when already present; don’t duplicate in vision.  
7. **CG owns mesh quality** — remesh/Poisson/BVH/topology sections.  
8. **Domains ground meaning** — geo → `domains::geospatial`; bio/stain → biological/chemical.  
9. **Anti-monolith** — single-function files; library subdirs.  
10. **Honesty** — Generative SR ≠ ground truth; clinical = proposals; D5.13 is mesh-seal until σ-nodes land.  
11. **Browser 10D is a product of this programme**, not a separate chrome experiment.

---

## 2. Source-of-truth documents (do not fork)

| Doc | Owns |
|-----|------|
| This file | Programme DAG, swarms, browser deliverables |
| `vision-gpu-10d-geometry-gap-audit-2026.md` | Gap register (as-is) |
| `native-super-resolution-excellence-2026.md` | SR tiers U/B/H, tiling, SR0–SR6 |
| `native-vision-capability-excellence-2026.md` | D1–D9 catalogue |
| `vision-capability-catalogue-2026.md` | Industry backlog W0–W8 |
| `bio-medical-cv-catalogue-2026.md` | Histopath / DICOM / deep-gated map |
| `vision-excellence-commercial-model-pack.md` + `vendor/vision/` | Weights |
| `wasm-capability-profiles.md` | Portal vs ontology vs full |
| Browser swarm plans | Trust/chrome only — **consume** Library + 10d tooling from here |

---

## 3. End-state architecture

```text
                    ┌──────────────────────────────────────────┐
                    │  shared_gpu()  ·  Forge  ·  ort EP       │
                    │  Vulkan / DX12-DML / Metal / WebGPU / CUDA│
                    └──────────────────┬───────────────────────┘
                                       │
   camera/files ──► qualia-vision ─────┤
   (cv, bio, SR,   CPU oracles + gpu   │
    biosense,      feature dispatch    │
    recipes)                           │
         │                             │
         │  solvers (LA, FFT, stats)   │
         │  computational_geometry     │
         ▼                             ▼
   MeshIR / volumes ──► Mesh ──► render::compile_10d
         │                    (+ Tensor10DNodes, topology, BVH, provenance)
         │                             │
         ▼                             ▼
   NQuin observations          sealed .10d + digests
         │                             │
         ▼                             ▼
   modalities (epistemic/deontic/   render::spectral + acoustic (σ)
   spatio_temporal/SHACL)              │
         │                             ▼
         ▼                      PortalGpu / portal WASM
   domains (geo/bio/chem)       10D browser: paint, scrub, hear, rights gate
```

**Desktop first** for full CV+GPU; **portal WASM** for 10D browse of sealed assets + WebGPU viewport (not full classical CV in browser unless size-gated later).

---

## 4. Programme phases (excellence milestones)

| Phase | Name | Done when |
|-------|------|-----------|
| **P0** | Orientation | This plan + audit live; CLAIMs in NOTICES |
| **P1** | GPU spine | Vision `gpu` feature dispatches Forge vision ops on `shared_gpu`; VRAM/thermal degrade |
| **P2** | SR excellence | SR0–SR5 classical+tiling+light CNN+GPU; optional ESRGAN/Swin |
| **P3** | Geometry excellence | MeshIR → CG cleanup → `.10d` with topology/BVH; print path solid |
| **P4** | Manifold / EMF | Detections/bio → Tensor10DNodes + σ; spectral+acoustic in portal |
| **P5** | Rights + domains | Biosense/PAD/geo/bio through modalities + domain adapters |
| **P6** | 10D browser tooling | Load/browse/scrub sealed `.10d` in desktop + portal WASM; provenance UI |
| **P7** | Closeout | Registry honesty; progress log; Library MANIFEST; no false Present |

Phases may overlap when tracks are exclusive (see §6).

---

## 5. Track map (swarm units)

Each track is **≤ one session**, exclusive paths, own tests. Orchestrator spawns in parallel only when Exclusive paths do not overlap.

### Block A — GPU spine (P1)

| Track | Exclusive paths | Deliverable | Tests |
|-------|-----------------|-------------|-------|
| **A1** | `qualia-vision/Cargo.toml`, thin `gpu/` facade | Non-empty `gpu` feature; optional dep on core forge/shared_gpu | feature builds |
| **A2** | `qualia-vision/src/gpu/` or bridge | Dispatch `ops` Conv/Pool/Resize → Forge when Cool | CPU oracle parity |
| **A3** | `gpu_context` or vision policy only | Vision VRAM tag + thermal degrade helper | unit |
| **A4** | `weights/onnx_session` | ort DML/CUDA EP selection when available | smoke if hardware |

### Block B — Super-resolution (P2) — detail in SR plan

| Track | Maps to | Exclusive |
|-------|---------|-----------|
| **B0** | SR0 classical | `cv/sr/` |
| **B1** | SR1 tiling | `sr/tile_*` |
| **B2** | SR5 classical WGSL | Forge + `sr/gpu_classical` |
| **B3** | SR2 FSRCNN/ESPCN | `vendor/vision/sr/`, `sr/cnn_light` |
| **B4** | SR3 ESRGAN tiled | `sr/esrgan_*` |
| **B5** | SR4 SwinIR | `sr/swin_*` |
| **B6** | SR6 surfaces | desktop/studio enhance |

### Block C — Geometry → `.10d` (P3)

| Track | Exclusive | Deliverable |
|-------|-----------|-------------|
| **C1** | `qualia-vision/spatial/` + client adapter | Shared `MeshIR`→`Mesh` API (promote out of client-only) |
| **C2** | CG cold path + vision recipe | Optional remesh/decimate/quality after validate |
| **C3** | `render/compile_10d` consumers | Topology + SpatialIndex sections on vision recon |
| **C4** | `spatial/` + print | Photogrammetry multi-view beyond heightfield (or honest Partial) |

### Block D — Manifold / EMF σ (P4) — **10D browser fuel**

| Track | Exclusive | Deliverable |
|-------|-----------|-------------|
| **D1** | `render/compile_10d` + vision handoff | `compile_mesh_to_10d_with_nodes` from detections |
| **D2** | vision semantic + spectral map | Class/confidence → **σ** (documented table) |
| **D3** | `render/spectral`, `acoustic` | Portal paint/scrub uses node σ (already for anatomy — wire vision assets) |
| **D4** | provenance | ProvenanceSidecar: media digest, model hash, consent purpose |

### Block E — Rights + domains (P5)

| Track | Exclusive | Deliverable |
|-------|-----------|-------------|
| **E1** | biosense recipes + modalities | Every biosense path emits epistemic quins + deontic gate |
| **E2** | geospatial adapters | Geo-tagged frames → `domains::geospatial` canvas/place |
| **E3** | bio stain → domain | Histopathology proposals → biological sensitivity tags |

### Block F — 10D browser tooling (P6) — **principal product cut**

| Track | Exclusive | Deliverable |
|-------|-----------|-------------|
| **F1** | desktop commands | `browse_vision_10d`, load `recon.10d`, list digests in Library |
| **F2** | portal / render | Portal load sealed vision `.10d` from bytes (WASM portal profile) |
| **F3** | time scrub / navigation | Use `render/time_scrub`, camera, standpoint for multi-node assets |
| **F4** | rights barrier | `render/barrier` + place_time: refuse unattested / forbidden contexts |
| **F5** | Library catalogue | SR + mesh + 10d assets on Software shelf |
| **F6** | size gate | Portal wasm size budget; no full `qualia-vision` in ontology wasm |

### Block G — Closeout (P7)

| Track | Deliverable |
|-------|-------------|
| **G1** | Registry D1–D9 honesty pass (Present only when true) |
| **G2** | Progress log wave entries |
| **G3** | Integration smoke: capture/synthetic → SR optional → MeshIR → CG opt → `.10d` → portal paint |
| **G4** | RELEASE NOTICES + push |

---

## 6. Parallelism DAG

```text
P0 ──► A1 ──► A2 ──► A3
              │
              ├──────────► B0 ──► B1 ──► B2 ──► B3 ──► B4/B5 ──► B6
              │                      │
              │                      └── (B2 needs A2)
              │
              ├──► C1 ──► C2 ──► C3 ──► C4
              │              │
              │              └──► D1 ──► D2 ──► D3 ──► D4
              │
              ├──► E1 ──► E2/E3
              │
              └──► F1–F6 (after D1+D3 minimum for real 10d browse)
                        │
                        └──► G1–G4
```

**Safe parallel spawn examples:**

| Wave | Tracks |
|------|--------|
| **Sprint 1** | A1, B0, C1, E1 |
| **Sprint 2** | A2, B1, C2, D2 (σ table design) |
| **Sprint 3** | A3, B2, C3, D1 |
| **Sprint 4** | B3, D3, F1, F5 |
| **Sprint 5** | B4/B5, F2–F4, F6 |
| **Sprint 6** | B6, G* |

**Collision rules:**

- One agent owns `commands/mod.rs` appends (desktop) at a time  
- `render/compile_10d.rs` — C3/D1 serial or same agent  
- `gpu_context.rs` — A3 only unless CLAIM released  
- No git worktrees; canonical tree only  

---

## 7. 10D browser capabilities (explicit deliverables)

What “tooling for 10D browser” means in this programme:

| Capability | Native desktop | WASM portal |
|------------|----------------|-------------|
| Load sealed `.10d` from vision recon | F1 | F2 |
| Display mesh on shared/WebGPU device | F1 → PortalGpu | portal WebGPU |
| Paint Tensor10D nodes with **σ** colour | D3 | spectral path |
| Hear σ as acoustic plane | D3 | AcousticPlane if profile allows |
| Time scrub / multi-node navigation | F3 | F3 |
| Rights: refuse without attestation | F4 / barrier | same SHACL/deontic |
| Library index of digests | F5 | read-only list |
| Live classical CV in browser | **Out of scope** for ontology wasm; optional future lite profile | — |

**Success demo (desktop):**  
synthetic/demo image → optional SR → heightfield or CG mesh → `.10d` with nodes+σ → open in 10D browser → spectral colour + optional audio scrub.

**Success demo (portal):**  
fetch sealed `.10d` bytes (no full CV) → WebGPU paint with σ — proves **browser 10D** without shipping OpenCV-in-WASM.

---

## 8. Track agent prompt template

```text
You are Track [ID] on Vision-10D Browser Excellence Programme (branch 0.0.25).
Canonical: C:\Projects\qualia-27062026 only. No worktrees.

Read first:
- docs/plans/vision-10d-browser-excellence-programme-2026.md (§ your track)
- docs/plans/vision-gpu-10d-geometry-gap-audit-2026.md
- specialist plan if SR/bio (native-super-resolution / bio-medical catalogue)

Rules:
- Use domains, modalities, render, solvers, computational_geometry, shared_gpu —
  do not invent parallel stacks.
- Native Rust; OpenCV/NCNN = weights/lineage only.
- Single-function files; fail closed on rights/thermal/VRAM.
- CLAIM exclusive paths in coordination/NOTICES.md.
- cargo test for your package; no false Present in registry.

Scope: [paste track table row]
Done: acceptance tests + progress-log line + RELEASE/PROGRESS notice.
```

---

## 9. Excellence acceptance (programme-level)

| # | Criterion |
|---|-----------|
| 1 | Vision GPU path exercises `shared_gpu` on at least one host backend (DX12/Vulkan/Metal) for classical or Forge vision ops |
| 2 | SR classical beats nearest (PSNR); tiled path never OOMs under default budget |
| 3 | Vision recon can emit `.10d` with **mesh + Tensor10DNodes + provenance** (σ set by documented map) |
| 4 | Optional CG remesh path exists and is opt-in |
| 5 | Desktop 10D browser loads vision `.10d` and paints σ |
| 6 | Portal WASM loads sealed `.10d` (browse) without linking full `qualia-vision` |
| 7 | Biosense/PAD remain consent-bound; generative SR labelled |
| 8 | Registry rows match reality; progress log per sprint |

---

## 9-B. Library home — honesty + migration (principal ask, 2026-07-17)

**Today (true):** vision *algorithms* live in the separate crate **`qualia-vision`**
(`cv/`, `bio/`, `biosense/`, `sr/`, `spatial/`, …). They are **not** under
`qualia-core-db::specialized_libs::*`. What *is* in core-db:

| In `qualia-core-db` | Role |
|---------------------|------|
| `render::compile_10d`, spectral, acoustic, Tensor10D | Seal / paint / 10D atom |
| `container_10d` | Mesh, nodes, topology, spatial, provenance |
| `specialized_libs::computational_geometry` | QEM, half-edges, BVH (host C2/C3) |

| Outside core-db | Role |
|-----------------|------|
| `qualia-vision` | CV/SR/biosense/bio native algorithms (edge-friendly, no forced full engine link) |
| `qualia-client-core` `vision_*` | Host pipelines, browse/load, Gs continuum |

**Why separate crate (current design):** programme F6 / size gate — portal and
ontology WASM must load sealed `.10d` **without** linking full classical CV.

**Migration TODO (must land; not deferred forever):**

| ID | Work |
|----|------|
| **MIG-V1** | Inventory which `qualia-vision` modules are pure cold algorithms vs product recipes |
| **MIG-V2** | Promote pure kernels into `specialized_libs::computer_vision` (or `vision_cv`) with same wasm gates as other specialized libs — **or** re-export facade from core-db without breaking crate ABI |
| **MIG-V3** | Keep `qualia-vision` as thin edge/product surface (weights, recipes, consent) over specialized_libs |
| **MIG-V4** | MCP / Library catalogue rows for vision specialized lib parity with crypto/medical |

Until MIG-V2, do **not** claim vision is a `specialized_libs` peer of medical/physics.
Track MIG-* in the progress log and closeout G1.

---

## 10. Explicit non-goals

- Servo / full browser engine  
- Full classical CV inside `wasm-ontology`  
- Replacing Wellfair anatomy pipeline (integrate, don’t fork)  
- NC/GPL SR weights as defaults  
- Claiming FEA A4 from vision twin without solvers/CG honesty  
- Claiming vision algorithms already live under `specialized_libs` before MIG-V2

---

## 11. Orchestrator checklist (start of programme)

- [ ] Append `CLAIM | Vision-10D Browser Excellence Programme | A1 B0 C1 E1` to `coordination/NOTICES.md`  
- [ ] Spawn Sprint 1 tracks with exclusive paths  
- [ ] After each sprint: `cargo test -p qualia-vision --lib` (+ client/desktop as touched)  
- [ ] Progress log entry  
- [ ] Push `0.0.25`  

**Execute phrase:** `execute vision-10d programme` or `execute sprint 1` (A1 B0 C1 E1).

---

## 12. Session / progress log

Append to `native-vision-capability-excellence-PROGRESS-LOG.md` or create:

`docs/plans/vision-10d-browser-excellence-PROGRESS-LOG.md`

Each sprint: phase, tracks done, measured tests, ⚑ principal asks, next sprint.

---

*End of programme plan. Excellence = closed loop from sensor to 10D browser on Qualia’s existing engine — domains, modalities, render, solvers, geometry, GPU — via swarms with exclusive tracks.*
