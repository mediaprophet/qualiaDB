# Plan — Native Vision Capability Excellence (2026 product)

**Date:** 2026-07-17 (expanded: selfhood biometrics, surveillance policy, 3D manufacture, biology, full domain map)  
**Ambition:** Best-in-class **2026** vision + biosensing + spatial manufacture + bio-imaging capabilities, **all in Rust**, inside Qualia’s integrated rights-aware environment — **excellence, not “it’ll do.”**  
**Not:** OpenCV clone, vendor OpenCV, cloud face/emotion APIs as product default, demo-grade biosense toys.  
**Branch:** `0.0.33` · canonical tree only  
**Status:** Ready when Timothy says **execute vision-excellence** / **execute VX0**  
**Related:**  
- Vision/audio: `native-visual-intelligence-and-generative-3d.md`, `qualia-vision`, `qualia-audio`  
- Selfhood: `selfhood-personhood-content-taxonomy.md`, agency-domain selfhood, Wellfair biometrics RDF  
- Geometry/engineering: `computational_geometry`, `engineering_analysis`, `physics_simulation`  
- Medical/bio: `medical_computing`, Anatomy QApp, chemistry_modeling  
- Query: SPARQL-MM (vision/audio), SPARQL federation / federated endpoints  

---

## 0. North star

| Pillar | 2026 product outcome |
|--------|----------------------|
| **Own your body-signal** | A person holds and governs **their own biometrics** and **mindware** as an **inalienable extension of self** — not as platform inventory. |
| **Policy over surveillance** | Rules about CCTV / camera use of biometrics are **expressible and enforceable** via graph query (SPARQL-MM, SPARQL-FED, deontic norms) — the person is not a passive data source. |
| **See and make** | From light → measure → **3D artefacts** (print/CAD/training) with engineering-honest validation where claimed. |
| **Understand life systems** | Biology/clinical imaging and structure share the same media → mesh → graph → rights path. |
| **Classical CV floor** | No capability holes vs classical CV *classes* (OpenCV as checklist only). |
| **Integrated excellence** | One process: Rust, zero-heap hot paths, epistemic honesty, wgpu, Library, sanctuary — exceed standalone CV SDKs. |

If a capability only exists as “call vendor SDK / cloud face API / OpenCV,” it is **not** a Qualia capability yet.

---

## 1. Foundational doctrine: mindware, biometrics, inalienability

### 1.1 Doctrine (product law, not marketing)

| Principle | Operational consequence |
|-----------|-------------------------|
| **Biometrics are selfhood** | Templates, embeddings, rPPG time-series, face mesh parameters used for identity or health — **selfhood class** (see `selfhood-personhood-content-taxonomy.md` and agency-domain `reproductive_biometric_genetic`). |
| **Mindware is selfhood** | Models, weights, agents, preferences that act *as* or *for* the person — same bar when they embody or unlock the person. |
| **Inalienable extension of self** | System **cannot** treat biometrics/mindware as alienable platform assets; secondary use requires **manifest purpose** + consent; revoke stops use. |
| **Manifestation boundary** | Selfhood → personhood only by the principal’s act of disclosure (taxonomy design note). |
| **Surveillance is not the default consumer** | CCTV / ambient cameras may **query whether processing is permitted**; they do not silently own the face. |

### 1.2 Permissioning surveillance and CCTV-class systems

| Mechanism | Role |
|-----------|------|
| **Local biometric vault** | Templates encrypted; never plain NQuin payloads. |
| **Policy graph** | Deontic + purpose + time + place + camera_id + biometric_class norms as quins. |
| **SPARQL-MM** | Region/time/media queries over *observations* (“faces detected in zone Z at t”) without exporting raw biometrics. |
| **SPARQL-FED** | Federated query across nodes/pods: *does this camera’s intended biometric use satisfy principal policy?* |
| **Deontic evaluation** | OBLIGATE / PERMIT / FORBID on processing acts (e.g. `forbid biometric identification without purpose P`). |
| **Fail closed** | Unknown camera, expired consent, or missing policy → **no process** (or process only non-identifying aggregates if policy allows). |
| **Audit** | Who / when / why / what / where / **cost** on every biometric evaluation. |

**Excellence:** a person can publish (or hold private) a **biometric use policy**; compliant systems check it before identification/enrolment/tracking — not after the fact.

### 1.3 Capability rows (selfhood governance)

| Capability | Excellence target |
|------------|-------------------|
| Biometric self-vault | Create/rotate/revoke templates under sanctuary |
| Purpose-bound unlock | Security / wellfair / research enums; deny by default |
| Policy authoring UI + N3/deontic compile | Principal writes rules once |
| SPARQL-MM observation query without template leak | Query returns counts/proposals, not templates |
| SPARQL-FED policy check API | Remote camera node asks; local node answers permit/deny + reason |
| CCTV compliance mode | Camera pipeline runs only policy-allowed stages (e.g. motion only, no face embed) |
| Mindware binding | Local agent/model may not load face unlock without same selfhood gate |

