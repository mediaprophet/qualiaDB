# Vision / CV gap audit — GPU (native + WASM), `.10d` / EMF σ, computational geometry

**Date:** 2026-07-17  
**Branch:** `0.0.28`  
**Trigger:** Principal concern that CV excellence and SR plans under-used the existing GPU backplane, 10D container (inc. EMF/σ), and computational geometry library.  
**Status:** Audit complete. Findings are against the tree as of this date — not a claim that gaps are fixed.

**Core pillars (do not forget when wiring CV):**

| Tree | Path | Role for vision / SR / bio |
|------|------|----------------------------|
| **domains** | `qualia-core-db/src/domains/` | Domain science: `biological/`, `chemical/`, `geospatial/` (DEM, STAC, terrain, canvas), `physical/`, `mathematical/`, `financial/` — CV/bio results land as **domain-grounded** quins / adapters, not free-floating pixels |
| **modalities** | `qualia-core-db/src/modalities/` | Epistemic/deontic/spatio-temporal/manifold/logic — **rights + certainty + LTL** around observations; SHACL geometry assets; `manifold` / `spatio_temporal` bind place-time |
| **render** | `qualia-core-db/src/render/` | `compile_10d`, spectral/acoustic **σ**, `PortalGpu`, assets, anatomy_pack, place_time, physics of artefacts — **manifold projection + EMF display** |
| **solvers** | `qualia-core-db/src/solvers/` | Numerics CV may **call** (not reimplement): `linear_algebra` (SVD/Macenko-class), `transforms/fourier` (rPPG/spectral), `interpolation`, `statistics`, calculus/ODE, learning (RF-class when wired), geometric algebra |

Also remember: `specialized_libs/computational_geometry/`, `gpu_context`, `container_10d`, `tensor::Tensor10D`.

---

## 1. Bottom line (honest)

| Area | Reality today |
|------|----------------|
| **Native GPU for LLM + volumetric render** | **Real** — `gpu_context::shared_gpu()`, optional CUDA/DirectML, `PortalGpu`, Forge |
| **Native GPU for classical CV / biosense / bio** | **Not wired** — `qualia-vision` is CPU (+ optional `ort` without GPU EP config) |
| **WASM GPU** | **Viewport + optional LLM** via WebGPU — **no** `qualia-vision` on any wasm product |
| **Vision → `.10d`** | **Partial** — heightfield recon → sealed **QuantizedMesh** only (client pipeline) |
| **Vision → Tensor10D / EMF σ (α,μ,σ spectral lane)** | **Not closed-loop** from CV detections / bio |
| **Vision → computational_geometry** | **Not used** (zero imports); CG library is large and real on its own |

Your worry is **correct**: recent CV/biosense/bio/SR *plans* mention GPU; **product CV code does not call** `shared_gpu`, Forge vision ops, VRAM ledger, or CG remesh/topology. The engine already has the backplane — CV has not sat on it yet.

---

## 2. GPU — native path

### 2.1 What exists and is production-relevant

| Layer | Path | Role |
|-------|------|------|
| Shared device | `qualia-core-db/src/gpu_context.rs` | Process-wide `wgpu` Device/Queue; Windows default **DX12**; override `QUALIA_WGPU_BACKEND` → vulkan / metal / gl |
| DirectML | Optional LLM / VRAM probe (`inference/directml_bridge.rs`) | Side path + budget probe — **not** vision |
| CUDA | `feature = "cuda"`, `inference/cuda_lane.rs`, Forge WMMA | **LLM / Forge GEMM** primarily |
| Volumetric render | `render/gpu` `PortalGpu::new_offscreen` → **shared_gpu** | Mesh / Tensor10D viewport |
| SDK | `webizen-render` VolumetricRenderer | Same shared device |
| Forge vision ops | `wgsl_forge/graph_ops/vision.rs` | **WGSL + CPU oracle**: Conv2d, Pool2d, Resize2d |
| VRAM ledger | `global_vram_ledger()`, universes U0 LLM / U1 Tensor10D / U2 Viewport | Render records; **no Vision universe** |
| Thermal | `ThermalGovernor` (LLM-oriented) | **Not** gating CV jobs |

### 2.2 What `qualia-vision` does

| Feature / area | GPU? |
|----------------|------|
| `cv/*` classical | CPU only |
| `ops/{conv2d,pool2d,resize2d}` | CPU oracles (contracts for Forge) |
| `biosense/*`, `bio/*`, embeddings, recipes | CPU only |
| `gpu = []` in Cargo.toml | **Empty stub** (“Future: wire shared wgpu + forge”) |
| `ort` | Optional; **no** CUDA/DML EP selection in-tree |

**Conclusion:** Native GPU backplane is **shared by LLM + renderer + Forge**, not by the vision crate. SR plan §0.1 is the *intended* wire-up; it is **not yet code**.

### 2.3 Minimum fix programme (native CV on GPU)

