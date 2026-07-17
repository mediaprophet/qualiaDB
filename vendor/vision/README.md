# Qualia vendor vision assets

**Purpose:** Local, licence-clean model **weights and manifests** so swarm jobs can load real files instead of waiting on a vague “commercial gate.”

**Rule:** OpenCV Zoo / MediaPipe / OMZ are **weight sources only**. Product ABI remains pure-Rust Qualia (`qualia-vision`). No OpenCV C++ runtime required.

## Licence honesty (read this)

| Tag | Meaning | Unblocks when |
|-----|---------|----------------|
| **PermissiveReady** | Code + weights claimed **MIT** or **Apache-2.0** (or equivalent) | Weight file present under this tree + adapter wired |
| **WeightAbsent** | Licence OK; binary not downloaded yet | Run `download.ps1` or place file manually |
| **AdapterMissing** | Weight may exist; Qualia loader not shipped | Swarm track implements ONNX/TFLite adapter |
| **TrainingDeferred** | Principal will train/fine-tune later (machine off) | Not blocking inference on published weights |
| **Policy** | Deontic / consent / CCTV purpose | Runtime permit, not a licence wall |
| **DiligenceNote** | Training-data residual risk (document only) | Does **not** equal “needs commercial licence” for Apache/MIT zoo weights |
| **LicenceHostile** | Non-commercial training-data / NC weight licence | Stay heuristic-only or counsel clear |

**Do not** label YuNet (MIT), SFace (Apache-2.0), MediaPipe (Apache-2.0), YOLO-NAS (Apache-2.0), or OMZ retail emotion (Apache-2.0) as “commercial licence gated.”  
They are **PermissiveReady** with possible **WeightAbsent** / **AdapterMissing** only.

## Layout

```text
vendor/vision/
  MANIFEST.json          # machine-readable pack index
  download.ps1           # fetch PermissiveReady weights
  licenses/              # upstream licence texts (tracked)
  face/
    yunet/               # MIT — face detect ONNX
    sface/               # Apache-2.0 — face embed ONNX
    mediapipe_landmarker/# Apache-2.0 — mesh / blendshapes
  detect/
    yolo_nas/            # Apache-2.0 — general detect
  affect/
    openvino_emotions_retail/  # Apache-2.0 optional Path B
  pose/
    mediapipe_pose/
    mediapipe_hands/
  depth/
    midas_lite/          # optional; licence per pack entry
  embeddings/
    clip_lite/           # local-first search (W8)
  seg/  ocr/             # placeholders until pack entry filled
```

**Git policy:** Track manifests, README, licence texts, download script, `.gitkeep`.  
**Ignore** large binaries (`*.onnx`, `*.tflite`, `*.task`, `*.bin`) so the tree stays cloneable; download on each machine (or Timothy can force-add if he wants LFS later).

**Runtime resolution order** (adapters):

1. `vendor/vision/<family>/<model>/` (this tree)  
2. `bundled/models/vision/` (optional install copy)  
3. `{storage}/models/vision/` (user data dir)

## Fetch (this machine)

```powershell
# From repo root
.\vendor\vision\download.ps1
# or selective:
.\vendor\vision\download.ps1 -Assets yunet,sface
```

Training / fine-tune corpora are **out of this pack** (TrainingDeferred). Inference weights only.

## Swarm

See `docs/plans/vision-excellence-swarm-execute-2026.md`.
