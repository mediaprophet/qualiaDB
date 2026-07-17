# Vision Excellence — Commercial-Use Model Pack (gate assets)

**Date:** 2026-07-17  
**Purpose:** Principal-curated, **permissive-licence** offline models to clear COMPLETE-WITH-GATE rows without vendoring OpenCV as the product ABI and without non-commercial training-data traps (e.g. MS-Celeb-1M, AffectNet).  
**Formats preferred:** ONNX / TFLite for local execution; Qualia adapters convert to internal buffers / optional P64 later.  
**Status:** Approved candidate list for acquisition + integration — **weights not committed to git** (download offline into `{storage}/models/` or `bundled/models/vision/` when principal fetches them).

---

## 0. Diligence rule (always)

| Check | Required |
|-------|----------|
| Code licence | Apache-2.0 / MIT preferred |
| **Weight** licence | Explicit commercial redistribution/use |
| **Training data** | Prefer models whose training data licence is not known-hostile to commercial weight use |
| Redistribution | Document whether we may ship in installer or only load user-fetched files |

If any check fails, row stays COMPLETE-WITH-GATE or heuristic-only.

---

## 1. Pack table (maps to registry gates)

| Gate / registry ID | Solution | Licence (claimed) | Format | Source |
|--------------------|----------|------------------|--------|--------|
| **Face mesh / landmarks** D3.01 | MediaPipe Face Landmarker / Face Mesh (468 pts) | Apache 2.0 | TFLite → ONNX optional | [MediaPipe](https://github.com/google/mediapipe) · [Face Landmarker](https://developers.google.com/mediapipe/solutions/vision/face_landmarker) |
| **Production embedding** D3.10 | OpenCV Zoo **SFace** | Apache 2.0 (weights in zoo) | ONNX | [opencv_zoo face_recognition_sface](https://github.com/opencv/opencv_zoo/tree/main/models/face_recognition_sface) |
| **Face detector** D2.01 / mesh frontend | OpenCV Zoo **YuNet** | MIT | ONNX | [opencv_zoo face_detection_yunet](https://github.com/opencv/opencv_zoo/tree/main/models/face_detection_yunet) |
| **General detector / backbone** D2.01, D2.03 | **YOLO-NAS** (Deci SuperGradients) | Apache 2.0 | ONNX / TFLite | [super-gradients](https://github.com/Deci-AI/super-gradients) |
| **Affect** D3.13 | OpenVINO `emotions-recognition-retail-0003` **or** MediaPipe blendshapes heuristic | Apache 2.0 (OpenVINO OMZ) / N/A heuristic | IR/ONNX or pure logic | [Open Model Zoo emotions-recognition-retail-0003](https://github.com/openvinotoolkit/open_model_zoo/tree/master/models/intel/emotions-recognition-retail-0003) |
| **Liveness / PAD** D3.09 | **Active challenge-response** on mesh + blendshapes (no brittle passive RGB ML) | Design + our code | Source | Spec below |

**Avoid for commercial product default:** models trained primarily on MS-Celeb-1M / AffectNet-class non-commercial datasets unless counsel clears weight use.

---

## 2. Local layout (when fetched)

Do **not** commit multi‑MB weights to the Qualia git tree unless Timothy explicitly wants them tracked.

```text
{storage}/models/vision/
  README.md                 # copy of diligence notes
  face_landmarker.task      # or .tflite / .onnx — MediaPipe
  sface.onnx                # embedding
  yunet.onnx                # face detect
  yolo_nas_s.onnx           # optional general detector
  emotions_retail.onnx      # optional; or omit and use blendshapes
  MANIFEST.json             # hashes, licence, gate ids
```

`MANIFEST.json` fields: `id`, `gate`, `filename`, `sha256`, `licence`, `source_url`, `format`, `fetched_unix`.

---

## 3. Integration adapters (Qualia side)

| Asset | Adapter home (single-function / subdir law) | Public ABI |
|-------|-----------------------------------------------|------------|
| YuNet ONNX | `biosense/face/yunet_detect_onnx.rs` (or `cv/detect/`) | boxes → existing detector path |
| MediaPipe mesh | `biosense/face_mesh/mediapipe_landmarker.rs` | landmarks → ROI / rPPG / PAD / EVM |
| SFace ONNX | `biosense/biometrics/sface_embed_onnx.rs` | embedding → sanctuary template store (not NQuin) |
| YOLO-NAS | `cv/detect/yolo_nas_onnx.rs` | multi-class boxes → epistemic quins |
| Emotion OMZ | `biosense/affect/openvino_emotion_onnx.rs` | **proposals** + uncertainty |
| Challenge PAD | `biosense/liveness/challenge_pad.rs` | permit/deny + reason codes |

**Runtime:** prefer pure-Rust ONNX runtime (e.g. `ort` / tract — evaluate licence + size) or TFLite via thin FFI only if unavoidable; **no Python**. Product must not require OpenCV C++ as the vision engine (Zoo models are **weight sources**, not ABI owners).

---

## 4. Pure-landmark challenge PAD (principal architecture)

**Goal:** Presentation attack detection from **3D face mesh landmarks only** — no pixel-level texture, no screen-glare / RGB spoof nets, no massive anti-spoof training sets.

**Implementation:** `crates/qualia-vision/src/biosense/liveness/` (single-function modules).

### 4.1 Pipeline (strict order)

| Step | Module | Gate |
|------|--------|------|
| 0 | Consent + `camera_stream_integrity` | Fail closed: no consent; virtual camera when `require_physical` |
| 1 | `temporal_window` | **TTS ~800 ms** start; **TTC ~2000 ms** complete — blocks replay brute-force |
| 2 | `rigid_head_pose` | PnP-class pose from nose/chin/outer eyes vs canonical 3D model → pitch/yaw/roll °; scale by interocular |
| 3 | `action_threshold` | Challenge math: yaw ≥ 25°; mouth gap / IOD; blendshape smile/blink when present |
| 4 | `non_rigid_z` | **Core lock:** nose–cheek projected ratio residual after linear yaw fit — flat masks fail |
| 5 | `landmark_jitter` | ~1 s noise floor — static mask / over-smooth replay fail |

Entry: `evaluate_landmark_pad(...)`. Legacy pose/blend row path: `evaluate_challenge_pad(...)`.

### 4.2 Challenge set

| Id | Action | Pass criterion (defaults) |
|----|--------|---------------------------|
| `yaw_left` / `yaw_right` | Head turn | \|Δyaw\| ≥ 25° + non-rigid Z |
| `pitch_up` / `pitch_down` | Nod | pitch Δ + non-rigid Z when applicable |
| `smile` / `open_mouth` | Expression | mouth ratio / IOD or blendshape peak |
| `blink_2` | Blink twice | two blink peaks in window |

Prefer `issue_rotation_challenge(seed)` for stronger 3D PAD.

### 4.3 Attack coverage (geometric)

| Attack | Why it fails |
|--------|----------------|
| Printed photo / paper cutout | Rigid ratios under yaw → `FlatSurface` |
| Static 3D mask (held still) | Jitter too low → `StaticMesh` |
| Screen replay (slow response) | TTS/TTC miss |
| Smooth deepfake mesh inject | Jitter anomaly and/or virtual camera |
| Virtual camera avatar inject | `CameraStreamAttestation` + OS hardware path |

### 4.4 Outputs / reasons

`PadReason`: `Pass` | `NoConsent` | `TimeToStartExceeded` | `TimeToCompleteExceeded` | `WrongAction` | `StaticMesh` | `FlatSurface` | `VirtualCamera` | `UnattestedStream` | `JitterAnomaly` | `PoseUnavailable` | `InsufficientFrames` | `Timeout`

### 4.5 Non-claims

- PAD is **not** identity proof alone — combine with 1:1 template under selfhood consent.
- Landmark integrity depends on the mesh adapter; pair with **physical CMOS attestation** in production hosts.
- Thresholds are defaults until calibrated and recorded in `MANIFEST.json`.

---

## 5. Affect strategy (two paths)

| Path | When | Gate close |
|------|------|------------|
| **A. Blendshapes heuristic** | Prefer zero affect-model risk | Map MediaPipe blendshapes → valence/arousal **proposals** with high uncertainty; no OMZ download |
| **B. OpenVINO retail emotion** | Principal wants discrete classes | Fetch OMZ model; still **proposal-only** UI; diligence on retail model card |

Default for excellence ship: **Path A** until principal fetches Path B.

---

## 6. Gate closure checklist (principal)

- [ ] Download models into local `models/vision/`  
- [ ] Fill `MANIFEST.json` with sha256 + licence quotes  
- [ ] Confirm commercial use OK for **weights** (and note training-data residual risk if any)  
- [ ] Sign off challenge PAD as production PAD for unlock path  
- [ ] Optional: contact PPG pack for rPPG clinical gate (separate from this pack)  
- [ ] Agent wires loaders + registry flips CompleteWithGate → Present where honest  

---

## 7. Explicit non-goals

- Shipping MS-Celeb / AffectNet–tainted commercial defaults  
- AGPL YOLO (v8/v9 family) as closed-source product detector without licence review  
- Passive RGB PAD as sole security control  
- Committing multi‑GB weights to the public git tree without principal request  

---

*Pack curated for Qualia vision excellence gates. Code integrates; principal acquires weights.*