| Step | Work |
|------|------|
| G1 | Non-empty `qualia-vision` feature `gpu` → optional dep on `qualia-core-db` (`gpu-runtime` / forge) **or** thin facade crate to avoid wasm link bloat |
| G2 | Dispatch `ops::conv2d/pool/resize` through Forge executor when `shared_gpu` Cool |
| G3 | `ComputeUniverse::Vision` (or borrow U2 Viewport with explicit policy) + VRAM estimate before tiles/ONNX |
| G4 | Thermal degrade: Cool→full, Warm→classical/light, Critical→CPU classical only |
| G5 | ort session: request DML/CUDA EP when present; report backend in SrReport / metrics |
| G6 | Optional: detector/SR tile loop uploads RGBA8 → compute → readback via shared device (same as PortalGpu readback pattern) |

---

## 3. GPU — WASM path

### 3.1 Profiles

| Profile | GPU | Vision crate? |
|---------|-----|----------------|
| `wasm-ontology` / webizen-lite-wasm | **No** | **No** |
| `portal` | **WebGPU viewport** | **No** |
| `wasm-llm` / `wasm-full` | WebGPU **inference** + portal | **No** |
| `wasm-scientific` | gpu-runtime for solvers | **No** CV |

### 3.2 Construction split

| | Native | WASM |
|--|--------|------|
| PortalGpu | `shared_gpu()` offscreen; surface = dedicated instance | `try_new_async` + `BROWSER_WEBGPU` + canvas |
| `wgsl_forge` | Compiled (`not(wasm32)`) | **Not compiled** |
| `qualia-vision` | Linked (desktop/client) | **Not linked** into any wasm product |

Viewport WGSL (`shaders/viewport/*`, spectral colour from **σ**) is **display**, not CV preprocess.

### 3.3 WASM CV implication

There is **no** browser classical-vision GPU path. Edge vision would need a new product slice (size/Sentinel gated), likely **CPU first**, then a **subset** of WGSL vision ops under a non-ontology profile — Forge as-is is native-only.

---

## 4. `.10d`, Tensor10D, EMF / σ spectrum

### 4.1 Two “10D”s (do not conflate)

| Name | Module | Role |
|------|--------|------|
| **`Tensor10D`** | `tensor/mod.rs` | 40-byte atom: `q,v,w,x,y,z,t,α,μ,σ` |
| **`.10d` container** | `container_10d/` | Sealed geometry sidecar (`10d\0`) |
| **`ManifoldCoordinate10D`** | P64 / manifold | **LLM weight** geometry — **not** vision meshes |

### 4.2 EMF / spectral meaning of σ

- Axis order (normative): `q,v,w,x,y,z,t,α,μ,σ` — `axis_role.rs`
- **σ** is the shared phenomenal / spectral index (not a separate EMF struct in the mesh section)
- Projection: `render/spectral.rs` (σ → λ 400–700 nm → CIE/sRGB), `render/acoustic.rs` (σ → Hz)
- Anatomy burden → σ: wellfair `burden_to_sigma` (organ colour/sound) — **not** vision MeshIR path

### 4.3 Vision handoff today

```text
qualia-vision MeshIR (heightfield)
    → client vision_pipeline::mesh_ir_to_core_mesh
    → compile_mesh_to_10d  →  QuantizedMesh section only
    → geometry NQuins + recon.10d on disk
```

**Missing for “full engine” handoff:**

- No `Tensor10DNodes` from detections / class / confidence  
- No σ assignment from vision class, rPPG, affect, or bio stain index  
- No Topology / SpatialIndex sections from vision (those need CG)  
- Bio volume path (HU/MIP) does **not** yet emit mesh or `.10d`

### 4.4 Minimum fix programme (CV → 10d / σ)

| Step | Work |
|------|------|
| T1 | Promote `mesh_ir_to_core_mesh` into shared API (vision or core) |
| T2 | Optional `compile_mesh_to_10d_with_nodes`: pack detection centroids / ROI as Tensor10D (x,y,z,t,σ from class colour map) |
| T3 | Provenance sidecar on vision recon (media digest, model hash) |
| T4 | Bio: isosurface / marching lite or heightfield from volume → MeshIR → same seal |
| T5 | Studio: load `recon.10d` into PortalGpu / volumetric (desktop cmds partially exist for anatomy packs) |

---

## 5. Computational geometry library

### 5.1 Location and scale

`qualia-core-db/src/specialized_libs/computational_geometry/` — large native library:

Delaunay 2/3, Voronoi, CSG/boolean, remesh/decimate, screened Poisson, isosurface, BVH/kd, hulls, arrangements, TDA, **GPU geometry** (`gpu.rs`, `gpu_3d.rs`), **authoring → `.10d`**, MCP `execute_geometry_tool_json`.

Also used by `container_10d` topology / spatial_index sections.

### 5.2 Vision connection

