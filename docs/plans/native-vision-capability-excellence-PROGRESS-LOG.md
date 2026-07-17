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