---

## 2. Extensive capability domain map

Use this as the **inventory spine**. Status: **Missing** / **Partial** / **Present** (as of 2026-07-17).  
**Target** for 2026 excellence ship is Present or honest COMPLETE-WITH-GATE.

### D1 — Classical vision (floor)

| ID | Capability | Now | Target |
|----|------------|-----|--------|
| D1.01 | 2D buffer / ROI / channels / arith | Partial | Present |
| D1.02 | Colour spaces, hist, equalize | Missing | Present |
| D1.03 | Filters (Gauss, median, bilateral, …) | Missing | Present |
| D1.04 | Morphology | Missing | Present |
| D1.05 | Edges (Sobel, Canny, …) | Missing | Present |
| D1.06 | Contours / CC / shape moments | Missing | Present |
| D1.07 | Geometric warps (affine, perspective, remap) | Missing | Present |
| D1.08 | Features (ORB-class) + match + RANSAC | Missing | Present |
| D1.09 | Optical flow + BG subtract | Missing | Present |
| D1.10 | Codecs PNG/JPEG/WebP | Partial | Present |
| D1.11 | Video file I/O | Missing | Present |
| D1.12 | Camera capture (intent-gated) | Partial (audio mic pattern) | Present |
| D1.13 | Drawing / overlay | Partial | Present |
| D1.14 | Photo denoise / inpaint | Missing | Present |
| D1.15 | Stitch / panorama | Missing | Optional Present |

### D2 — Learned vision

| ID | Capability | Now | Target |
|----|------------|-----|--------|
| D2.01 | Detector + tracker + NMS | Partial | Present + production weights gated |
| D2.02 | Semantic quins + reject/correct | Present | Present |
| D2.03 | QVWT / P64 vision load | Partial seed | Present + licence gate |
| D2.04 | Segmentation (instance/semantic) | Missing | Present |
| D2.05 | Depth estimation (monocular) | Missing | Present |
| D2.06 | Pose (body/hand) | Missing | Present |
| D2.07 | OCR / scene text | Missing | Present when product needs |
| D2.08 | Generative image (local) | Partial ref | Present + honesty |

### D3 — Biosensing & micro-change (**excellence vertical**)

| ID | Capability | Now | Target |
|----|------------|-----|--------|
| D3.01 | Face landmarks / mesh | Missing | Present |
| D3.02 | Frame/ROI quality gating | Missing | Present |
| D3.03 | Multi-ROI rPPG (HR + conf/SNR) | Missing | Present |
| D3.04 | HRV proxies (window-gated) | Missing | Present |
| D3.05 | Respiration from video | Missing | Present |
| D3.06 | Eulerian colour magnification | Missing | Present |
| D3.07 | Eulerian motion magnification | Missing | Present |
| D3.08 | Lagrangian micro-motion amplify | Missing | Present |
| D3.09 | Liveness / PAD | Missing | Present |
| D3.10 | Face biometric template vault | Partial RDF | Present + crypto |
| D3.11 | Voice biometric (audio crate) | Partial speech path | Present under same policy |
| D3.12 | Multimodal bio fusion | Missing | Present |
| D3.13 | Affect proposals + uncertainty | Missing | Present (opt-in) |
| D3.14 | AU-lite / micro-event proposals | Missing | Present |
| D3.15 | Biosignal → graph (no raw template) | Missing | Present |
| D3.16 | Contact PPG validation harness | Missing | COMPLETE-WITH-GATE |

### D4 — Selfhood policy & surveillance governance

| ID | Capability | Now | Target |
|----|------------|-----|--------|
| D4.01 | Purpose-bound biosense consent | Missing | Present |
| D4.02 | Deontic norms for biometric processing | Partial deontic engine | Present wired |
| D4.03 | SPARQL-MM over vision observations | Partial | Present |
| D4.04 | SPARQL-FED policy ask/answer | Partial federation catalog | Present for biometric policy |
| D4.05 | CCTV pipeline policy filter | Missing | Present |
| D4.06 | Multi-principal / multi-camera graph | Missing | Present |
| D4.07 | Cross-border / jurisdiction tags on policy | Missing | Present |
| D4.08 | Duress / sanctuary interaction | Partial sanctuary | Present |
| D4.09 | Mindware–biometric co-binding | Missing | Present |
| D4.10 | Rights audit trail (Six Vectors where wired) | Partial | Present |

### D5 — 3D, manufacture, training geometry