| Link | Status |
|------|--------|
| Vision imports CG | **None** |
| Twin bridge → remesh/Poisson/quality | **No** (viz / A1 only) |
| Registry D6.01 | “Beyond / Present + vision mesh ingest” — **ingest not done** |
| Desktop CG tool | Separate command path, not vision pipeline |

### 5.3 Minimum fix programme (CV → CG)

| Step | Work |
|------|------|
| C1 | After MeshIR validate: optional `remesh_3` / `decimate_3` / `mesh_quality` via CG (feature-gated, cold Tier-2 arenas) |
| C2 | Photogrammetry / multi-view recon call `screened_poisson` / `reconstruct_3d` instead of heightfield-only |
| C3 | Emit Topology + SpatialIndex sections into vision `.10d` using CG builders |
| C4 | Bio cell labels → surface extraction → CG mesh cleanup → print readiness |

---

## 6. Architecture diagram (as-is vs target)

```text
AS-IS
  qualia-vision ──CPU──► MeshIR ──client──► Mesh ──compile_10d──► QuantizedMesh .10d
       │                                      │
       │ no shared_gpu                        └── geometry quins
       │ no CG
       ▼
  optional ort (CPU EP)

  shared_gpu ◄── LLM QTensorEngine
             ◄── PortalGpu / webizen-render (Tensor10D + mesh display, σ colour)
             ◄── WGSL Forge (incl. vision Conv/Pool/Resize — unused by vision)

TARGET (programme)
  qualia-vision ──CPU oracles + gpu feature──► Forge / shared_gpu tiles
       │
       ├─ MeshIR ──optional CG remesh/Poisson──► Mesh
       │              │
       │              ▼
       │         compile_10d (mesh + Tensor10DNodes σ + topology + provenance)
       │              │
       └──────────────┴──► PortalGpu / studio (same shared_gpu)
```

---

## 7. How domains / modalities / render / solvers fit CV (wiring targets)

```text
  pixels / mesh (qualia-vision)
        │
        ├─► solvers::linear_algebra / transforms / statistics   (numerics helpers)
        ├─► specialized_libs::computational_geometry            (remesh, Poisson, BVH)
        │
        ▼
  NQuin observations ── modalities (epistemic, deontic, spatio_temporal, SHACL)
        │
        ├─► domains::geospatial (place/DEM/canvas) when geo-tagged
        ├─► domains::biological / chemical when stain/bio assays
        │
        ▼
  render::compile_10d + Tensor10D (σ → spectral + acoustic)
        │
        ▼
  render::gpu PortalGpu / shared_gpu   (same backplane as LLM)
```

**Do not** reimplement SVD/FFT/RF inside vision when `solvers` already owns them.  
**Do not** project σ outside `render::spectral` / `acoustic`.  
**Do not** attach rights outside `modalities` deontic/epistemic.  
**Do not** invent a parallel geo stack — use `domains::geospatial`.

---

## 8. Recommended swarm order (closes the gap)

| Wave | Name | Depends |
|------|------|---------|
| **VG0** | Audit (this doc) | — |
| **VG1** | `qualia-vision` `gpu` feature → dispatch Forge vision ops + shared_gpu | VG0 |
| **VG2** | VRAM/thermal policy for vision/SR tiles | VG1 |
| **VG3** | MeshIR → CG optional cleanup → full `.10d` sections | — |
| **VG4** | Detections → Tensor10DNodes + σ map via **render** spectral/acoustic | VG3 |
| **VG5** | SR classical WGSL on shared_gpu (SR5 from SR plan) | VG1 |
| **VG6** | WASM: decide product (CPU lite vision vs none); no Forge on wasm without redesign | policy |
| **VG7** | Wire biosense/bio outputs through **modalities** + optional **domains::biological** | parallel |
| **VG8** | Prefer **solvers** LA/FFT/stats for histopathology/radiomics where duplicated | cleanup |

**Do not** claim D5.13 “10d handoff Present” as “full manifold with EMF” — it is **mesh seal + quins**, not σ-node manifold.

---

## 9. Registry honesty tweaks (recommended)

| ID | Today | Fairer note |
|----|-------|-------------|
| D5.13 | Present “geometry quins path” | Present: mesh→`.10d` QuantizedMesh; **Partial**: Tensor10DNodes/σ/topology |
| D6.01 | Beyond CG | Beyond CG lib; **Missing** vision mesh ingest |
| D9.04 wasm edge | Partial | Partial: **no** qualia-vision on wasm |
| D1 / D2 SR GPU | plan only | Missing until VG1/VG5 |

---

## 10. Principal sign-off (optional)

| ID | Question | Default |
|----|----------|---------|
| VG-P1 | Add `ComputeUniverse::Vision` or share U2 Viewport? | Share U2 with explicit “vision tile” ledger tags first |
| VG-P2 | Link vision into any wasm profile this year? | No (size); desktop GPU first |
| VG-P3 | Auto-remesh vision recon via CG? | Opt-in quality pass, default heightfield |

---

*Audit only. Implementation starts with VG1 / SR0–SR1–SR5 when principal says execute.*
