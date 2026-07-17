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

| ID | Item | Track / path | Status | Notes |
|----|------|--------------|--------|-------|
| **TODO-EVM1** | **Excellence-grade Eulerian Video Magnification (EVM)** | **T-EVM** · `biosense/magnification/` | **queued** | Lite colour/motion already present (`eulerian_*_magnify.rs`). Promote to research-grade EVM — see breakdown below. |
| TODO-MESH1 | MediaPipe / face mesh adapter → `LandmarkFrame` | T-MESH · `biosense/face_mesh/` | queued | Feed PAD PAR + rPPG ROI; Apache pack when weights drop |
| TODO-PAD1 | Host camera attestation on unlock path | T-PAD + desktop | queued | `CameraStreamAttestation::physical_attested()` |
| TODO-PAD2 | Calibrate PAR τ / yaw policy in MANIFEST | T-PAD | queued | Principal real-capture pass |
| TODO-ONNX1 | ONNX/TFLite loaders for commercial pack | T-MESH / weights | queued | After principal downloads models |
| TODO-FED1 | SPARQL-FED multi-camera policy depth | T-POL | queued | COMPLETE-WITH-GATE |
| TODO-3D1 | Photogrammetry multi-view beyond heightfield | T-RECON | queued | After STL path solid |
| TODO-UI1 | Studio/desktop biosense + EVM workbench wire | T-UI / T-DESK | queued | Consent-bound; no silent bio |

### TODO-EVM1 breakdown (excellence bar — not lite)

Current code is a **temporal residual + gain** demo. Excellence EVM (Wu et al. style) requires:

| Step | Deliverable (single-function files preferred) | Done when |
|------|-----------------------------------------------|-----------|
| EVM1.a | Gaussian / Laplacian pyramid build + reconstruct (caller-buffered levels) | Unit tests: reconstruct ≈ input; level count fixed |
| EVM1.b | Temporal band-pass with explicit `fps`, `f_lo_hz`, `f_hi_hz` (IIR or FIR; e.g. 0.7–4 Hz HR band) | Band rejects DC + high-freq noise in synthetic sinusoid |
| EVM1.c | **Colour EVM** in chrominance / YIQ-class space (amplify chroma band, not raw RGB noise) | Replaces/extends `eulerian_color_magnify` without monolith |
| EVM1.d | **Motion EVM** multi-scale (spatial band × temporal band × α) | Replaces/extends `eulerian_motion_magnify` |
| EVM1.e | SNR / energy gate + **abstain** (refuse invent; report why) | Matches rPPG honesty rule R2.3 |
| EVM1.f | Consent + quality pre-gate; optional face-ROI crop path | No processing without `BiosenseConsent` |
| EVM1.g | Registry D3.06 / D3.07 honesty strings + progress-log entry | Present only when a–e green |
| EVM1.h | Recipe hook (e.g. see-my-pulse visualization) under consent | Optional after core |

**Out of scope for TODO-EVM1:** using EVM as PAD (PAR + jitter remain the locks); model-Z depth; Python reference path.

**Depends on:** T-CV1 buffers (done). Optional: face ROI / mesh for ROI-cropped mag.

**Verify:** `cargo test -p qualia-vision --lib magnification` (+ new pyramid/bandpass tests).
