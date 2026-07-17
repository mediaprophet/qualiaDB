# Plan — Native Vision Capability Excellence (2026 product)

**Date:** 2026-07-17  
**Ambition:** Ship the **best classical + biosensing + integrated vision capability set** for a **2026 all-Rust, rights-aware, local-first** product — **excellence, not “it’ll do.”** Not OpenCV parity; not a clone.  
**Floor checklist:** [OpenCV 4.13.0](https://docs.opencv.org/4.13.0/) and peer classical CV surfaces (no capability holes).  
**Ceiling verticals:** rPPG/pulse, Eulerian micro-change magnification, selfhood-grade biometrics, scientifically honest affect, multimodal fusion.  
**Branch:** `0.0.25` · canonical tree only  
**Status:** Ready when Timothy says **execute vision-excellence** or **execute VX0**  
**Supersedes:** `opencv-coverage-parity-plan.md` (retired name)  
**Related:** `native-visual-intelligence-and-generative-3d.md`, `qualia-vision`, geometry, Forge, audio, Wellfair biometrics RDF, agency-domain selfhood  

---

## 0. North star

| | |
|--|--|
| **Product outcome** | Vision that can preprocess, measure, detect, track, **biosense**, **amplify micro-change**, calibrate, reconstruct, and explain — **in-process, pure Rust**, same governance as graph, identity, Library, sanctuary, LLM. |
| **vs vendor OpenCV / cloud biometrics** | Qualia **implements**; no required OpenCV C++/Python; no required cloud face/emotion API. |
| **vs “parity” / “it’ll do”** | Classical CV classes = **floor**. Biosensing/affect = **research-grade + rights-grade** or not shipped as product claims. |
| **What “best” means** | Best *integrated local-first human-rights-centric 2026 system*: coverage + SNR/confidence + consent/selfhood + epistemic honesty — not a carnival BPM or a silent emotion %. |

If a capability is only “call OpenCV / call cloud face API,” it is **not** a Qualia capability yet.

---

## 1. Purpose (principal framing)

| What this is | What this is not |
|--------------|------------------|
| Grow **Qualia capabilities** (classical + **biosense**) in-tree Rust | OpenCV port or API clone |
| Alternative to vendoring OpenCV **or** cloud biometrics SDKs | Competing for OpenCV’s brand |
| Fill capability holes → **exceed** via integration + biosensing excellence | Bit-exact OpenCV matching; demo-grade toys |
| Part of **something new** (graph, rights, selfhood, geometry, multimodal) | Mat-only research script library |

**Floor sources:** OpenCV-class classical ops.  
**Ceiling sources:** rPPG, EVM, biometrics under selfhood, honest affect, multimodal, no cloud, no Python.

---

## 2. Honest starting point (2026-07-17)

### 2.1 Already strong (beyond classical CV vendors)

- Semantic vision: NQuins, reject/correct, SPARQL-MM-class queries  
- Media digests, seed QVWT, detector/tracker scaffolds  
- Image→3D heightfield + MeshIR + twin A1 honesty  
- Computational geometry depth; Forge/wgpu; local LLM  
- `qualia-audio` + shared media clock  
- Wellfair vault biometric RDF hooks; agency-domain **selfhood** for biometrics  

### 2.2 Gaps

**Classical floor:** colour, filters, morph, edges, contours, hist, warps, ORB-class features, flow, calib/stereo, codecs/video I/O, denoise/inpaint.  

**Biosense excellence (required for ambition, not optional polish):** face mesh, multi-ROI rPPG, Eulerian/Lagrangian magnification, liveness, encrypted face templates, affective **proposals** with uncertainty — **all currently missing as vision capabilities.**

**Integration excellence:** every product-facing result → digests, optional quins, Studio, geometry, Wellfair, consent audit.

---

## 3. Design principles

1. **All product path in Rust** — no Python; no required OpenCV or cloud face API.  
2. **Integrated environment** — media, epistemic graph, Forge, geometry, Studio, Library, Wellfair.  
3. **Zero-heap hot paths** for kernels; cold arenas for construction.  
4. **Pixels and raw biometric templates never in NQuins** — digests, scores, uncertainty, provenance; templates sanctuary-class only.  
5. **Epistemic honesty** — especially affect and biometrics: proposals ≠ facts.  
6. **Shared wgpu**; CPU oracles for claimed kernels.  
7. **OpenCV optional test oracle only.**  
8. **Licence-clean defaults** (ORB-class; patented algos only with principal decision).  
9. **Capability registry** is truth for “do we have this?”  
10. **Exceed via composition:** filter → mesh → rPPG/EVM → graph → human attestation.  
11. **No silent incomplete; no “it’ll do”** on biosensing claims.  
12. Libraryization: `cv/` + **`biosense/`**.  
13. **Selfhood bar** for biometrics (agency-domain reproductive/biometric/genetic).

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
crates/qualia-vision/src/
  cv/                 # classical capability classes
    mod.rs            # capability registry
    buffer.rs, color.rs, filter.rs, morph.rs, edges.rs, hist.rs
    transform.rs, contours.rs, features/, calib/, flow.rs, photo.rs, codecs.rs
  biosense/           # excellence vertical — biometrics, affect, micro-change
    mod.rs            # registry + consent gates
    face_mesh.rs      # landmarks / mesh (not identity claim by default)
    rppg.rs           # remote photoplethysmography (pulse)
    magnification.rs  # Eulerian / motion magnification of micro-changes
    biometrics/       # template extract/compare under sanctuary policy
    affect.rs         # affective proposals + uncertainty (never silent fact)
    liveness.rs       # anti-spoof / presentation attack detection
    quality.rs        # face/frame quality for biosignal validity
    fusion.rs         # multimodal (vision + audio physiology cues)
    excellence.rs     # composed clinical-honest pipelines
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
| Denoise / inpaint | Missing | Present + honesty |
| Stitch / panorama | Missing | Optional vertical |
| Pipeline scheduling | Partial Forge | Explicit vision schedules |

### 4.2 Excellence vertical — biosensing, micro-change, biometrics, affect

**Bar: excellence, not scaffold.** Every row below must ship with consent gates, quality metrics, uncertainty, synthetic+real eval slots, and fail-closed behaviour when signal quality is insufficient.

| Capability class | Status now | Excellence target (2026 product) |
|------------------|------------|----------------------------------|
| **Face landmarks / mesh** | Missing | Dense landmarks + temporal track; quality score; not “identity” unless biometric mode on |
| **Remote PPG (pulse / HR)** | Missing | Multi-ROI rPPG (POS/CHROM-class + spectral peak); HR + HRV proxies; SNR / confidence; motion rejection |
| **Respiration from video** | Missing | Motion- or colour-derived respiratory rate with confidence |
| **Eulerian video magnification** | Missing | Colour + motion magnification of micro-changes (pulse visualisation, micro-tremor) with stability controls |
| **Lagrangian / feature-path magnification** | Missing | Track-then-amplify path for structured micro-motion |
| **Biometric face template** | Partial (vault RDF elsewhere) | Extract/compare under **selfhood** policy; template encryption; no silent secondary use |
| **Multi-modal biometrics** | Partial (audio path separate) | Face + voice (+ optional gait) fusion **only** with explicit purpose binding |
| **Liveness / PAD** | Missing | Presentation-attack detection; fail closed on spoof suspicion |
| **Affective sensing** | Missing | Valence/arousal or discrete classes as **proposals** with uncertainty; literature-honest limits; no “truth” UI default |
| **Micro-expression / AU-lite** | Missing | Action-unit or temporal micro-event proposals; not courtroom-grade claims |
| **Biosignal graph compile** | Missing | NQuins: observation + confidence + method hash + consent context; sanctuary routing |
| **Wellfair / health handoff** | Partial vault | Optional export to Wellfair with purpose + sensitivity |

### 4.3 Ceiling — exceed the market (Qualia-only)

| Excellence capability | Why it beats a vendor CV lib / cloud biometrics SDK in 2026 |
|----------------------|--------------------------------------------------------------|
| **Epistemic graph** | Biosignals and affect are **proposals** with reject/correct; not sticky labels |
| **Selfhood / sanctuary** | Biometric templates under highest agency-domain bar; local-only by default |
| **Consent as code** | Capture + process + store each require intent; revoke stops use |
| **Clinical-honest metrics** | SNR, confidence, failure reasons — not a single magic “emotion %” |
| **Micro-change amplification** | Research-grade EVM-class viz **and** quantitative rPPG in one stack |
| **Geometry continuum** | Face mesh / depth → MeshIR when useful for twins |
| **Multimodal** | Shared clock with audio (voice stress/physiology cues) |
| **Local GPU + no cloud** | Same process; no silent off-device face API |
| **Library / rights audit** | Who processed biometrics, when, purpose, cost vectors |

**Market “best” claim** requires **floor + biosense excellence + integration** — all three.

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
**Note:** feeds biosense (stable ROIs, motion rejection for rPPG).

### VX6 — Photo + optional stitch

Denoise, inpaint; optional panorama.  
**Exit:** restoration-class ops in-tree.

### VX7 — Learned vision (gated)

Licensed backbone; QVWT/P64/ONNX policy; H1 metrics split synthetic/real.  
**Exit:** COMPLETE-WITH-GATE only with principal assets.  
**Note:** face mesh / affect heads may load here under consent.

### VXB — Biosensing excellence suite (**core excellence vertical, not optional polish**)

Objective: **best-in-class local biosensing stack for a 2026 rights-centric product** — quantitative, consent-bound, uncertainty-aware. Not a carnival pulse toy.

#### VXB0 — Policy + consent + quality gates

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| VXB0.1 | ADR: biosense purpose binding, sanctuary default, selfhood domain | Written |
| VXB0.2 | `BiosenseConsent` / purpose enum (wellfair self-monitor / research / security) | Fail closed without grant |
| VXB0.3 | Frame quality metrics (blur, lighting, face fraction, motion energy) | Reject low-quality windows with reason codes |
| VXB0.4 | Audit: who/when/purpose/method_hash for each biosense run | JSONL + optional quins |

#### VXB1 — Face mesh / landmarks (foundation for everything else)

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| VXB1.1 | Real-time face detect + **68+ or mesh** landmarks (Rust; weights licence-gated) | Temporal stability tests |
| VXB1.2 | Multi-face handling + primary subject selection under consent | Explicit multi-person policy |
| VXB1.3 | Landmark track with dropout recovery | Synthetic + real-slot eval |

#### VXB2 — Remote PPG (pulse) — excellence bar

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| VXB2.1 | Multi-ROI skin segmentation (cheeks / forehead) | ROI quality scores |
| VXB2.2 | At least two rPPG algorithms (e.g. POS + CHROM or equivalent) + ensemble | Published method hashes |
| VXB2.3 | Spectral peak HR estimate + confidence / SNR | Synthetic PPG-modulated video fixtures |
| VXB2.4 | HRV-class proxies only when window/SNR sufficient | Fail closed with reason |
| VXB2.5 | Motion & illumination artefact rejection | Stress fixtures |
| VXB2.6 | Optional contact PPG compare harness (principal device) | COMPLETE-WITH-GATE until principal supplies |

**Excellence:** report **confidence and method**, not a naked BPM. Prefer silent refusal over wrong pulse.

#### VXB3 — Micro-change amplification (see the invisible)

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| VXB3.1 | **Eulerian** colour magnification (pulse-visible skin) | Synthetic sinusoidal colour fixture |
| VXB3.2 | **Eulerian** motion magnification (micro-tremor / breathing) | Synthetic sub-pixel motion fixture |
| VXB3.3 | Stability: attenuate noise explosion; clamp gain; artefact flags | Documented limits |
| VXB3.4 | Optional Lagrangian path (track + amplify) | Compare to Eulerian on fixtures |
| VXB3.5 | Studio: side-by-side original vs magnified + parameter honesty | Dogfood |

**Excellence:** research-grade controls (spatial pyramid / temporal bandpass design documented), not a single fixed filter demo.

#### VXB4 — Respiration & derived physiology

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| VXB4.1 | Respiratory rate from chest/shoulder motion or rPPG harmonics | Confidence-gated |
| VXB4.2 | Fusion with audio breath cues when consented | Multimodal clock |

#### VXB5 — Biometrics (selfhood-grade)

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| VXB5.1 | Face embedding extract → **encrypted template** store | Never plain graph |
| VXB5.2 | 1:1 verify / 1:N identify only with purpose + threshold + audit | Deny by default |
| VXB5.3 | Liveness / PAD (texture + challenge or passive) | Spoof fixtures fail closed |
| VXB5.4 | Voice biometric path via `qualia-audio` under same consent model | Cross-crate policy |
| VXB5.5 | Template revoke / rotate | Cryptographic erase path |

**Excellence:** security properties + UX that does **not** reintroduce WebID-TLS-style nag; decisions are pin/purpose based.

#### VXB6 — Affective sensing (scientific honesty required)

| Slice | Deliver | Acceptance |
|-------|---------|------------|
| VXB6.1 | Affect model outputs **valence/arousal** and/or discrete classes as **proposals** | Uncertainty always shown |
| VXB6.2 | Explicit **non-claims** UI: not diagnosis, not courtroom truth, culture-sensitive limits | Copy + machine flags |
| VXB6.3 | Optional AU-lite / temporal micro-event detector | Separate from “emotion label” |
| VXB6.4 | Human reject/correct path into graph | Same pattern as vision detector |
| VXB6.5 | Eval slots: synthetic + principal-approved corpora only | No scraped faces |

**Excellence:** best-in-class means **best-governed and best-calibrated**, not highest claimed accuracy on a marketing slide. Overclaiming is a product failure.

#### VXB7 — Integration recipes

| Recipe | Pipeline |
|--------|----------|
| **Self-monitor pulse** | consent → quality → face mesh → rPPG → BPM+conf → Wellfair optional → audit |
| **See my pulse** | consent → EVM colour mag → side-by-side → no identity claim |
| **Sanctuary unlock assist** | consent security purpose → liveness → 1:1 verify → fail closed |
| **Affect journal (opt-in)** | consent research/wellfair → proposals only → human confirm |

### VX8 — Excellence composition + product surface

| Slice | Deliver |
|-------|---------|
| VX8.1 | End-to-end recipes: *import → filter → feature → pose → mesh → quins → Studio* |
| VX8.2 | Biosense recipes VXB7 in Studio / Wellfair |
| VX8.3 | Capability ledger Present/Partial/Missing/Beyond for **cv + biosense** |
| VX8.4 | Optional OpenCV **oracle** feature for classical regression only |
| VX8.5 | WASM/edge: declared subset (e.g. rPPG offline window; no heavy PAD) |
| VX8.6 | Rights/audit dogfood: biometric and affect processing appear in liability/cost trails when wired |

**Exit for “2026 product excellence”:**  
floor classical classes Present (or N/A); **VXB pulse + magnification + consent-grade biometrics + honest affect** dogfoodable; inventory honest; **no vendor OpenCV / no cloud face API** on product default path.

---

## 6. Priority

```text
VX0 registry/ADR (include biosense capability rows)
 → VX1 essential image processing
 → VX2 codecs/I/O + camera consent
 → VX3 features + warps
 → VXB0 consent/quality  ┐
 → VXB1 face mesh        ├─ biosense excellence (parallel once VX1–2 solid)
 → VXB2 rPPG             │
 → VXB3 magnification    ┘
 → VX5 motion (feeds rPPG stability) interleaved as needed
 → VXB4 respiration / VXB5 biometrics / VXB6 affect
 → VX4 calib/stereo when depth/pose demanded
 → VX6 photo · VX7 licensed models
 → VX8 ledger + UI + recipes
```

**Biosensing is not deferred to “later polish.”** It is a **primary excellence track**, started as soon as basic buffers/colour/camera quality exist.

---

## 7. Success criteria (excellence, not “it’ll do”)

**Success is:**

1. Capability registry shows **Present** for every class we claim for the 2026 product — **including biosense rows we advertise**.  
2. Those classes are **Rust, in-tree**, tested, with **documented error/confidence behaviour**.  
3. Product features **do not require** vendoring OpenCV or cloud biometrics APIs.  
4. Excellence pipelines run end-to-end (classical **and** biosense recipes).  
5. Biometrics respect **selfhood / sanctuary / purpose binding**; affect never presents as silent fact.  
6. rPPG and magnification report **confidence / SNR / failure reasons**.  
7. Honesty labels everywhere (method hash, synthetic vs real eval, weight licence).

**Failure modes to avoid:**

- Calling the work “parity,” “clone,” or shipping a **demo-grade** pulse/emotion toy as product.  
- Biometrics without consent, liveness, or revoke.  
- Emotion labels without uncertainty and non-claims.  
- Kernels without integration into graph/Studio/Wellfair.  
- Pulling OpenCV or cloud face APIs into the product default path.

---

## 8. Effort honesty

| Wave | Note |
|------|------|
| VX0–VX1 | Foundation; multi-session |
| VX2–VX3 | High product leverage |
| **VXB0–VXB3** | **Large** — research-grade rPPG + EVM is multi-session excellence work |
| VXB5–VXB6 | Security + ethics + model gates |
| VX4, VX6 | Verticals by demand |
| VX7 | Human licence/corpus gates |
| Full OpenCV CUDA-contrib | Not required for excellence claim |

Excellence is **integrated completeness + measurement quality + rights**, not “every OpenCV contrib module.”

---

## 9. Principal decisions

| ID | Ask | Default if silent |
|----|-----|-------------------|
| **VX-D1** | Fill through VX3 first for classical floor? | **Yes** |
| **VX-D2** | Optional OpenCV test oracle? | Yes; never product default |
| **VX-D3** | Camera vs file I/O first? | File first; camera for biosense dogfood soon after |
| **VX-D4** | Patented detectors? | No — ORB-class default |
| **VX-D5** | Public “best in class 2026” marketing claim timing? | Only after VX8 + VXB2/VXB3 ledger green |
| **VX-B1** | Affective sensing on by default in product? | **No** — opt-in purpose only |
| **VX-B2** | Biometric unlock as security feature in first excellence ship? | Principal call; if yes, liveness mandatory |
| **VX-B3** | Contact PPG / clinical device for rPPG validation corpus? | COMPLETE-WITH-GATE until supplied |
| **VX-B4** | Multi-person biosense in frame? | Default: primary subject only; others require explicit consent |

---

## 10. Immediate next step

When Timothy says **execute vision-excellence** or **execute VX0**:

1. CLAIM `qualia-vision/src/cv/` **and** register `biosense/` capability rows in the registry (even if stub).  
2. Land ADR (classical + biosense selfhood/consent) + fixtures + progress log.  
3. Implement VX1; stand up VXB0 consent/quality in parallel when buffers exist.  
4. After each wave: update registry + progress log + tests — **no silent “it’ll do.”**

---

*End of plan. Floor = classical capability classes. Excellence = biosensing + micro-change + consent-grade biometrics + honest affect + full platform integration. Implementation = pure Rust native, never a vendor clone, never a carnival demo.*
