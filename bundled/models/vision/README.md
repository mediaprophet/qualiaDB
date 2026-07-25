# Bundled vision models (install layout)

**Preferred developer tree:** `vendor/vision/` (see that README + `download.ps1`).

This directory is the **install-time / package** mirror. Adapters resolve in order:

1. `vendor/vision/`  
2. `bundled/models/vision/` (this tree)  
3. `{storage}/models/vision/`

## Licence honesty

YuNet **MIT**, SFace / MediaPipe / YOLO-NAS / OMZ emotion **Apache-2.0** are **PermissiveReady**.  
They are **not** blocked on “commercial licence.” Remaining states:

- **WeightAbsent** — run `.\vendor\vision\download.ps1` or copy files here  
- **AdapterMissing** — swarm wires ONNX/TFLite loaders  
- **TrainingDeferred** — principal fine-tune later (does not block published weights)

## Layout (match vendor)

```text
face/yunet/face_detection_yunet_2023mar.onnx
face/sface/face_recognition_sface_2021dec.onnx
face/mediapipe_landmarker/face_landmarker.task
detect/yolo_nas/yolo_nas_s.onnx
…
MANIFEST.json   # optional copy of vendor/vision/MANIFEST.json
```

Flat layout (`yunet.onnx` at this root) is also accepted by `resolve_vision_asset`.

## Swarm

`docs/plans/vision-excellence-swarm-execute-2026.md`
