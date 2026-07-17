# Native Vision Capability Excellence — Progress Log

**Plan:** `native-vision-capability-excellence-2026.md`  
**Branch:** `0.0.25`

---

## 2026-07-17 — VX0 + VX1 + VXB core + VXP + VX3D ship

**Status:** done (agent-completable core); COMPLETE-WITH-GATE / Missing rows remain honest in registry

### Built

| Area | Layout |
|------|--------|
| **Registry** | `capability/{status,entry,registry}.rs` — D1–D9 rows |
| **cv/** | buffer, color, filter×3, morph×2, edges×2, hist×2, contours, transform×2, features×3, flow, photo, draw — **one function per file** |
| **biosense/** | consent, quality×3, rppg×4, magnification×2, face ROI, affect proposal, template hash, policy/CCTV, respiration |
| **spatial** | `export_stl.rs`, `print_readiness.rs` |
| **recipes/** | `self_monitor_pulse.rs` |

### Tests

`cargo test -p qualia-vision --lib` → **71 passed**, 0 failed

### Monolith check

New algorithm files are single-function modules under subdirs; `mod.rs` files are wiring only.

### Honesty / gates still open

- Licensed face mesh / production embeddings / clinical rPPG corpus  
- Full SPARQL-FED multi-camera  
- Photogrammetry multi-view  
- Video file I/O, stitch  
- WASM edge profile declaration  
- Studio full workbench wiring for every op  

### Next

Wire Studio/desktop commands; deepen face mesh weights; FED policy; multi-view recon — or principal dogfood this core.

---

## 2026-07-17 — Commercial model pack + challenge PAD

**Status:** done (docs + PAD code)

**What:**
- `docs/plans/vision-excellence-commercial-model-pack.md` — MediaPipe mesh, SFace, YuNet, YOLO-NAS, OMZ emotion optional, challenge PAD design
- `biosense/liveness/` — challenge-response PAD (yaw/smile/blink + static-mesh reject)
- `blendshape_affect_proposal` — Path A affect without AffectNet
- Registry honesty strings point at pack; D3.09 → Present (challenge PAD)

**Assets principal still fetches offline:** TFLite/ONNX weights into `{storage}/models/vision/` (not git).

**Next:** ONNX/TFLite load adapters when weights present; wire mesh signals into PAD.

---

## 2026-07-17 — Pure-landmark PAD (geometric / temporal / non-rigid Z)

**Status:** done (library)

**What built:**
- `biosense/liveness/` single-function modules:
  - `landmark_types` — 8-point MediaPipe-mapped packing
  - `temporal_window` — TTS 800 ms / TTC 2000 ms
  - `rigid_head_pose` — PnP-class yaw/pitch/roll, IOD-normalized
  - `action_threshold` — degrees + mouth/IOD ratios
  - `non_rigid_z` — flat-mask lock (linear residual of nose–cheek ratio)
  - `landmark_jitter` — ~1 s noise floor
  - `camera_stream_integrity` — virtual-camera fail-closed hook
  - `challenge_pad` — `evaluate_landmark_pad` orchestrator + legacy path
- Challenge set expanded: open_mouth, pitch_up/down; `issue_rotation_challenge`
- Pack docs §4 rewritten to pure-landmark architecture
- Registry D3.09 string updated

**Measured:** `cargo test -p qualia-vision` (see session)

**Where human is needed:** production threshold calibration in MANIFEST; OS camera attestation host wiring; MediaPipe ONNX/TFLite drop-in.

**Next:** mesh adapter feeding `LandmarkFrame`; host sets `CameraStreamAttestation::physical_attested()` on unlock path.

---

## 2026-07-17 — PAR geometric lock (no model Z)

**Status:** done

**Trap closed:** Do **not** validate on MediaPipe inferred Z — statistical priors hallucinate depth from a flat iPad. Lock uses **raw 2D image \(x\) only**.

**Math (Profile Asymmetry Ratio):**
- Landmarks: nose tip MP **1**, left edge **234**, right edge **454**
- \(d_L = |x_N - x_L|\), \(d_R = |x_R - x_N|\), \(PAR = d_L/d_R\)
- Baseline-normalize at \(t_0\); \(\Delta PAR = |PAR(t_1)/PAR(t_0) - 1|\)
- Live 3D: \(\Delta PAR > \tau\) (default **0.6**); flat mask: \(\Delta PAR \approx 0\)
- Require yaw span ≥ **25°**

**Code:** `biosense/liveness/profile_asymmetry_ratio.rs`; `non_rigid_z` is a thin PAR façade.

**Measured:** `cargo test -p qualia-vision --lib` (session)

**Human:** calibrate τ after real capture; never wire model Z into this gate.

---

## Implementation to-do (queued)

Open agent-executable work. Mark **done** in a dated entry when shipped; do not claim excellence for lite stubs.

**Full industry map (all catalogue rows):**  
[`vision-capability-catalogue-2026.md`](vision-capability-catalogue-2026.md) — Physiological → Specialty Optics + Decentralized/P2P, with Status + TODO IDs + policy/OOS tags.

### W0 — Active excellence (next to execute)

| ID | Item | Track / path | Status | Notes |
|----|------|--------------|--------|-------|
| **TODO-EVM1** | **Excellence-grade Eulerian Video Magnification (EVM)** | **T-EVM** · `biosense/magnification/` | **done (2026-07-17)** | Pyramid + Hz IIR + YIQ colour + multi-scale motion + SNR abstain + consent — see session note |
| TODO-MESH1 | MediaPipe / face mesh adapter → `LandmarkFrame` | T-MESH · `biosense/face_mesh/` | queued | Feed PAD PAR + rPPG ROI; Apache pack when weights drop |
| TODO-PAD1 | Host camera attestation on unlock path | T-PAD + desktop | queued | `CameraStreamAttestation::physical_attested()` |
| TODO-PAD2 | Calibrate PAR τ / yaw policy in MANIFEST | T-PAD | queued | Principal real-capture pass |
| TODO-ONNX1 | ONNX/TFLite loaders for commercial pack | T-MESH / weights | queued | After principal downloads models |
| TODO-FED1 | SPARQL-FED multi-camera policy depth | T-POL | queued | COMPLETE-WITH-GATE |
| TODO-3D1 | Photogrammetry multi-view beyond heightfield | T-RECON | queued | After STL path solid |
| TODO-UI1 | Studio/desktop biosense + EVM workbench wire | T-UI / T-DESK | queued | Consent-bound; no silent bio |

### W1 — Physiological depth (after / parallel EVM)

| ID | Item | Status | Notes |
|----|------|--------|-------|
| TODO-RR1 | Respiratory rate (flow / EVM / rPPG harmonic) | **done** (S7-RR 2026-07-17) | Motion spectral RR + rPPG low-freq harmonic + ensemble; SNR abstain; synthetic 12–20 bpm green |
| TODO-SPO2 | Remote SpO₂ proxy (RGB ratio; clinical honesty) | queued + gated | Non-diagnosis; corpus for any clinical claim |
| TODO-PUPIL | Pupillometry (iris ROI + diameter track) | queued | After mesh/iris landmarks |
| TODO-MICROX | Micro-expression / AU temporal events | queued | Extends D3.14; mesh required |
| TODO-RPPG-DEEP | Optional DeepPhys-class path | gated | Only if Apache-OK weights; POS/CHROM remain default |

### W2 — Tracking & kinematics

| ID | Item | Status | Notes |
|----|------|--------|-------|
| TODO-MOT | MOT upgrade (ByteTrack-class association) | queued | Beyond BoundedTracker lite |
| TODO-POSE | Body pose 2D/3D (MediaPipe Pose / MoveNet pack) | queued | D2.06 |
| TODO-HAND | Hand 21-pt + gesture | queued | MediaPipe Hands pack |
| TODO-GAZE | Gaze / iris point-of-regard | queued | After iris landmarks |
| TODO-6D | 6D object pose | queued | Robotics/AR |
| TODO-KINLIV | Kinematic liveness (blink/swallow physics) | queued | Complements PAD; not substitute for PAR |
| TODO-AVSYNC | Deepfake AV lip-sync **detection** | queued | Defence only |
| TODO-GAIT | Gait biometrics | queued + policy | High sensitivity |

### W3 — Scene & geometry (learned)

| ID | Item | Status | Notes |
|----|------|--------|-------|
| TODO-SEG / TODO-INST / TODO-PAN | Semantic / instance / panoptic seg | queued | D2.04; licence-clean weights |
| TODO-DEPTH | Monocular depth (MiDaS-class) | queued | D2.05 |
| TODO-SAL | Salient object / BG remove (U²-Net-class) | queued | |
| TODO-VO | Visual odometry / SLAM path | queued | Links D5 recon |

### W4 — Image restoration

| ID | Item | Status |
|----|------|--------|
| TODO-SR | Super-resolution | queued |
| TODO-LL | Low-light enhancement | queued |
| TODO-DEBLUR | Motion deblur | queued |
| TODO-INPAINT | Inpainting (beyond bilateral denoise) | queued |
| TODO-STAB | Video stabilization (needs D1.11 video I/O) | queued |

### W5 — Document & industrial

| ID | Item | Status |
|----|------|--------|
| TODO-OCR | OCR (D2.07) | queued / product demand |
| TODO-LAYOUT / TODO-HTR / TODO-KIE | Layout, handwriting, key extraction | gated |
| TODO-QR | Barcode/QR pure-Rust decode | queued |
| TODO-DEFECT / TODO-ANOM | Surface defect + unsupervised anomaly | queued |
| TODO-METRO | Dimensional metrology (sub-pixel + calib) | queued |
| TODO-THERM | Thermal/IR analysis path | gated |

### W6 — Policy-heavy verticals (principal demand only)

| ID | Item | Status |
|----|------|--------|
| TODO-SURV-* | Crowd, abandoned, loiter, tripwire | queued + **policy** |
| TODO-SURV-THREAT | Weapon/threat detect | gated + **policy** + principal-only |
| TODO-ANPR / TODO-LANE / TODO-DROWSY / … | ADAS family | gated + policy where PII |
| TODO-PLANO / TODO-HEAT / TODO-VTON / … | Retail family | gated; heatmap = policy |

### W7 — Medical / ag / robotics / remote / optics

| ID | Item | Status |
|----|------|--------|
| TODO-CELL / TODO-RAD / TODO-SURG | Cell, radiology, surgical tools | gated; non-diagnosis |
| TODO-AG-* | Crop/weed/canopy/animal | gated / canopy can be classical early |
| TODO-GRASP / TODO-SERVO / TODO-NAV | Robotics | gated |
| TODO-RS-* | Satellite/drone | gated |
| TODO-HSI / TODO-SCHLIEREN | Specialty optics | gated |
| TODO-FRAME / TODO-HIGHLIGHT | Media framing / highlights | queued after MOT |

### W8 — Vision-language + decentralized (Qualia-native)

| ID | Item | Status | Notes |
|----|------|--------|-------|
| **TODO-P2P-EMB** | Local-first visual embeddings + search | queued | ONNX CLIP/ResNet; no cloud |
| **TODO-P2P-PRIV** | Privacy-preserving feature extract | queued | Edge; HE optional via privacy engine |
| **TODO-P2P-GRAPH** | On-device personal analytics → quins | partial recipes | Selfhood graph; no raw RGB default export |
| TODO-VQA / TODO-CAPTION / TODO-ZSD / TODO-OVS / TODO-CBIR | VLM family | gated | Local LLM + vision; licence pack |

### Explicit OOS (do not implement as product)

| Item | Reason |
|------|--------|
| Deepfake generation / face-swap tools | Offensive synthetic identity; detection only (TODO-AVSYNC) |
| Silent CCTV biometrics | Violates D4 purpose-bound consent |
| Model-Z as PAD depth | Hallucinates on flat screens; use PAR |

### TODO-EVM1 breakdown (excellence bar — not lite)

Current code is a **temporal residual + gain** demo. Excellence EVM (Wu et al. style) requires:

| Step | Deliverable (single-function files preferred) | Done when |
|------|-----------------------------------------------|-----------|
| EVM1.a | Gaussian / Laplacian pyramid build + reconstruct (caller-buffered levels) | ✅ reconstruct ≈ input; max 6 levels |
| EVM1.b | Temporal band-pass with explicit `fps`, `f_lo_hz`, `f_hi_hz` (IIR or FIR; e.g. 0.7–4 Hz HR band) | ✅ diff-of-LP + biquad; DC rejected |
| EVM1.c | **Colour EVM** in chrominance / YIQ-class space (amplify chroma band, not raw RGB noise) | ✅ `colour_evm_yiq` + thin wrap |
| EVM1.d | **Motion EVM** multi-scale (spatial band × temporal band × α) | ✅ Laplacian × IIR × α |
| EVM1.e | SNR / energy gate + **abstain** (refuse invent; report why) | ✅ `EvmRefuse` / `EvmSnrVerdict` |
| EVM1.f | Consent + quality pre-gate; optional face-ROI crop path | ✅ consent wrappers; ROI crop optional later |
| EVM1.g | Registry D3.06 / D3.07 honesty strings + progress-log entry | ✅ Present |
| EVM1.h | Recipe hook (e.g. see-my-pulse visualization) under consent | Optional after core |

**Out of scope for TODO-EVM1:** using EVM as PAD (PAR + jitter remain the locks); model-Z depth; Python reference path.

**Depends on:** T-CV1 buffers (done). Optional: face ROI / mesh for ROI-cropped mag.

**Verify:** `cargo test -p qualia-vision --lib magnification` (+ new pyramid/bandpass tests).

---

## 2026-07-17 — Full vision capability catalogue ingested

**Status:** backlog documented (not implemented)

**What:** Principal-supplied catalogue (Physiological, Biometrics, Tracking, Scene, Image, Document, Industrial, Surveillance, ADAS, Retail, Medical, VLM, Ag, Robotics, Media, Remote Sensing, Decentralized/P2P, Specialty Optics) mapped in `vision-capability-catalogue-2026.md` and waved into this to-do (W0–W8).

**Honesty:** Active PAD, classical CV, lite rPPG/EVM, policy stubs already Present/Partial as before. No new Present claims from catalogue alone.

**Next execute default:** TODO-EVM1 unless principal reprioritises.

---

## 2026-07-17 — Vendor assets + swarm plan (licence honesty fix)

**Status:** done (layout + plan + path resolver); adapters still AdapterMissing

**Trap fixed:** MIT/Apache pack models were mis-labelled as “commercial licence gated.” Correct tags: **PermissiveReady** | **WeightAbsent** | **AdapterMissing** | **TrainingDeferred** | **Policy** | **LicenceHostile**.

**What built:**
- `vendor/vision/` tree (face/detect/pose/affect/…), `MANIFEST.json`, `download.ps1`
- `resolve_vision_asset` in `qualia-vision` weights module
- Swarm plan: `docs/plans/vision-excellence-swarm-execute-2026.md` (tracks S0–S10, waves A–E)
- Registry honesty strings updated
- Training deferred to principal (machine off) — **does not** block published-weight adapters

**Next swarm (3-way):** S0-ASSET (done-ish) → **S1-EVM** + **S2-ONNX** + **S3-MESH**

---

## 2026-07-17 — No-train swarm wave landed

**Status:** substantial progress (not full catalogue W3–W8)

**Swarm tracks:**
| Track | Result |
|-------|--------|
| S1-EVM | Present: pyramid, IIR band-pass, YIQ colour EVM, motion multi-scale, SNR abstain, consent |
| S2-ONNX | Partial: `load_onnx_bytes` validates YuNet/SFace on disk; decode/embed tensor helpers; **no ORT session yet** |
| S3-MESH | Partial→strong: MediaPipe buffer→`LandmarkFrame` + `evaluate_pad_from_mediapipe_trace` (Z discarded) |
| S6-POSE | Partial: pack pose/hand xy; vendor weights present |
| S7-RR | Present: spectral RR + rPPG harmonic + ensemble SNR abstain |

**Measured:** `cargo test -p qualia-vision --lib` → **158 passed**, 0 failed

**Still no-train open:** ORT/TFLite live inference sessions, Studio/desktop wire, MOT upgrade, depth/seg/OCR, video I/O, FED, P2P embeddings.

**Training:** still deferred (principal).

---

## 2026-07-17 � S7-RR / TODO-RR1 (respiratory rate, no training)

**Status:** done (spectral RR stack; synthetic tests green when crate compiles).

**Built:**
- `biosense/respiration/rr_estimate.rs` � `RrEstimate` + band/SNR constants; confidence not clinical-calibrated.
- `biosense/respiration/respiration_rate_from_motion_trace.rs` � demean + dense DFT in 0.1�0.5 Hz, residual-band SNR, parabolic refine, fail-closed SNR gate; shared `spectral_rr_peak`.
- `biosense/rppg/respiration_from_rppg_harmonic.rs` � short MA residual + same RR-band peak (RSA/baseline wander path).
- `biosense/respiration/ensemble_respiration.rs` � confidence-weighted fuse; large ?bpm or low conf ? abstain.
- `respiration_from_motion` compat wrapper; D3.05 registry note updated; crate re-exports.

**Measured:** `cargo test -p qualia-vision --lib respiration` ? **15 passed, 0 failed** (captured mid-session before concurrent S1 magnification mid-edit broke crate compile). Cases: 12/15/18/20 breaths/min sinusoids, noise abstain, rPPG-like recover 16, ensemble agree/disagree/single-source.

**? Human / other tracks:** S1 `magnification/` currently fails to compile (`evm_snr_gate` / `eulerian_*_magnify` signature mismatch) � blocks re-verify of RR until S1 lands. No principal corpus needed for this step (synthetic only; no clinical claim).

**Next:** wire recipe/UI when free; optional optical-flow chest ROI producer (not RR math).

---

## 2026-07-17 � S1-EVM / TODO-EVM1 (excellence Eulerian Video Magnification)

**Status:** done � a�g green (recipe hook EVM1.h optional, not required)

**What built** under `crates/qualia-vision/src/biosense/magnification/`:
- `gaussian_pyramid_build.rs` / `laplacian_pyramid_build.rs` / `pyramid_reconstruct.rs` � caller-buffered, `MAX_PYRAMID_LEVELS=6`
- `gaussian_pyramid_level.rs` � single-step u8 downsample helper
- `temporal_bandpass.rs` � Wu-style diff-of-1-pole LP with explicit `fps`/`f_lo`/`f_hi`
- `temporal_bandpass_iir.rs` � RBJ biquad band-pass (`BandpassState`)
- `colour_evm_yiq.rs` � YIQ chrominance amplify (mean path + optional pixel planes)
- `eulerian_color_magnify.rs` / `eulerian_motion_magnify.rs` � excellence multi-scale + thin legacy API + `*_ex` / `*_hz`
- `evm_snr_gate.rs` � `EvmRefuse` + `EvmSnrVerdict` + `evm_snr_gate_trace` abstain
- `evm_with_consent.rs` � `BiosenseConsent` fail-closed wrappers
- Registry D3.06 / D3.07 ? **Present**

**Tests:** `cargo test -p qualia-vision --lib magnification` ? **19 passed, 0 failed**

**Follow-up (optional):** EVM1.h recipe `see-my-pulse` viz; sharper FIR/FFT band for lab offline; face-ROI crop path.