| ID | Capability | Now | Target |
|----|------------|-----|--------|
| D5.01 | MeshIR + validate | Present | Present |
| D5.02 | OBJ export | Present | Present |
| D5.03 | **STL** export (3D print) | Missing | Present |
| D5.04 | **3MF** / print metadata | Missing | Present |
| D5.05 | glTF/GLB export | Partial core render path | Present from vision handoff |
| D5.06 | Image→3D (heightfield) | Present | Present |
| D5.07 | Image→3D multi-view / better recon | Missing | Present |
| D5.08 | Photogrammetry pipeline (multi-image) | Missing | Present |
| D5.09 | Mesh repair for print (manifold, thickness) | Partial geometry tools | Present print checklist |
| D5.10 | Units / scale / printer envelope checks | Missing | Present |
| D5.11 | Synthetic training scenes (vision) | Present synthetic | Present + mesh labels |
| D5.12 | Synthetic 3D corpora for ML training | Partial | Present |
| D5.13 | `.10d` / Q42 geometry handoff | Present | Present |
| D5.14 | Twin eligibility + A1 engineering preview | Present | Present; deepen FEA only with honesty |

### D6 — Engineering / physics / math (existing → vision-linked)

*Already substantial in-tree; excellence = **wire** to vision meshes and print validation.*

| ID | Capability area (existing modules) | Now | Target for vision programme |
|----|--------------------------------------|-----|-----------------------------|
| D6.01 | Computational geometry (Delaunay, CSG, remesh, Poisson, …) | Present | Present + vision mesh ingest |
| D6.02 | FEM / structural / thermal / vibration | Present engineering_analysis | Print/stress **preview** with assurance class |
| D6.03 | CFD / fluid | Present (partial) | Optional vertical |
| D6.04 | Physics ODE / fields / MD | Present physics_simulation | Support synthetic training & biomechanics later |
| D6.05 | Linear algebra / multivar calculus | Present | Support recon/calib numerics |
| D6.06 | Symbolic math | Present | Documentation / teaching vertical |
| D6.07 | Assurance A0–A4 honesty | Documented | Never claim A4 from software alone |

### D7 — Biology & clinical structure

| ID | Capability | Now | Target |
|----|------------|-----|--------|
| D7.01 | Medical imaging DSP / records | Partial medical_computing | Present under Wellfair sensitivity |
| D7.02 | Anatomy mesh library / QApp | Partial Anatomy QApp | Present + SPARQL anatomy |
| D7.03 | Microscopy / slide pipeline | Missing | Present when product needs |
| D7.04 | Cell/organism tracking | Missing | Present research vertical |
| D7.05 | Biomarker / condition graph | Partial Anatomy knowledge | Present |
| D7.06 | Cheminformatics / drug discovery libs | Present specialized | Link when molecular imaging |
| D7.07 | Clinical formulas (non-vision) | Present | Keep separate; no false imaging diagnosis |
| D7.08 | HIPAA-class process notes | Partial compliance module | Present software-side only |
| D7.09 | Bio twin (viz-only default) | Twin viz-only | Same A1 honesty as engineering |

### D8 — Multimodal & time

| ID | Capability | Now | Target |
|----|------------|-----|--------|
| D8.01 | Shared media clock | Present audio | Present AV |
| D8.02 | Non-causal AV correlation | Present | Present |
| D8.03 | Joint biosense (video pulse + audio breath) | Missing | Present |
| D8.04 | Cross-modal training pairs export | Missing | Present |

### D9 — Product surfaces & packaging

| ID | Capability | Now | Target |
|----|------------|-----|--------|
| D9.01 | Studio Vision workbench | Partial | Present classical + biosense |
| D9.02 | Listen / Wellfair handoff | Partial | Present |
| D9.03 | Library model/ontology catalogue | Present seed | Present biosense models |
| D9.04 | WASM/edge capability profiles | Present pattern | Declared biosense subset |
| D9.05 | Desktop camera + consent UX | Partial | Present |

---

## 3. Requirements (cross-cutting)

### R1 — Rights & selfhood (hard requirements)

| Req | Statement |
|-----|-----------|
| R1.1 | Biometric templates and mindware unlock material are **selfhood**; default sanctuary / highest agency domain. |
| R1.2 | Processing requires **purpose-bound consent**; missing purpose → fail closed. |
| R1.3 | Principal can **revoke** templates and purposes; revoke is cryptographically meaningful. |
| R1.4 | Graph may hold **observations and policy**, never raw template bytes in NQuins. |
| R1.5 | Surveillance/CCTV nodes must support **policy check before** identification/enrolment. |
| R1.6 | Affect outputs are **proposals** with uncertainty and non-claims; never silent facts. |
| R1.7 | Multi-person frames: only consented primary (or explicit multi-consent). |
| R1.8 | Audit: who / when / why / what / where / cost for biometric and surveillance decisions. |

### R2 — Engineering quality (hard requirements)

