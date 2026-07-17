# Plan — Native Vision Capability Excellence (2026 product)

**Date:** 2026-07-17  
**Ambition:** Ship the **best classical + integrated vision capability set** available in a **2026-produced, all-Rust, rights-aware, local-first** product — not OpenCV parity, and **not** a clone.  
**Market checklist (floor only):** [OpenCV 4.13.0 module index](https://docs.opencv.org/4.13.0/) and peer stacks (e.g. classical CV libs, mobile CV SDKs) — used to ensure we **do not leave capability holes**, then we **exceed** via integration that vendors cannot match.  
**Branch:** `0.0.25` · canonical tree only  
**Status:** Ready when Timothy says **execute vision-excellence** or **execute VX0**  
**Supersedes:** `opencv-coverage-parity-plan.md` (renamed/reframed; do not use “parity” language)  
**Related:** `native-visual-intelligence-and-generative-3d.md`, `qualia-vision`, `computational_geometry`, Forge, audio cross-modal, browser/Library pipelines  

---

## 0. North star

| | |
|--|--|
| **Product outcome** | A 2026 Qualia/Webizen release where vision work — preprocess, measure, detect, track, calibrate, reconstruct, explain — runs **in-process, pure Rust**, under the same governance as graph, identity, Library, and LLM. |
| **vs vendor OpenCV** | Qualia is the **implementation**, not a wrapper. No required OpenCV C++/Python vendor dependency for product features. |
| **vs “parity”** | Meeting every OpenCV *class* of function is the **floor**. The **ceiling** is to **exceed** the market by making those ops first-class **platform capabilities**: zero-heap ABI, epistemic honesty, semantic graph, geometry handoff, multimodal clock, local GPU, sanctuary/rights. |
| **What “best” means here** | Best *as an integrated local-first human-rights-centric system in 2026* — correctness + coverage + integration + honesty — not a synthetic leaderboard of isolated kernel microbenchmarks against every CUDA OpenCV build. |

If a capability exists only as “call OpenCV,” it is **not** a Qualia capability yet.

---

## 1. Purpose (principal framing)

| What this is | What this is not |
|--------------|------------------|
| Grow **Qualia capabilities** so classical and modern vision ops live **in-tree, in Rust**, inside the integrated environment | An OpenCV port or API clone |
| An **implementation alternative** to vendoring OpenCV | Competing for the brand name “OpenCV” |
| Coverage checklist → **fill holes** → then **surpass** via integration | Bit-exact matching of OpenCV pixels/symbols |
| Part of **something new** (graph + rights + geometry + multimodal + wgpu + LLM Sentinel) | A Mat-only side library for research scripts |

**Checklist sources (floor):** OpenCV main modules and similar classical CV surfaces.  
**Excellence sources (ceiling):** integration depth, edge/WASM readiness, determinism, auditability, twin/geometry, multimodal, no cloud requirement, no Python runtime.

---

## 2. Honest starting point (2026-07-17)

### 2.1 Already strong (beyond classical CV vendors)

- Semantic vision: observations as NQuins, reject/correct, SPARQL-MM-class queries  
- Media digests, seed QVWT path, detector/tracker scaffolds, synthetic metrics  
- Image→3D heightfield + MeshIR + twin eligibility / A1 honesty  
- Computational geometry (Delaunay, CSG, remesh, Poisson-class recon, exact kernels, …)  
- Forge/wgpu, volumetric render, local LLM stack  
- `qualia-audio` + shared media clock  

### 2.2 Classical vision surface (must grow to clear the floor, then exceed)

**Present / partial:** RGB views, resize/letterbox, NMS, few conv/pool ops, BMP/PNG-JPEG pipeline pieces, grid detector, association tracker.  

**Missing capability classes (floor checklist):** full colour suite, Gaussian/median/bilateral, morphology, Sobel/Laplacian/Canny, contours/CC, histograms, Hough, affine/perspective warps, feature detect/describe/match (ORB-class), optical flow, stereo/calib/PnP, denoise/inpaint, fuller video I/O.

**Excellence gap:** even after classical ops land, wire every product-facing result into digests, optional quins, Studio, geometry, and rights — that is how we **exceed** a standalone CV library.

---

## 3. Design principles

1. **All product path in Rust** — no Python; no required OpenCV product link.  
2. **Integrated environment** — ops compose with media store, epistemic compile, Forge, geometry, Studio, Library.  
3. **Zero-heap hot paths** for kernels; cold construction via bounded arenas.  
4. **Pixels never in NQuins** — hashes, boxes, descriptor digests, provenance.  
5. **Epistemic honesty** — measurements/proposals ≠ facts; model outputs labelled.  
6. **Shared wgpu** for GPU; CPU oracles mandatory for claimed kernels.  
7. **OpenCV only as optional test oracle** — never public ABI.  
8. **Licence-clean defaults** (ORB-class features; patented algos only with principal decision).  
9. **Capability registry** is the source of truth for “do we have this?”  
10. **Exceed via composition:** one pipeline that does filter → feature → pose → mesh → graph → human attestation without leaving the process.  
11. CLAIM/RELEASE; progress log; no silent incomplete.  
12. Libraryization under `qualia-vision/src/cv/` (+ GPU ports in Forge).

### 3.1 ABI sketch (Qualia-native)

```rust
pub struct ImageView2D<'a> { /* width, height, stride, format, bytes */ }

pub fn gaussian_blur_u8(
    src: ImageView2D<'_>,
    ksize: u32,
    sigma: f32,
    dst: &mut [u8],
) -> Result<(), CvError>;
```

Public surface: **Qualia types + capability registry**, not `cv::` names.

### 3.2 Layout

```text
crates/qualia-vision/src/cv/
  mod.rs            # capability registry (id, status, maturity, honesty)
  buffer.rs
  color.rs
  filter.rs
  morph.rs
  edges.rs
  hist.rs
  transform.rs
  contours.rs
  features/
  calib/
  flow.rs
  photo.rs
  codecs.rs
  excellence.rs     # compose pipelines that exceed single-library use
```

---

## 4. Capability matrix

### 4.1 Floor — classical classes (checklist labels only)

| Capability class | Status now | Excellence target |
|------------------|------------|-------------------|
| Buffer / ROI / arith / channels | Partial | Present + deterministic + GPU path where hot |
| Filters / morph / colour / edges / hist / contours / draw | Partial | Present + forge ports + Studio |
| Codecs (PNG/JPEG/…) | Partial | Present + provenance on import |
| Video file / camera I/O | Missing–partial | Present; camera **intent-gated** |
| Desktop CV windows | N/A | Studio is UI |
| Optical flow / BG / classical track | Partial tracker | Present + fused with semantic tracks |
| Homography / PnP / stereo / calibrate | Missing | Present + mesh/twin handoff |
| Features detect/describe/match | Missing | Present + graph-linkable descriptor digests |
| Dense/learned detectors | Scaffold | Production weights COMPLETE-WITH-GATE + H1 |
| Denoise / inpaint | Missing | Present minimum + honesty |
| Stitch / panorama | Missing | Optional vertical |
| Pipeline scheduling | Partial Forge | Explicit vision schedules |

### 4.2 Ceiling — exceed the market (Qualia-only)

| Excellence capability | Why it beats a vendor CV lib in 2026 |
|----------------------|--------------------------------------|
| **Epistemic graph** | Every detection/flow/calib result can be proposal + reject/correct + SPARQL-MM |
| **Rights & capture consent** | Camera/video fail closed without principal intent |
| **Geometry continuum** | Features/disparity → MeshIR → computational_geometry → twin A1 honesty |
| **Multimodal** | Shared media clock with auditory intelligence |
| **Local GPU + LLM Sentinel** | Same process, no cloud required for core path |
| **Determinism / attestation** | Hashable pipelines, receipts, seed weights honesty |
| **Edge / WASM profiles** | Capability-gated builds, not “full desktop OpenCV or nothing” |
| **Library catalogue** | Models/ontologies as first-class shelf entries |

**Market “best” claim** is earned only when **floor + ceiling** are both real — classical ops present **and** integrated.

---

## 5. Waves (VX — vision excellence)

### VX0 — Registry, ADR, fixtures

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| VX0.1 | ADR: excellence goal; OpenCV = floor checklist | Written |
| VX0.2 | Machine-readable capability registry in `cv/` | List status Present/Partial/Missing/Beyond |
| VX0.3 | Fixtures: edges, checkerboard, warp, stereo pair synthetic | Offline |
| VX0.4 | Progress log `native-vision-capability-excellence-PROGRESS-LOG.md` | Created |

### VX1 — Essential image processing (clear the floor)

Colour, blur/median/bilateral, morph, edges, hist, contours, draw, ROI/channels.  
**Exit:** classical preprocess fully in-tree; no OpenCV needed for that class.

### VX2 — Codecs + I/O

PNG/JPEG (+ WebP as needed); video file; camera with consent.  
**Exit:** media enters Qualia without vendor codecs stack.

### VX3 — Features + warps

ORB-class + match + RANSAC homography; warp affine/perspective.  
**Exit:** align/register without OpenCV; descriptor digests optional to graph.

### VX4 — Calib / stereo / depth → geometry

Zhang-class calib, PnP, stereo lite → MeshIR / `.10d` / geometry workspace.  
**Exit:** depth/pose vertical exceeds heightfield-only; twin-ready.

### VX5 — Motion excellence

LK / dense flow class; BG subtract; tracker fuses flow/features + semantic IDs.  
**Exit:** video analysis as platform capability.

### VX6 — Photo + optional stitch

Denoise, inpaint; optional panorama.  
**Exit:** restoration-class ops in-tree.

### VX7 — Learned vision (gated)

Licensed backbone; QVWT/P64/ONNX policy; H1 metrics split synthetic/real.  
**Exit:** COMPLETE-WITH-GATE only with principal assets.

### VX8 — Excellence composition + product surface

| Slice | Deliver |
|-------|---------|
| VX8.1 | End-to-end recipes: *import → filter → feature → pose → mesh → quins → Studio* |
| VX8.2 | Capability ledger published in-repo (honest Present/Partial/Missing/Beyond) |
| VX8.3 | Studio Vision workbench: run classical + semantic ops in one place |
| VX8.4 | Optional OpenCV **oracle** feature for regression only |
| VX8.5 | WASM/edge profile: declared subset of VX1–VX3 |

**Exit for “2026 product excellence”:** floor classes Present (or explicit N/A), ceiling pipelines dogfoodable, inventory honest, **no product dependency on vendor OpenCV**.

---

## 6. Priority

```text
VX0 registry/ADR
 → VX1 essential image processing
 → VX2 codecs/I/O
 → VX3 features + warps
 → VX8.1 partial recipes early (integration habit)
 → VX4 calib/stereo when depth/pose demanded
 → VX5 motion
 → VX6 photo/stitch as needed
 → VX7 licensed models
 → VX8 full ledger + UI + edge profile
```

---

## 7. Success criteria (no “parity”)

**Success is:**

1. Capability registry shows **Present** for every class we claim for the 2026 product.  
2. Those classes are **Rust, in-tree**, tested under Qualia ABI.  
3. Product features **do not require vendoring OpenCV**.  
4. At least one **excellence pipeline** (VX8.1) runs end-to-end with digests + optional quins + geometry handoff.  
5. Honesty labels everywhere (reference vs production weights; synthetic vs H1).  
6. Long tail remains explicit Missing/N/A — never silent.

**Failure modes to avoid:**

- Calling the work “parity” or “clone.”  
- Shipping kernels without integration.  
- Claiming “best in market” without registry evidence and dogfood.  
- Pulling OpenCV into the product default path.

---

## 8. Effort honesty

| Wave | Note |
|------|------|
| VX0–VX1 | Foundation; multi-session |
| VX2–VX3 | High product leverage |
| VX4–VX6 | Verticals by demand |
| VX7 | Human licence/corpus gates |
| Full CUDA-contrib surface of OpenCV | **Not required** for excellence claim |

Excellence is **integrated completeness + quality**, not “every OpenCV contrib module.”

---

## 9. Principal decisions

| ID | Ask | Default if silent |
|----|-----|-------------------|
| **VX-D1** | Fill through VX3 first? | **Yes** |
| **VX-D2** | Optional OpenCV test oracle? | Yes; never product default |
| **VX-D3** | Camera vs file I/O first? | File first |
| **VX-D4** | Patented detectors? | No — ORB-class default |
| **VX-D5** | Public “best in class 2026” marketing claim timing? | Only after VX8 ledger green |

---

## 10. Immediate next step

When Timothy says **execute vision-excellence** or **execute VX0**:

1. CLAIM `qualia-vision/src/cv/` (and Forge only if GPU slice claimed).  
2. Land ADR + capability registry + fixtures + progress log.  
3. Implement VX1.  
4. After each wave: update registry + progress log + tests.

---

*End of plan. Floor = classical capability classes (OpenCV as checklist). Ceiling = integrated 2026 Qualia excellence. Implementation = pure Rust native, never a vendor clone.*
