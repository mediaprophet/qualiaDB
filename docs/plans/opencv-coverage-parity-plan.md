# Plan — Classical Computer Vision Coverage vs OpenCV 4.13

**Date:** 2026-07-17  
**Reference:** [OpenCV 4.13.0 docs](https://docs.opencv.org/4.13.0/)  
**Branch:** `0.0.25` · canonical tree only  
**Status:** Ready when Timothy says **execute opencv-parity wave N**  
**Related:** `native-visual-intelligence-and-generative-3d.md`, `qualia-vision`, `computational_geometry`, Forge vision ops  

---

## 0. Short answer

| Question | Answer |
|----------|--------|
| Do we have **equivalent or better** coverage than OpenCV 4.13 overall? | **No.** Not for classical CV (imgproc / features2d / calib3d / video / photo / stitching / CUDA CV). |
| Are we **better** in any relevant dimensions? | **Yes, orthogonally:** semantic NQuin observations, zero-heap ABI, rights/epistemic honesty, multimodal (vision+audio), computational geometry depth, native GPU inference/render stack — none of which is OpenCV’s job. |
| Should we become “OpenCV in Rust”? | **No.** Goal is **Qualia-native CV substrate** that covers the **product-critical** OpenCV surface with our ABI/rules, then **stop** (or feature-gate the long tail). |

This plan is the path from “semantic vision scaffold + geometry engine” to “honest classical CV coverage where the product needs it.”

---

## 1. What OpenCV 4.13 actually is (scope of comparison)

### 1.1 Main modules (must map)

| OpenCV module | Role |
|---------------|------|
| **core** | Arrays, Mat-like buffers, arith, reduce, split/merge |
| **imgproc** | Filters, morph, colour, geometry transforms, contours, hist, drawing |
| **imgcodecs** | Read/write PNG/JPEG/TIFF/… |
| **videoio** | Cameras, file video decode/encode |
| **highgui** | Windows/UI (not a product goal for Qualia) |
| **video** | Optical flow, background sub, object tracking classics |
| **calib3d** | Camera calibration, stereo, pose, PnP, homography |
| **features2d** | Detectors/descriptors (ORB, SIFT*, AKAZE…), matching |
| **objdetect** | Cascade/HOG/QR/barcode-class detectors |
| **dnn** | Generic DNN inference (ONNX-class) |
| **ml** | Classical ML (SVM, trees, …) |
| **flann** | ANN search |
| **photo** | Denoise, inpaint, HDR, seamless clone |
| **stitching** | Panorama pipelines |
| **gapi** | Graph pipeline API |

\* SIFT may be patent-status dependent; plan prefers unencumbered defaults (ORB/AKAZE-class).

### 1.2 Extra modules (selective)

CUDA CV (`cuda*`), RGB-D, SFM, stereo extras, tracking, text, face, superres, ximgproc, … — **huge long tail**. Default stance: **out of scope** unless product demand + exclusive CLAIM.

### 1.3 Explicit non-goals for Qualia

| OpenCV surface | Qualia stance |
|----------------|---------------|
| highgui / cvv interactive windows | Studio/desktop already own UI |
| Python bindings as library API | **No Python in product libraries** |
| Shipping third-party DNN zoo as “ours” | COMPLETE-WITH-GATE on licences |
| Silent OpenCV C++ FFI as the permanent ABI | Optional **cold** backend only; public ABI stays Rust + NQuin |

---

## 2. Honest Qualia inventory (2026-07-17)

### 2.1 `qualia-vision` (product vision path)

| Area | Present | Level |
|------|---------|--------|
| Pixel buffer view / formats | `ImageView`, RGB8 | Partial core |
| Resize / letterbox / normalize | preprocess | Partial imgproc |
| NMS | preprocess | Partial objdetect |
| Conv2d / pool / resize ops | `ops/` + Forge | Tiny dnn/imgproc |
| Detector + tracker | grid head + bounded tracker | Scaffold, not OpenCV trackers |
| Classifier | linear probe | Scaffold |
| Overlay / BMP encode | overlay | Partial drawing/codecs |
| Media store + digests | media_store | Beyond OpenCV |
| Semantic / SPARQL-MM quins | semantic | **Beyond OpenCV** |
| Synthetic data + metrics | synthetic, metrics | Partial datasets |
| Seed QVWT weights | weights | Partial dnn |
| Generative image (ref) | generator | Not OpenCV |
| Image→3D heightfield + MeshIR | spatial | Partial recon, not calib3d |
| Twin / A1 elasticity gate | twin_bridge | Beyond OpenCV |

**Missing (critical classical CV):** colour space suite, Gaussian/median/bilateral filters, morphology, Canny/Sobel/Laplacian, contours/connected components, histograms/equalize, Hough, warps (affine/perspective), feature detect/describe/match, optical flow, stereo, camera calibrate, PnP, dense matching, full video I/O, photo (inpaint/denoise), stitching.

### 2.2 Engine strengths outside OpenCV’s lane

| Area | Qualia |
|------|--------|
| Computational geometry | Very large: Delaunay, Voronoi, CSG, BVH, remesh, Poisson recon, exact kernels, … |
| Linear algebra / ML / stats | specialized_libs (not Mat-centric CV) |
| GPU | wgpu Forge + volumetric render + LLM tensors |
| Audio | `qualia-audio` (OpenCV has almost none) |
| Rights / trust / graph | Platform core |

**Conclusion:** comparing only OpenCV modules understates Qualia’s geometry/AI platform; comparing only Qualia vision understates OpenCV’s classical CV breadth. This plan is about the **CV gap**, not geometry.

---

## 3. Coverage matrix (OpenCV main modules)

Legend: **Done** · **Partial** · **Missing** · **N/A** (not a Qualia goal) · **Beyond** (Qualia stronger / different)

| OpenCV | Qualia today | Target for “product parity” |
|--------|--------------|-----------------------------|
| core | Partial (views, few ops) | Done — `ImageBuffer` + arith + ROI + channel ops, caller-buffered |
| imgproc | Partial (resize, NMS-adjacent) | Done — filters, morph, edges, contours, colour, hist, warp |
| imgcodecs | Partial (BMP out; PNG/JPEG import path in pipeline) | Done — PNG/JPEG/WebP decode/encode via pure-Rust codecs |
| videoio | Missing (desktop mic/cam is audio/browser-adjacent only) | Partial — file decode + camera intent-gated capture |
| highgui | N/A | N/A — Studio surfaces |
| video | Missing (tracker is IoU association only) | Partial — Lucas–Kanade / Farneback-class + BG subtract |
| calib3d | Missing | Partial — homography, PnP, stereo basics, calibrate chessboard |
| features2d | Missing | Done — ORB (default) + matcher + RANSAC homography |
| objdetect | Partial (grid detector) | Partial — QR optional; cascade not required if DNN path |
| dnn | Partial (QVWT seed head; LLM separate) | Partial — ONNX/P64 vision graph load **COMPLETE-WITH-GATE** |
| ml | Partial (linear probe; specialized_libs ML) | Done enough via specialized_libs + vision heads |
| flann | Missing | Partial — KD/BF match first; FLANN-class later |
| photo | Missing | Partial — denoise + inpaint minimum |
| stitching | Missing | Optional wave — feature-based panorama |
| gapi | Missing | Optional — schedule vision graphs on Forge |

**Extra OpenCV CUDA/contrib:** default **out of scope** unless Timothy prioritises a vertical (e.g. stereo robot, ArUco warehouse).

---

## 4. Design principles for Qualia CV (non-negotiable)

1. **No Python** in libraries / product path.  
2. **Zero-heap hot paths** for infer/filter kernels: caller supplies `&mut [T]` / fixed scratch.  
3. **Pixels never in NQuins** — digests, boxes, descriptors hashes, provenance only.  
4. **Epistemic honesty** — detector/flow outputs are proposals, not facts.  
5. **Shared wgpu only** for GPU path; CPU oracles mandatory.  
6. **No silent OpenCV C++ dependency** as the public ABI; if an optional `opencv-sys` backend appears, it is **cold, feature-gated, honesty-labelled**.  
7. Prefer **pure Rust** first (image, ndarray-free or bounded, custom kernels).  
8. **Feature-detect legal/licence** before shipping patented algos; default ORB/AKAZE-class.  
9. CLAIM/RELEASE exclusive files; progress log per wave.  
10. Measurement honesty: “OpenCV parity” means **listed acceptance tests**, not marketing.

### 4.1 ABI sketch (core)

```rust
// Illustrative — refine in ADR
#[repr(C)]
pub struct ImageView2D<'a> { /* width, height, stride, format, bytes: &'a [u8] */ }

pub fn gaussian_blur_u8(
    src: ImageView2D<'_>,
    ksize: u32,
    sigma: f32,
    dst: &mut [u8],
) -> Result<(), CvError>;
```

Stack-local kernels where possible; large scratch via `geometry_workspace`-style arenas for cold builds.

### 4.2 Module layout (target)

```text
crates/qualia-vision/src/
  cv/                 # NEW classical CV substrate
    mod.rs
    buffer.rs         # ImageBuffer2D, ROI, formats
    color.rs
    filter.rs
    morph.rs
    edges.rs
    hist.rs
    transform.rs      # affine, perspective, remap
    contours.rs
    features/         # ORB + match
    calib/            # homography, PnP, stereo (phased)
    flow.rs           # optical flow (phased)
    photo.rs          # denoise, inpaint (phased)
    codecs.rs         # wrap pure-Rust encode/decode
  ... existing semantic stack unchanged
```

Do **not** dump into one 5k-line file (project libraryization rule).

---

## 5. Programme waves

### Wave 0 — ADR + inventory freeze

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **O0.1** | ADR: Qualia CV vs OpenCV scope + non-goals | Written, principal-visible |
| **O0.2** | Fixture pack: synthetic edges, checkerboard, stereo pair (generated) | Offline tests |
| **O0.3** | Progress log `opencv-parity-PROGRESS-LOG.md` | Created |

### Wave 1 — Core + imgproc essentials (highest leverage)

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **O1.1** | `ImageBuffer2D` + ROI + convert RGB/Gray/RGBA | Round-trip tests |
| **O1.2** | Colour: RGB↔Gray, RGB↔HSV/YCrCb | Known colour fixtures |
| **O1.3** | Filters: box, Gaussian, median, bilateral (CPU) | Impulse response tests |
| **O1.4** | Morphology: erode, dilate, open, close | Binary blob fixtures |
| **O1.5** | Edges: Sobel, Laplacian, Canny | Synthetic step-edge |
| **O1.6** | Hist + equalize + CLAHE (optional) | Uniform vs peaked hist |
| **O1.7** | Contours: findContours + approxPolyDP (subset) | Known shapes |
| **O1.8** | Drawing: line, rect, circle, polyline on RGBA | Pixel change tests |
| **O1.9** | Wire Forge ports for blur/Sobel **or** document CPU-only | Naga + oracle if GPU |

**Exit:** “imgproc essential suite green” — product can preprocess like a classical CV stack.

### Wave 2 — Codecs + I/O

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **O2.1** | Decode PNG/JPEG (image crate or existing pipeline) → `ImageBuffer2D` | File fixtures |
| **O2.2** | Encode PNG/JPEG | Round-trip PSNR threshold |
| **O2.3** | Video file decode (one container path) | Optional feature; honesty on codecs |
| **O2.4** | Camera capture intent gate (reuse audio capture norms) | Fail closed without consent |

**Exit:** Offline images fully; live video **Partial**.

### Wave 3 — Features2d + geometry transforms

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **O3.1** | ORB detect + describe (CPU) | Reproducible on synthetic corners |
| **O3.2** | Brute-force Hamming match + ratio test | Match count sanity |
| **O3.3** | RANSAC homography / findHomography | Synthetic plane warp |
| **O3.4** | WarpPerspective / WarpAffine | Grid warp tests |
| **O3.5** | (Optional) AKAZE or FAST+BRIEF | Licence-clean |

**Exit:** Register / stitch-prep / document alignment possible without OpenCV.

### Wave 4 — Calib3d / stereo (product vertical)

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **O4.1** | Chessboard detect + calibrateCamera (Zhang) | Synthetic board reprojection error bound |
| **O4.2** | solvePnP | Known pose fixture |
| **O4.3** | Stereo BM or SGBM-lite | Synthetic disparity smoke |
| **O4.4** | Hand-off disparity → MeshIR / `.10d` | Link to existing spatial path |

**Exit:** “robot / twin / recon” vertical beyond heightfield-only.

### Wave 5 — Video analysis

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **O5.1** | Lucas–Kanade sparse flow | Synthetic translation |
| **O5.2** | Farneback dense flow (or simplified) | Direction smoke |
| **O5.3** | Background subtract (MOG-lite or KNN-lite) | Static cam fixture |
| **O5.4** | Upgrade tracker to use flow/features (not IoU-only) | Multi-object sequence |

### Wave 6 — Photo + optional stitch

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **O6.1** | Fast NlMeans or bilateral-heavy denoise | Noise reduction metric |
| **O6.2** | Telea/Navier–Stokes inpaint subset | Hole fill smoke |
| **O6.3** | Feature stitch prototype | 2-image panorama |

### Wave 7 — DNN bridge (COMPLETE-WITH-GATE)

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **O7.1** | Document QVWT/P64/ONNX roles vs OpenCV dnn | ADR |
| **O7.2** | Load one real vision backbone under licence | Human gate |
| **O7.3** | Detector head on real embeddings | H1 metrics split |

### Wave 8 — Hardening

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| **O8.1** | Benchmark suite vs reference (optional OpenCV **test-only** oracle behind feature) | Numbers + caveats |
| **O8.2** | WASM-safe subset feature flags | `wasm-ontology` still builds |
| **O8.3** | Studio Vision workbench exposes key ops | Dogfood |

---

## 6. Optional: OpenCV as **test oracle** only

| Allowed | Forbidden |
|---------|-----------|
| Feature `opencv-oracle` in **dev-dependencies** / ignored CI for golden diffs | Default product link to OpenCV |
| Compare Canny/ORB outputs on fixtures | Re-export OpenCV types as Qualia ABI |
| Document max pixel error | Claim “we are OpenCV” because FFI exists |

---

## 7. Priority order (recommended)

```text
O0 ADR/fixtures
 → O1 imgproc essentials     ← largest product gap
 → O2 codecs
 → O3 features2d + warps
 → O4 calib3d/stereo (if twin/robot demand)
 → O5 video analysis
 → O6 photo/stitch (as needed)
 → O7 licensed DNN
 → O8 hardening
```

**Do not** start CUDA contrib ports, highgui, or full SFM until O1–O3 are green.

---

## 8. Effort honesty

| Wave | Order-of-magnitude (agent sessions) | Risk |
|------|--------------------------------------|------|
| O0 | 0.5 | Low |
| O1 | several | Medium (Canny/contours quality) |
| O2 | 1–2 | Codec edge cases |
| O3 | several | ORB correctness |
| O4 | multi-session | Numerical calib stability |
| O5–O6 | multi-session | Quality vs OpenCV |
| O7 | human-gated | Licence + weights |
| Full OpenCV surface | **years / multi-team** | Do not claim |

“Better than OpenCV 4.13 overall” is **not** the exit criterion.  
Exit criterion: **matrix in §3 is Done or Partial for product-needed rows**, with honesty labels.

---

## 9. What would make us “better” than OpenCV (already true / keep)

| Dimension | Keep investing |
|-----------|----------------|
| Semantic graph of observations | Vision quins + SPARQL-MM |
| Rights, consent, epistemic reject/correct | Existing paths |
| Zero-heap / edge / WASM discipline | ABI rules |
| Geometry + FEA honesty tiers | computational_geometry + twin |
| Multimodal time alignment | Shared clock with audio |
| Local LLM + Sentinel governance | Not OpenCV |

CV waves must **plug into** these, not replace them with Mat-only pipelines.

---

## 10. Immediate next step

When Timothy says **execute opencv-parity** (or **execute O0**):

1. CLAIM in `coordination/NOTICES.md`.  
2. Create `docs/plans/opencv-parity-PROGRESS-LOG.md`.  
3. Land ADR + fixtures (O0).  
4. Implement Wave 1 (`cv/` under `qualia-vision`) with tests.  
5. RELEASE + progress entry per wave.

---

## 11. Decision asks (principal)

| ID | Ask | Default if silent |
|----|-----|-------------------|
| **CV1** | Target: product CV suite (O1–O3) or full calib/stereo (O4+)? | O1–O3 first |
| **CV2** | Allow optional `opencv-oracle` test feature on machines with OpenCV installed? | Yes, never default |
| **CV3** | Camera videoio priority vs file-only? | File first |
| **CV4** | Any patented detector (classic SIFT) desired? | No — ORB default |

---

*End of plan. Honest gap: classical CV. Plan: Qualia-native substrate O0→O8, not a wholesale OpenCV clone.*