| Req | Statement |
|-----|-----------|
| R2.1 | Pure Rust product path; no required OpenCV/cloud biometrics. |
| R2.2 | Zero-heap hot paths for kernels; caller buffers. |
| R2.3 | rPPG/EVM report **confidence/SNR/failure reasons**; refuse over invent. |
| R2.4 | Print exports: manifold / units / scale checks before “print-ready” label. |
| R2.5 | Assurance class on engineering claims (A0–A4); no fake A4. |
| R2.6 | Tests: synthetic fixtures mandatory; real corpora COMPLETE-WITH-GATE. |
| R2.7 | Method/model hashes on every biosense and detector result. |
| R2.8 | Deterministic seeds for synthetic training corpora. |

### R3 — Query & federation (hard requirements)

| Req | Statement |
|-----|-----------|
| R3.1 | SPARQL-MM over vision/audio **observations** without template export. |
| R3.2 | SPARQL-FED (or equivalent federated ask) for **biometric policy permit/deny**. |
| R3.3 | Deontic compile from N3/policy UI into enforceable norms. |
| R3.4 | Camera/node identity bound in context field of policy quins. |

### R4 — 3D manufacture & training (hard requirements)

| Req | Statement |
|-----|-----------|
| R4.1 | STL (and preferably 3MF) export from validated MeshIR. |
| R4.2 | Print readiness report (holes, non-manifold, thin walls heuristic). |
| R4.3 | Link to engineering_analysis for **optional** stress/thermal **preview** with honesty. |
| R4.4 | Synthetic 3D + 2D labelled sets for training; provenance receipts. |
| R4.5 | Photogrammetry / multi-view path beyond heightfield for excellence. |

### R5 — Biology (hard requirements)

| Req | Statement |
|-----|-----------|
| R5.1 | Clinical/bio imaging paths inherit Wellfair sensitivity and selfhood where biometric. |
| R5.2 | No automated diagnosis claim from vision without clinical process + human. |
| R5.3 | Anatomy meshes addressable in graph (existing Anatomy direction). |
| R5.4 | Microscopy/tracking verticals are first-class when built — same media→graph pattern. |

---

## 4. Design principles (implementation)

1. All product path **Rust**; no Python libraries.  
2. Integrated with graph, Library, sanctuary, Studio, Wellfair.  
3. Zero-heap hot paths; cold arenas for construction.  
4. Pixels/templates out of NQuins.  
5. Epistemic honesty always.  
6. Shared wgpu; CPU oracles.  
7. OpenCV **test oracle** optional only.  
8. Licence-clean defaults; principal gates for weights/corpora.  
9. Capability registry = truth.  
10. **Selfhood bar** for biometrics/mindware.  
11. **Surveillance is policy-subject**, not default owner of faces.  
12. Reuse engineering/geometry/physics libs — do not reimplement FEM for vanity.  
13. Libraryization: `cv/`, `biosense/`, `spatial/` export formats, policy modules.  
14. No “it’ll do” for biosense or print-ready labels.  
15. **Anti-monolith law §4.2** — single-function files, subdir libraries; swarm enforces.  
16. **Swarm delivery §10** — exclusive tracks, CLAIM/RELEASE, progress log, orchestrator integrates.

### 4.1 Module layout (target)

```text
crates/qualia-vision/src/
  cv/                 # classical D1 — library of subdirs
  biosense/           # D3 excellence — library of subdirs
  spatial/            # existing + print formats (D5)
  policy/             # D4 helpers (or client-core if graph-heavy)
  recipes/            # composed excellence pipelines (one recipe per file)

# Prefer existing crates for heavy domains:
#   computational_geometry, engineering_analysis, physics_simulation,
#   medical_computing, wellfare, sparql_mm, deontic_logic
```

### 4.2 Anti-monolith law (mandatory for this programme)

**No monolithic files.** This programme is large; sprawl is a failure mode. Follow project libraryization (AGENTS/Claude: big file → library with subdirectory) and **stricter** defaults here:

| Rule | Detail |
|------|--------|
| **One primary function (or one coherent type + its inherent methods) per `.rs` file** | Prefer `gaussian_blur_u8.rs` exporting `gaussian_blur_u8`, not `filters.rs` with 20 filters. |
| **Library = directory** | Concern `filter` → `cv/filter/mod.rs` + `cv/filter/gaussian_blur_u8.rs` + `…/median_blur_u8.rs` + `…/tests/` or `#[cfg(test)]` in same file only if tiny. |
| **`mod.rs` is a wiring file only** | Re-exports, `mod` declarations, capability registration hooks. **No** algorithm bodies in `mod.rs` beyond trivial `pub use`. Target **&lt; ~80 lines**. |
| **Hard size tripwire** | If a file approaches **~150–200 lines** of logic, **split before merge**. Do not wait for 500. |
| **Tests co-located** | Prefer `#[cfg(test)] mod tests` in the same single-function file, or `foo/tests/gaussian_blur_u8.rs` if fixtures are heavy — still one concern per test file. |
| **No “util.rs” / “helpers.rs” grab-bags** | Name by function: `snr_f32.rs`, `bandpass_iir.rs`. |
| **Types that are shared** | `types/` or `common/` with **one type (or tightly coupled enum+error) per file**. |
| **Recipes** | `recipes/self_monitor_pulse.rs` — orchestration only; call into single-function modules. |
| **Desktop / Studio** | New UI panels as separate components; do not grow `commands/mod.rs` by thousands of lines — add `commands/vision_*.rs` or `browser/`-style submodules and re-export. |
| **Pre-existing monoliths** | Do not block new work on full renames of old files mid-feature; **new code** must obey this law. Flag old monoliths for a later libraryization pass. |

