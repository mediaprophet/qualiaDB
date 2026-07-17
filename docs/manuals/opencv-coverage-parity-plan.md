# Plan — Native Classical Vision Capabilities (OpenCV as coverage checklist, not a clone)

**Date:** 2026-07-17 (reframed same day)  
**Reference checklist:** [OpenCV 4.13.0 module index](https://docs.opencv.org/4.13.0/) — used only as a **capability checklist**, not as an architecture or API to copy  
**Branch:** `0.0.25` · canonical tree only  
**Status:** Ready when Timothy says **execute native-cv wave N** (alias: opencv-capability-fill)  
**Related:** `native-visual-intelligence-and-generative-3d.md`, `qualia-vision`, `computational_geometry`, Forge vision ops, capability manifests  

---

## 0. Purpose (principal framing)

This work is **not** “clone OpenCV” and **not** “vendor OpenCV into Qualia.”

| What it is | What it is not |
|------------|----------------|
| Grow **Qualia capabilities** so classical vision operations exist **in-tree, in Rust**, inside the integrated environment | A line-by-line or API-compatible OpenCV port |
| An **implementation alternative** to linking/vendoring OpenCV (C++/bindings/Python) | Competing on “are we OpenCV 4.13?” as a brand |
| Coverage questions answered by: *does Qualia expose this class of function under our ABI?* | Coverage answered by: *do we match OpenCV symbols/behaviour bit-exact?* |
| Part of **something new**: semantic graph + rights + geometry + multimodal + zero-heap + wgpu | A drop-in replacement for every OpenCV consumer |

**OpenCV docs are a map of function *classes*** (filter, morph, features, calib, flow, …). Qualia implements those classes **as native capabilities** of the platform, then wires them into media store, NQuins, Studio, Forge, and twins — which OpenCV does not do.

---

## 1. Short status

| Question | Answer |
|----------|--------|
| Is every OpenCV-main-module *class* of capability already in Qualia? | **No.** Classical CV surface is still thin vs the checklist. |
| Is Qualia already “something else” with strengths OpenCV lacks? | **Yes** — semantic observations, rights, geometry depth, audio, LLM/Sentinel, integrated desktop. |
| End state of this plan? | **Qualia capability inventory** lists classical vision ops as first-class, pure-Rust, integrated — so product work does **not** need a vendor OpenCV dependency for those ops. |

---

## 2. Why “coverage” still matters (without cloning)

Without a checklist, gaps hide as “we’ll call OpenCV later” or silent incomplete.

With a checklist:

1. Each OpenCV **module class** becomes a **Qualia capability row** (present / partial / missing).  
2. Missing rows become **implementation work in Rust**, not vendor tickets.  
3. Done means: callable from `qualia-vision` / engine paths, tested, honesty-labelled, integrable with digests/quins/UI.  
4. Long-tail OpenCV contrib/CUDA rows stay **out of scope** unless a product vertical needs them.

That is **capability completion**, not cloning.

---

## 3. Design principles

1. **All product path in Rust** — no Python; no required OpenCV C++ link for shipping features.  
2. **Integrated environment** — ops live next to media store, epistemic compile, Forge, geometry, Studio Vision; outputs can become digests/quins when needed.  
3. **Zero-heap hot paths** for kernels (caller buffers / fixed scratch); cold construction may use bounded arenas.  
4. **Pixels never in NQuins** — hashes, boxes, descriptors hashes, provenance.  
5. **Epistemic honesty** — classical CV outputs are measurements/proposals, not automatic facts.  
6. **Shared wgpu** for GPU ports; CPU oracle required.  
7. **OpenCV optional only as test oracle** (`opencv-oracle` feature) — never the public ABI.  
8. **Licence-clean defaults** for features (ORB-class; patented algos only with explicit principal decision).  
9. CLAIM/RELEASE; progress log; no silent incomplete.  
10. Prefer **libraryization** (`qualia-vision/src/cv/…`) over one megamonolith.

### 3.1 ABI (Qualia-native, not `cv::Mat`)

```rust
// Illustrative — refine in ADR
pub struct ImageView2D<'a> { /* width, height, stride, format, bytes */ }

pub fn gaussian_blur_u8(
    src: ImageView2D<'_>,
    ksize: u32,
    sigma: f32,
    dst: &mut [u8],
) -> Result<(), CvError>;
```

Public surface is **Qualia types + capability registry**, not OpenCV type names.

### 3.2 Module layout (target)

```text
crates/qualia-vision/src/cv/
  mod.rs          # capability registry + re-exports
  buffer.rs       # ImageBuffer2D, ROI, channel ops
  color.rs
  filter.rs
  morph.rs
  edges.rs
  hist.rs
  transform.rs    # affine, perspective, remap
  contours.rs
  features/       # detect, describe, match
  calib/          # homography, PnP, stereo (phased)
  flow.rs
  photo.rs
  codecs.rs
```

Existing semantic stack (`semantic`, `detector`, `tracker`, `spatial`, …) **stays**; classical ops **feed** it.

---

## 4. Capability checklist (OpenCV modules as *classes*)

Legend for **Qualia capability status**: **Present** · **Partial** · **Missing** · **N/A** · **Beyond** (Qualia has a different, stronger story)

| Capability class (OpenCV name only as label) | Qualia today | Fill-in target |
|----------------------------------------------|--------------|----------------|
| Buffer / arith / ROI / channels (**core**) | Partial | Present — full 2D buffer ops |
| Filters, morph, colour, edges, hist, contours, draw (**imgproc**) | Partial | Present — essentials first |
| Image encode/decode (**imgcodecs**) | Partial | Present — PNG/JPEG (+ WebP as needed) |
| Video file / camera (**videoio**) | Missing/partial | Partial→Present — file first; camera intent-gated |
| Desktop CV windows (**highgui**) | N/A | N/A — Studio is the UI |
| Optical flow / BG sub / classical track (**video**) | Partial (tracker only) | Present — flow + improve tracker |
| Homography, PnP, stereo, calibrate (**calib3d**) | Missing (heightfield only) | Present for product verticals |
| Feature detect/describe/match (**features2d**) | Missing | Present — ORB default |
| Classical detectors / QR (**objdetect**) | Partial (grid head) | Partial→Present as needed |
| Vision network load (**dnn**) | Partial (QVWT seed) | Partial — real weights COMPLETE-WITH-GATE |
| Classical ML (**ml**) | Partial (probe + specialized_libs) | Present enough via platform ML |
| ANN search (**flann**) | Missing | Partial — BF first, ANN later |
| Denoise / inpaint (**photo**) | Missing | Present minimum set |
| Panorama (**stitching**) | Missing | Optional capability |
| Pipeline graph API (**gapi**) | Missing | Optional — Forge schedules |
| Semantic observations / rights | Beyond | Keep leading |
| Computational geometry | Beyond | Keep leading; connect depth/meshes |

**Missing today (fill list):** colour suite, Gaussian/median/bilateral, morphology, Sobel/Laplacian/Canny, contours/CC, histograms, Hough, affine/perspective warps, ORB+match+RANSAC, optical flow, stereo/calib, photo denoise/inpaint, fuller codecs/video I/O.

---

## 5. Integration into “something new” (why not a clone)

Each classical capability, when added, must have an **integration story**:

| Layer | How classical ops plug in |
|-------|---------------------------|
| **Media** | Digests + retention; no raw pixels in graph |
| **Epistemic** | Optional compile of measurements to quins (with model/hash provenance) |
| **Geometry** | Disparity / contours / meshes → MeshIR / computational_geometry |
| **GPU** | Forge ports for hot kernels when justified |
| **Studio** | Vision workbench: run op → show result → reject/correct if model-backed |
| **Library** | Optional catalogue of vision capability packages / weight slots |
| **Rights** | Camera/video capture still intent-gated |

OpenCV stops at Mat arrays. Qualia continues into **governance and integration**. That is the product difference.

---

## 6. Waves (capability fill, not port milestones)

### Wave 0 — Capability ADR + fixtures

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **C0.1** | ADR: native CV as OpenCV-alternative *capability*, not clone | Written |
| **C0.2** | Capability registry stub in `cv/mod.rs` (id, status, honesty) | Queryable list |
| **C0.3** | Synthetic fixtures (edges, checkerboard, warp) | Offline tests |
| **C0.4** | Progress log | Created |

### Wave 1 — Buffer + essential image processing

Colour, blur/median/bilateral, morph, edges, hist, contours, draw, ROI/channel ops.  
**Exit:** product can do classical preprocess entirely in-tree.

### Wave 2 — Codecs + capture

PNG/JPEG (WebP optional); video file path; camera with intent/consent.  
**Exit:** no need for OpenCV imgcodecs/videoio for those paths.

### Wave 3 — Features + geometric transforms

ORB + match + RANSAC homography; warp affine/perspective.  
**Exit:** register/align documents without vendor OpenCV.

### Wave 4 — Calib / stereo / depth handoff

Zhang-class calib, PnP, stereo matching lite → MeshIR / geometry.  
**Exit:** twin/recon vertical beyond heightfield-only.

### Wave 5 — Motion

LK / dense flow class; BG subtract; tracker uses motion/features.  
**Exit:** video analysis capability present.

### Wave 6 — Photo (+ optional stitch)

Denoise, inpaint; optional 2-image stitch.  
**Exit:** restoration-class ops in-tree.

### Wave 7 — Learned vision (gated)

Licensed/P64/QVWT real backbone; H1 metrics.  
**Exit:** COMPLETE-WITH-GATE closed only with principal weights/corpus.

### Wave 8 — Capability ledger + dogfood

Update CAPABILITY / HANDOVER-style inventory; Studio surfaces; optional OpenCV **oracle** tests only.

---

## 7. Priority

```text
C0 ADR + capability registry
 → C1 essential image processing   ← largest gap for “no vendor OpenCV”
 → C2 codecs / I/O
 → C3 features + warps
 → C4 calib/stereo when product needs depth/pose
 → C5 motion
 → C6 photo/stitch as needed
 → C7 licensed models
 → C8 ledger + UI
```

---

## 8. Success criteria (reframed)

**Success is not:** “We match OpenCV 4.13.”

**Success is:**

1. For each capability class we claim **Present**, there is a **Rust** implementation in the Qualia tree, tested, under Qualia ABI.  
2. Product features that needed that class **do not require vendoring OpenCV**.  
3. Capability registry / inventory reflects reality (Present / Partial / Missing / Beyond).  
4. Integration hooks (media, quins, geometry, Studio) exist where product-facing.  
5. Long tail remains explicitly Missing or N/A — no silent pretend.

---

## 9. Effort honesty

| Wave | Nature |
|------|--------|
| C0–C1 | Foundational; multi-session |
| C2–C3 | High product value |
| C4–C6 | Verticals; prioritise by demand |
| C7 | Human gates (licence, corpora) |
| “All of OpenCV contrib/CUDA” | **Not a goal** of this plan |

---

## 10. Principal decisions

| ID | Ask | Default if silent |
|----|-----|-------------------|
| **CV1** | Fill through C3 first, or jump to C4 stereo/calib? | **C0–C3 first** |
| **CV2** | Allow optional OpenCV **test-only** oracle on some machines? | Yes; never product default |
| **CV3** | Camera I/O priority? | File codecs before live camera |
| **CV4** | Any patented detector required? | No — ORB-class default |

---

## 11. Immediate next step

When Timothy says **execute native-cv** (or **execute C0**):

1. CLAIM exclusive paths under `qualia-vision/src/cv/`.  
2. Land ADR + capability registry + fixtures.  
3. Implement Wave C1 (essential image processing).  
4. Progress log after each wave; update capability inventory.

---

*End of plan. OpenCV = checklist of capability classes. Qualia = native Rust implementation inside an integrated, rights-aware, geometric, multimodal system — an alternative to vendor OpenCV, not a clone.*
