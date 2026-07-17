# MediaPipe Face Landmarker

| Field | Value |
|-------|--------|
| Licence | **Apache-2.0** |
| Tag | **PermissiveReady** |
| Gates | D3.01 mesh, D3.09 PAD landmarks, D3.13 blendshapes Path A |
| File | `face_landmarker.task` (or ONNX/TFLite export) |

Feeds `LandmarkFrame` for PAR PAD (2D **x** only — never model Z).

```powershell
..\..\download.ps1 -Assets mediapipe_face_landmarker
```