#### 4.2.1 Example shape (rPPG)

```text
crates/qualia-vision/src/biosense/
  mod.rs                      # mods + re-exports only
  consent/
    mod.rs
    biosense_consent.rs       # struct + grant/revoke
    purpose.rs                # enum
  quality/
    mod.rs
    frame_blur_score.rs
    face_fraction.rs
    motion_energy.rs
    reject_low_quality.rs
  rppg/
    mod.rs
    skin_roi_cheeks.rs
    pos_rppg_trace.rs
    chrom_rppg_trace.rs
    spectral_hr_peak.rs
    ensemble_hr.rs
    snr_confidence.rs
  magnification/
    mod.rs
    eulerian_color_magnify.rs
    eulerian_motion_magnify.rs
    ...
  registry.rs                 # capability rows only (or capability/register_biosense.rs)
```

#### 4.2.2 Example shape (classical filter)

```text
crates/qualia-vision/src/cv/
  mod.rs
  buffer/
    mod.rs
    image_view2d.rs
    image_buffer2d.rs
    copy_roi_u8.rs
  filter/
    mod.rs
    gaussian_blur_u8.rs
    median_blur_u8.rs
    bilateral_filter_u8.rs
  edges/
    mod.rs
    sobel_u8.rs
    canny_u8.rs
```

**Agent rejection criterion:** a PR/wave that adds a 400+ line multi-algorithm file **fails review** under this plan unless Timothy grants an exception in NOTICES.

---

## 5. Waves

### VX0 — Registry, ADR, fixtures

- ADR: excellence + selfhood biometrics + surveillance policy + 3D manufacture + biology  
- Capability registry rows for **D1–D9**  
- Fixtures: edges, checkerboard, synthetic PPG video, synthetic micro-motion, print mesh  
- Progress log  

### VX1 — Classical image processing (D1 essentials)

Colour, filters, morph, edges, hist, contours, draw, ROI.

### VX2 — Codecs + capture (D1.10–12, D9)

PNG/JPEG; video file; camera consent UX.

### VX3 — Features + warps (D1.07–08)

ORB-class, match, RANSAC, warp.

### VXB — Biosensing excellence (D3) — **primary, not polish**

| Subwave | Content |
|---------|---------|
| **VXB0** | Consent, purpose, quality gates, audit |
| **VXB1** | Face mesh / landmarks |
| **VXB2** | Multi-ROI rPPG + SNR/confidence |
| **VXB3** | Eulerian (and Lagrangian) micro-change magnification — **TODO-EVM1** excellence-grade EVM (pyramid + Hz band-pass + chroma colour mag + SNR abstain); lite residual mag is **not** the bar |
| **VXB4** | Respiration + audio fusion |
| **VXB5** | Biometric vault, liveness, revoke, voice co-policy |
| **VXB6** | Affect proposals + non-claims + reject/correct |
| **VXB7** | Recipes: self-monitor pulse, see-my-pulse, sanctuary unlock, affect journal |

### VXP — Policy & surveillance (D4) — **paired with VXB**

| Subwave | Content |
|---------|---------|
| **VXP0** | Policy vocabulary (camera, biometric_class, purpose, place, time) |
| **VXP1** | Deontic compile + evaluate_processing_act() |
| **VXP2** | SPARQL-MM observation queries (no template leak) |
| **VXP3** | SPARQL-FED / federated **policy ask** (permit/deny + reason) |
| **VXP4** | CCTV compliance mode: stage filter by policy |
| **VXP5** | Studio/Library authoring of biometric use policy |

### VX3D — Manufacture & training geometry (D5 + D6 wire-up)

| Subwave | Content |
|---------|---------|
| **VX3D0** | STL + 3MF export from MeshIR |
| **VX3D1** | Print readiness report (manifold, units, envelope) |
| **VX3D2** | Multi-view / photogrammetry path (beyond heightfield) |
| **VX3D3** | Synthetic 3D+2D training corpora + receipts |
| **VX3D4** | Optional FEM/thermal **preview** via engineering_analysis (A1 honesty) |
| **VX3D5** | Training export for local ML (masks, depth, mesh labels) |

### VXBIO — Biology applications (D7)

| Subwave | Content |
|---------|---------|
| **VXBIO0** | Sensitivity/selfhood routing for bio/clinical images |
| **VXBIO1** | Anatomy mesh ↔ graph completeness (build on Anatomy QApp) |
| **VXBIO2** | Medical imaging pipeline hygiene (DSP existing → vision buffers) |
| **VXBIO3** | Microscopy / cell track vertical (if demanded) |
| **VXBIO4** | Explicit non-diagnosis UI and software assurance notes |

