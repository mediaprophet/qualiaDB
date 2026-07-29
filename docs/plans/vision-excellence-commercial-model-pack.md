# Vision Excellence — Permissive Model Pack (vendor assets)

**Date:** 2026-07-17 (revised licence honesty)  
**Purpose:** Principal-curated **MIT / Apache-2.0** offline models so swarm jobs load real weights without inventing a “commercial licence” wall. OpenCV Zoo / MediaPipe / OMZ are **weight sources only** — not the product ABI.  
**Formats preferred:** ONNX / TFLite; Qualia adapters convert to internal buffers.  
**Status:** Pack + `vendor/vision/` layout + `download.ps1`. Large binaries gitignored; fetch per machine.

**Correct tags:** `PermissiveReady` | `WeightAbsent` | `AdapterMissing` | `TrainingDeferred` | `Policy` | `LicenceHostile`  
**Incorrect:** calling YuNet/SFace/MediaPipe/YOLO-NAS “commercial licence gated.”

---

## 0. Diligence rule (always)

| Check | Required |
|-------|----------|
| Code licence | Apache-2.0 / MIT preferred |
| **Weight** licence | SPDX for the **weight file** (MIT/Apache pack is OK for product use) |
| **Training data** | Prefer non-hostile corpora; residual risk is **DiligenceNote**, not a paywall for Apache zoo weights |
| Redistribution | MANIFEST + licence texts under `vendor/vision/licenses/` for installers |

`CompleteWithGate` means **WeightAbsent or AdapterMissing**, not “buy a commercial licence,” unless tag is `LicenceHostile`.

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

## 2. Local layout (canonical)

```text
vendor/vision/                    # developer + swarm (canonical)
  MANIFEST.json
  download.ps1
  face/yunet/*.onnx               # MIT
  face/sface/*.onnx               # Apache-2.0
  face/mediapipe_landmarker/*     # Apache-2.0
  detect/yolo_nas/                # Apache-2.0 (manual place OK)
  …
bundled/models/vision/            # package mirror (same layout)
{storage}/models/vision/          # user data dir override
```

```powershell
.\vendor\vision\download.ps1
```

`FETCH.json` per asset records sha256 after download. Swarm plan: `vision-excellence-swarm-execute-2026.md`.

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
| 4 | `profile_asymmetry_ratio` (PAR) | **Core lock:** raw 2D \(x\) only on MP **1 / 234 / 454**. \(\Delta PAR = \|PAR(t_1)/PAR(t_0)-1\| > \tau\) (τ≈0.6), yaw ≥ 25°. **Never use model Z** (hallucinates depth on flat screens). |
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