### VX4 — Calib / stereo / depth (D1 + D5)

Zhang, PnP, stereo → MeshIR.

### VX5 — Motion (D1.09) — supports rPPG stability

### VX6 — Photo / stitch

### VX7 — Licensed learned models (D2)

### VX8 — Composition, ledger, product surfaces (D8–D9)

- Recipes spanning classical + biosense + print + policy  
- Full capability ledger green/honest  
- Studio Vision + Wellfair + policy authoring  
- WASM subset declaration  
- Rights/cost trails for biometric acts  

---

## 6. Priority order

```text
VX0  registry + selfhood/surveillance/3D/bio ADRs
 → VX1 classical image processing
 → VX2 codecs + camera consent
 → VXB0 + VXP0  consent + policy vocab (parallel)
 → VXB1 face mesh
 → VXB2 rPPG + VXB3 magnification   ← excellence biosense
 → VXP1–VXP4  deontic + SPARQL-MM + FED policy
 → VX3 features/warps
 → VX3D0–VX3D1  STL/print readiness   ← manufacture
 → VX5 motion (rPPG support)
 → VXB5 biometrics vault + liveness
 → VXB6 affect (opt-in)
 → VX3D2–VX3D5 recon + training corpora
 → VXBIO0–VXBIO2 biology floor
 → VX4 stereo/calib as needed
 → VX6–VX7 photo + licensed models
 → VX8 full excellence ship
```

---

## 7. Success criteria (2026 excellence)

1. **D1–D3** claimed rows are Present (or N/A) with tests and confidence behaviour.  
2. **D4:** principal can author biometric policy; CCTV-class ask gets permit/deny via graph/FED path.  
3. **D5:** validated mesh → **STL/3MF** + print readiness; training corpora exportable with provenance.  
4. **D6:** engineering previews use existing solvers with **assurance labels**.  
5. **D7:** bio/clinical paths respect sensitivity; no fake diagnosis.  
6. Biometrics/mindware treated as **inalienable selfhood** in code paths, not only docs.  
7. No product default dependency on OpenCV or cloud biometrics.  
8. Registry + progress log match reality (no silent incomplete).

---

## 8. Effort honesty

| Block | Scale |
|-------|--------|
| VX0–VX2 | Foundation multi-session |
| VXB0–VXB3 | **Large** research-grade biosense |
| VXP | Medium–large (policy + FED wiring) |
| VX3D | Medium (formats + recon depth) |
| VXBIO | Medium; deep microscopy is multi-session |
| VX8 | Integration and dogfood |

---

## 9. Principal decisions

| ID | Ask | Default if silent |
|----|-----|-------------------|
| VX-D1 | Classical floor through VX3 first? | **Yes** |
| VX-D2 | OpenCV test oracle only? | Yes |
| VX-B1 | Affect on by default? | **No** — opt-in |
| VX-B2 | Biometric unlock in first excellence ship? | Principal; liveness mandatory if yes |
| VX-B3 | Contact PPG corpus for rPPG validation? | COMPLETE-WITH-GATE |
| VX-P1 | Publish biometric policy vocabulary publicly? | Principal |
| VX-P2 | Multi-camera federation in first ship? | Single-node policy first; FED next |
| VX-3D1 | STL vs 3MF priority? | **STL first**, 3MF next |
| VX-3D2 | Photogrammetry in first excellence ship? | After STL + heightfield path solid |
| VX-BIO1 | Microscopy in first ship? | No unless principal prioritises |

---

## 10. Swarm delivery — how to get this done

This section is the **execution manual** for multi-agent swarms. Architecture is above; **here is how agents ship it** without collision or monoliths.

### 10.1 Roles

| Role | Who | Duties |
|------|-----|--------|
| **Principal** | Timothy | Prioritises waves, supplies corpora/weights/gates, dogfoods, settles VX-D* decisions |
| **Orchestrator** | One parent agent | CLAIM/RELEASE, assigns tracks, integrates, runs tests, updates progress log, refuses monoliths |
| **Track agent** | Subagent or sequential session | Owns **one exclusive directory set**, implements files per §4.2, tests, reports, does not touch other tracks |
| **Review gate** | Orchestrator (or principal) | Size check, capability registry update, honesty labels |

### 10.2 Global swarm rules

1. **Canonical tree only** — `C:\Projects\qualia-27062026`; no worktrees for routine work.  
2. **CLAIM before write** — append to `coordination/NOTICES.md` with exclusive paths.  
3. **One CLAIM per exclusive set** — if path is claimed, stop and report.  
4. **RELEASE** with: files added, tests run, registry rows flipped, honesty notes.  
5. **Progress log** — `docs/plans/native-vision-capability-excellence-PROGRESS-LOG.md` after every wave/track.  
6. **Anti-monolith law §4.2** — non-negotiable for new code.  
7. **No Python** in product libraries.  
8. **No vendor OpenCV** in product path; optional `opencv-oracle` test feature only if principal allows.  
9. **Parent integrates** — track agents do not force-push over each other; orchestrator merges order.  
10. **Capability registry first** — every Present claim updates machine-readable registry in the same PR/wave.  
11. **Completeness bar** — no TODO-as-implementation for advertised excellence rows.  
12. **Selfhood** — biometric/surveillance work fails closed without consent/policy paths.

### 10.3 Exclusive track map (parallelism)

Tracks may run **in parallel only** when exclusive directories do not overlap.

| Track ID | Scope | Exclusive paths (typical) | Depends on |
|----------|--------|---------------------------|------------|
| **T-REG** | Registry, ADR, fixtures, progress log | `qualia-vision/src/capability/` or `cv/registry*`, `biosense/registry*`, `fixtures/` | — |
| **T-CV1** | Classical buffers + colour + filter + morph + edges | `qualia-vision/src/cv/buffer/`, `color/`, `filter/`, `morph/`, `edges/`, `hist/`, `contours/`, `draw/` | T-REG |
| **T-CV2** | Codecs + capture hooks | `cv/codecs/`, client/desktop camera commands (narrow files) | T-CV1 buffer types |
| **T-CV3** | Features + warps | `cv/features/`, `cv/transform/` | T-CV1 |
| **T-FLOW** | Optical flow / BG | `cv/flow/` | T-CV1 |
| **T-BIO0** | Consent, quality, audit | `biosense/consent/`, `biosense/quality/`, `biosense/audit/` | T-REG |
| **T-MESH** | Face landmarks / mesh | `biosense/face_mesh/` (+ weights path) | T-CV1, T-BIO0 |
| **T-RPPG** | rPPG algorithms | `biosense/rppg/` (one algo per file) | T-MESH or ROI quality |
| **T-EVM** | Magnification (**TODO-EVM1**) | `biosense/magnification/` — pyramid, temporal band-pass (`fps`/`f_lo`/`f_hi`), colour EVM (chroma), motion EVM, SNR abstain, consent gate | T-CV1 colour/filter |
| **T-PAD** | Liveness | `biosense/liveness/` | T-MESH |
| **T-TMPL** | Biometric vault templates | `biosense/biometrics/` + wellfair hooks (coordinate) | T-BIO0, T-MESH |
| **T-AFFECT** | Affect proposals | `biosense/affect/` | T-MESH, T-BIO0 |
| **T-POL** | Surveillance policy + SPARQL-MM/FED | `policy/` and/or `sparql_mm` + deontic bridge files **claimed narrowly** | T-BIO0 |
| **T-3D** | STL/3MF/print check | `spatial/export_stl.rs`-style **one format per file**, `spatial/print_*/` | existing MeshIR |
| **T-RECON** | Photogrammetry / multi-view | `spatial/recon_*/` or `cv/recon/` | T-CV3, T-3D basics |
| **T-BIOLOGY** | Bio/clinical vision path | `biosense/bio_*/` or medical_computing adapters | T-CV2 sensitivity |
| **T-UI** | Studio Vision / Wellfair surfaces | `vision_workbench` splits, wellfair panels | APIs stable |
| **T-DESK** | Desktop commands | `webizen-desktop/src/commands/vision_*.rs` **new files only** | APIs stable |
| **T-INT** | Recipes + ledger | `recipes/` one recipe per file | Multiple tracks done |

**Collision rule:** `commands/mod.rs` and large Studio files — **one agent at a time**, or only add `mod vision_excellence;` + re-exports.

### 10.4 Wave → track assignment (orchestrator cheat sheet)

| Wave | Tracks to spawn / sequence |
|------|----------------------------|
| VX0 | T-REG alone |
| VX1 | T-CV1 (may fan out sub-agents per subdir: filter ∥ edges ∥ morph if exclusive) |
| VX2 | T-CV2 + T-DESK camera (serial if commands clash) |
| VXB0 | T-BIO0 |
| VXB1 | T-MESH |
| VXB2 | T-RPPG |
| VXB3 | T-EVM |
| VXP | T-POL |
| VX3 | T-CV3 |
| VX3D | T-3D then T-RECON |
| VXB5 | T-PAD + T-TMPL |
| VXB6 | T-AFFECT |
| VXBIO | T-BIOLOGY |
| VX8 | T-UI + T-INT + ledger |

### 10.5 Single-track agent prompt template

Orchestrator pastes this (fill brackets):

```text
You are Track [T-ID] on Qualia vision excellence.
Canonical tree: C:\Projects\qualia-27062026 only. Branch 0.0.25.
CLAIM exclusive paths: [list]. Do not edit other tracks' dirs.
Read: docs/plans/native-vision-capability-excellence-2026.md §4.2 and §10.

Implement: [slice list, e.g. gaussian_blur_u8 + median_blur_u8].
Layout law: ONE primary function per .rs file; library subdirs; mod.rs wiring only.
No Python. No OpenCV product link. Zero-heap hot paths (caller buffers).
Update capability registry rows for what you finish.
Add tests in-file or co-located.
When done: cargo test -p qualia-vision --lib [filter]; append PROGRESS-LOG; RELEASE in NOTICES.
Do not claim excellence for unfinished biosense/confidence behaviour.
```

### 10.6 Subagent fan-out pattern (within a track)

When a track is large (e.g. T-CV1):

1. Orchestrator creates empty dir skeleton + `mod.rs` stubs.  
2. Spawns N agents with **disjoint file names** (`gaussian_blur_u8.rs` vs `median_blur_u8.rs`).  
3. Each agent only creates **its files** + local tests.  
4. Orchestrator runs full module tests and fixes re-exports once.

Do **not** give two agents the same `mod.rs` to edit concurrently — orchestrator owns `mod.rs` wiring or serialises it.

### 10.7 Definition of done (per track)

- [ ] Files obey §4.2 (spot-check: no multi-algorithm monoliths)  
- [ ] `cargo test -p qualia-vision` (and affected crates) green for the track  
- [ ] Capability registry updated  
- [ ] Honesty labels / consent fail-closed where required  
- [ ] Progress log entry  
- [ ] NOTICES RELEASE  
- [ ] No unclaimed edits outside exclusive set  

### 10.8 Progress log format (append-only)

```markdown
## YYYY-MM-DD — Track T-XXX / Wave VY
**Status:** done | partial | blocked
**Files:** list (expect many small files)
**Tests:** command + pass count
**Registry:** rows flipped Present
**Honesty / gates:** …
**Monolith check:** max file lines in track = N (must be well under 200 logic lines unless exception)
**Next:** …
```

### 10.9 Human gates (swarm must not fake)

| Gate | Owner | Swarm behaviour |
|------|--------|-----------------|
| Licensed face/mesh/affect weights | Timothy | COMPLETE-WITH-GATE; seed/reference only until supplied |
| rPPG clinical compare corpus | Timothy | Synthetic fixtures until contact PPG available |
| Biometric policy legal text | Timothy | Code vocabulary + examples; principal authors binding policy |
| H1 vision eval images | Timothy | Synthetic metrics only until H1 |
| “Best in market” public claim | Timothy | Only after VX8 ledger green |

### 10.10 Orchestrator startup checklist (execute vision-excellence)

1. `CLAIM | Vision excellence VX0 + swarm boot | cv/, biosense/, fixtures, registry`  
2. Create progress log from §10.8 header.  
3. Land empty directory trees + registry schema (T-REG).  
4. Publish track board in NOTICES (which tracks free/blocked).  
5. Spawn or sequence per §10.4.  
6. After each track: monolith scan (`find` large new `.rs` files) + tests.  
7. Integrate recipes only when dependencies Present.  
8. Final VX8: ledger + Studio dogfood notes.

---

## 11. Immediate next step

When Timothy says **execute vision-excellence** / **VX0**:

1. Orchestrator CLAIMs and runs **§10.10**.  
2. T-REG: capability manifest for **D1–D9**, empty `cv/` + `biosense/` trees per §4.2.  
3. Fixtures: classical + synthetic PPG video + micro-motion + print mesh.  
4. Then parallel **T-CV1** and **T-BIO0** as soon as buffer types exist.  
5. Every agent follows **single-function files + library subdirs**.

---

## 12. One-page mental model

```text
                    ┌─────────────────────────────┐
                    │  Principal selfhood vault     │
                    │  biometrics · mindware        │
                    │  purpose · revoke · audit     │
                    └──────────────┬────────────────┘
                                   │ policy / deontic
           ┌───────────────────────┼───────────────────────┐
           ▼                       ▼                       ▼
    ┌─────────────┐         ┌─────────────┐         ┌─────────────┐
    │ Vision/CV   │         │ Biosense    │         │ Surveillance│
    │ classical   │────────▶│ rPPG·EVM·   │         │ CCTV node   │
    │ learned     │         │ face mesh   │◀─SPARQL─│ FED ask     │
    └──────┬──────┘         └──────┬──────┘  MM/FED └─────────────┘
           │                       │
           ▼                       ▼
    ┌─────────────────────────────────────────────┐
    │ MeshIR → STL/3MF · training corpora · twin  │
    │ geometry · engineering preview (A1 honesty) │
    │ biology/anatomy under Wellfair sensitivity  │
    └─────────────────────────────────────────────┘
```

---

*End of plan. Biometrics and mindware = inalienable self. Surveillance asks permission via graph. 3D print and training are first-class. Biology shares the same rights-aware media path. Classical CV is the floor; excellence is the whole diagram.*
