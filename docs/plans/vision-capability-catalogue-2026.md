# Vision Capability Catalogue → Qualia Implementation Map

**Branch:** `0.0.25`  
**Source:** Principal-supplied industry function catalogue (Physiological → Specialty Optics + Decentralized)  
**Living to-do:** `native-vision-capability-excellence-PROGRESS-LOG.md` § Implementation to-do  
**Registry spine:** D1–D9 in `qualia-vision` capability registry  

This document is the **full backlog map**. It does **not** claim excellence for unbuilt rows.  
Licence preference: Apache-2.0 / MIT / other commercial-OK weights; no Python product path; no OpenCV product ABI.

### Status legend

| Tag | Meaning |
|-----|---------|
| **Present** | Library path exists and is honest for scope claimed |
| **Partial** | Lite / scaffold / incomplete excellence bar |
| **Queued** | On implementation to-do (agent-executable when prioritised) |
| **WeightAbsent** | Permissive (MIT/Apache) weight not on disk — run `vendor/vision/download.ps1` |
| **AdapterMissing** | Weight OK or pure-Rust path; loader/algorithm not wired |
| **TrainingDeferred** | Principal fine-tune later — **does not** block published weights |
| **Gated** | Needs product demand, clinical corpus, or true LicenceHostile diligence |
| **Policy** | Must pass deontic / consent / CCTV purpose before enable |
| **OOS** | Out of product scope (or defence-only; do not ship offensive capability) |
| **Wire** | Exists elsewhere in monorepo; vision programme = integrate |

**Licence honesty:** YuNet **MIT**, SFace / MediaPipe / YOLO-NAS / OMZ emotion **Apache-2.0** are **PermissiveReady** under `vendor/vision/`. Do **not** call them commercial-licence gated.

### Priority waves (orchestrator)

| Wave | Focus | First TODO IDs |
|------|--------|----------------|
| **W0** | In-flight excellence (biosense locks + EVM) | TODO-EVM1, TODO-MESH1, TODO-PAD*, TODO-ONNX1 |
| **W1** | Physiological depth | TODO-RR1, TODO-SPO2, TODO-PUPIL, TODO-MICROX |
| **W2** | Tracking & pose | TODO-POSE, TODO-HAND, TODO-MOT, TODO-GAZE |
| **W3** | Scene / geometry learning | TODO-SEG, TODO-DEPTH, TODO-SAL, TODO-VO |
| **W4** | Image restoration | TODO-SR, TODO-LL, TODO-DEBLUR, TODO-INPAINT, TODO-STAB |
| **W5** | Document / industrial | TODO-OCR, TODO-LAYOUT, TODO-DEFECT, TODO-QR |
| **W6** | Policy-heavy verticals | TODO-SURV* (Policy), ADAS, retail — principal demand only |
| **W7** | Medical / ag / robotics / remote | TODO-CELL, TODO-AG*, TODO-ROB*, TODO-RS* |
| **W8** | Vision-language + local-first embeddings | TODO-VLM*, TODO-P2P* |

---

## 1. Physiological & Health

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Remote PPG (rPPG) | POS, CHROM, DeepPhys | D3.03 `rppg/` POS+CHROM+SNR; DeepPhys gated | Present (POS/CHROM); DeepPhys Gated | TODO-RPPG-DEEP |
| Eulerian Video Magnification | Spatial-temporal IIR/FIR | D3.06–07 lite; excellence | Partial → excellence | **TODO-EVM1** |
| Respiratory rate | Optical flow, EVM, rPPG harmonics | D3.05 motion energy; deepen | Present lite | TODO-RR1 |
| Pupillometry | Daugman, CNN | New D3.x / face iris ROI | Queued | TODO-PUPIL |
| SpO₂ remote | Multi-λ / RGB ratio rPPG | New D3.x; clinical honesty | Queued + Gated | TODO-SPO2 |
| Micro-expression | 3D-CNN, flow+SVM | D3.14 AU-lite / temporal mesh | Partial | TODO-MICROX |

**Policy:** all rows fail closed without purpose-bound consent; non-diagnosis UI for clinical-adjacent outputs.

---

## 2. Biometrics & Liveness

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Gaze tracking | MediaPipe Iris, Gaze360 | After mesh/iris landmarks | Queued | TODO-GAZE |
| Gait analysis | GaitSet, silhouette/pose RNN | D2 pose + track; high sensitivity | Queued + Policy | TODO-GAIT |
| Periocular recognition | ResNet/ArcFace fine-tune | D3.10 vault family | Gated | TODO-PERIOC |
| Deepfake AV sync detect | SyncNet, LipForensics | Defence PAD adjacent | Queued | TODO-AVSYNC |
| Kinematic liveness | Pose + LSTM; blink/swallow | D3.09 + mesh events | Partial (PAD/jitter) | TODO-KINLIV |
| Active PAD (challenge) | 3D mesh + temporal heuristics | D3.09 pure-landmark PAR | **Present** | TODO-PAD1/2 (calib/attest) |

**OOS:** Deepfake *generation* / face-swap tools as product features (detection only).

---

## 3. Tracking & Kinematics

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Multi-object tracking | DeepSORT, ByteTrack, BoT-SORT | D2.01 BoundedTracker + grid | Partial | TODO-MOT |
| Human pose (2D/3D) | OpenPose, MoveNet, MP Pose | D2.06 | Missing → Queued | TODO-POSE |
| Hand articulation / gesture | MediaPipe Hands, HaGRID | D2.06 family | Queued | TODO-HAND |
| Action recognition | SlowFast, I3D | Learned temporal | Gated | TODO-ACT |
| 6D object pose | PoseCNN, DOPE, DenseFusion | Robotics/AR | Queued | TODO-6D |
| Skeleton action prediction | ST-GCN | After pose | Gated | TODO-SKELPRED |

---

## 4. Scene & Geometry

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Monocular depth | MiDaS, Depth Anything | D2.05 | Missing | TODO-DEPTH |
| Panoptic segmentation | Mask2Former | D2.04 family | Missing | TODO-PAN |
| Semantic segmentation | DeepLab, U-Net, SegFormer | D2.04 | Missing | TODO-SEG |
| Instance segmentation | Mask R-CNN, YOLO-Seg | D2.04 | Missing | TODO-INST |
| Visual odometry / SLAM | ORB-SLAM3, RTAB-Map | D5 recon / new | Missing | TODO-VO |
| Salient object detection | U²-Net | BG remove / ROI | Queued | TODO-SAL |

Related Present: heightfield image→3D (D5.06), MeshIR, STL, print readiness.

---

## 5. Image Processing

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Super-resolution | FSRCNN/ESPCN + Real-ESRGAN + SwinIR (native tiers) | D1.16 / D2.10–13 | Queued | **TODO-SR** → `native-super-resolution-excellence-2026.md` |
| Low-light enhancement | Zero-DCE, LLNet | D1 photo family | Queued | TODO-LL |
| Motion deblurring | MPRNet, NAFNet | D1 photo | Queued | TODO-DEBLUR |
| Image inpainting | EdgeConnect / local gen | D1.14 denoise present; inpaint | Partial | TODO-INPAINT |
| Video stabilization | Feature + MeshFlow | Needs video I/O | Queued | TODO-STAB |
| Style transfer / colorize | CycleGAN, DeOldify | Generative honesty | Gated | TODO-STYLE |

Classical floor Present: filters, morph, edges, hist, warp, features, flow, bilateral denoise, draw.

---

## 6. Document & Text

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| OCR | Tesseract, EasyOCR, PaddleOCR | D2.07 | Missing | TODO-OCR |
| Document layout | LayoutLMv3, YOLOX | After OCR | Gated | TODO-LAYOUT |
| Handwriting (HTR) | CRNN, TrOCR | D2.07 family | Gated | TODO-HTR |
| Signature verification | Siamese, DTW | Biometric-class Policy | Gated + Policy | TODO-SIG |
| Key information extraction | LayoutXLM, Donut | After layout | Gated | TODO-KIE |

---

## 7. Industrial & Inspection

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Surface defect detection | PatchCore, Mask R-CNN | Vertical | Queued | TODO-DEFECT |
| Unsupervised anomaly | PaDiM, FastFlow | Vertical | Queued | TODO-ANOM |
| Thermal / IR analysis | FLIR CNNs | Capture modality | Gated | TODO-THERM |
| Dimensional metrology | Sub-pixel edges, calib | D1 edges + calib | Partial | TODO-METRO |
| Barcode / QR decode | ZBar, ZXing, BoofCV | Pure Rust preferred | Queued | TODO-QR |

---

## 8. Security & Surveillance (**Policy-first**)

All rows require **D4** purpose, jurisdiction, and fail-closed CCTV mode. No silent biometric extract.

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Crowd density | CSRNet, DM-Count | Policy vertical | Queued + Policy | TODO-SURV-CROWD |
| Abandoned object | BG subtract + track | Policy | Queued + Policy | TODO-SURV-ABAND |
| Weapon / threat detect | Fine-tuned YOLO | **Principal-only vertical**; high false-positive risk | Gated + Policy | TODO-SURV-THREAT |
| Loitering | DeepSORT + temporal logic | Policy + track | Queued + Policy | TODO-SURV-LOIT |
| Virtual tripwire | Vector math + detector | Policy | Queued + Policy | TODO-SURV-TRIP |

---

## 9. Automotive & ADAS

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| ANPR / ALPR | LPRNet, OpenALPR | Policy (plates = sensitive) | Gated + Policy | TODO-ANPR |
| Lane detection | SCNN, LaneATT | ADAS vertical | Gated | TODO-LANE |
| Driver drowsiness | Landmarks + PERCLOS | Biosense + Policy (vehicle) | Queued | TODO-DROWSY |
| Vehicle speed estimate | Homography + flow | Calib required | Gated | TODO-SPEED |
| Traffic flow / class | YOLO + ByteTrack | After MOT | Gated | TODO-TRAFFIC |

---

## 10. Retail & Commerce

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Planogram compliance | Det + graph match | Vertical | Gated | TODO-PLANO |
| Customer heatmapping | MOT + KDE | **Policy** (people tracking) | Gated + Policy | TODO-HEAT |
| Virtual try-on | VITON-HD, makeup GAN | Generative | Gated | TODO-VTON |
| Checkout association | Multi-cam 3D track | Policy | Gated + Policy | TODO-CHECKOUT |
| Fine-grained product recog | Triplet / ArcFace | Embeddings | Gated | TODO-SKU |

---

## 11. Medical & Science

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Cell count / segment | Cellpose, U-Net | D7.03–04 | Missing | TODO-CELL |
| Radiological anomaly | CheXNet, 3D U-Net | D7.01; **non-diagnosis** | Gated + Policy | TODO-RAD |
| Surgical tool tracking | YOLO / Mask R-CNN | Clinical vertical | Gated + Policy | TODO-SURG |

Wire: `medical_computing`, Anatomy QApp, sensitivity routing (Partial Present).

---

## 12. Vision-Language

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| VQA | LLaVA, BLIP-2 | Local LLM + vision | Gated | TODO-VQA |
| Image retrieval (CBIR) | CLIP + FAISS | Local-first embeddings | Queued | TODO-CBIR |
| Image captioning | BLIP, GIT | Local | Gated | TODO-CAPTION |
| Zero-shot detection | Grounding DINO, GLIP | D2 detector family | Gated | TODO-ZSD |
| Open-vocab segmentation | CLIPSeg, SAM | D2.04 family | Gated | TODO-OVS |

---

## 13. Agriculture & Environment

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Crop disease ID | MobileNet/ResNet | Vertical | Gated | TODO-AG-DISEASE |
| Weed detect / spray | Edge YOLO | Vertical | Gated | TODO-AG-WEED |
| Canopy cover (ExG/ExR) | Colour threshold | Classical D1 | Queued | TODO-AG-CANOPY |
| Animal biometrics | Triplet CNN + track | Policy (animal/farm) | Gated | TODO-AG-ANIMAL |

---

## 14. Robotics & Control

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Grasp pose | GraspNet, Dex-Net | 6D + depth | Gated | TODO-GRASP |
| Visual servoing | Jacobian + features | Control loop | Gated | TODO-SERVO |
| Obstacle avoidance nav | V-SLAM, RL | After VO/depth | Gated | TODO-NAV |

---

## 15. Media & Broadcast

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Auto camera framing | MOT + PID | After MOT | Queued | TODO-FRAME |
| Highlight extraction | Spatiotemporal + audio | Cross-modal D8 | Queued | TODO-HIGHLIGHT |
| Deepfake generation / swap | SimSwap, Roop | **OOS product** | **OOS** | — (detect only: TODO-AVSYNC) |

---

## 16. Remote Sensing (Satellite / Drone)

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Building footprints | Mask R-CNN, DeepGlobe | Vertical | Gated | TODO-RS-BUILD |
| Change detection | Siamese, ChangeFormer | Vertical | Gated | TODO-RS-CHANGE |
| Vehicle/ship detect | YOLT-class | Vertical | Gated | TODO-RS-ASSET |

---

## 17. Decentralized & P2P (W3C / Solid / WebID) — **Qualia-native priority**

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Local-first visual embedding search | ONNX CLIP/ResNet | Library + ONNX; no cloud required | Queued | **TODO-P2P-EMB** |
| Privacy-preserving feature extract | Edge ML; HE optional | Privacy engine wire | Queued | **TODO-P2P-PRIV** |
| On-device personal analytics → graph | Edge YOLO/light transformer | Biosense + SPARQL-MM quins | Partial recipes | **TODO-P2P-GRAPH** |

These align with selfhood, sanctuary, and “no raw RGB off-device by default.”

---

## 18. Specialty Optics

| Function | Methods (ref) | Qualia map | Status | TODO |
|----------|---------------|------------|--------|------|
| Hyperspectral analysis | 3D-CNN, PCA | Multi-band buffer path | Gated | TODO-HSI |
| Schlieren / BOS tracking | Optical flow | D1.09 + specialty | Gated | TODO-SCHLIEREN |

---

## Cross-cutting already Present (do not re-implement)

- Classical CV kernels (D1.01–09, 13–14)  
- Challenge PAD + PAR (D3.09) — pure 2D landmark lock  
- rPPG POS/CHROM + spectral HR (D3.03)  
- Consent / deontic processing act / CCTV stage filter (D4)  
- MeshIR, STL, print readiness, twin A1 preview (D5)  
- Semantic vision quins + reject/correct (D2.02)  
- Perception library catalogue (D9.03)

---

## Explicit non-goals

1. **Deepfake generation / face swap as shipped product** — OOS; detection and PAD only.  
2. **Silent surveillance analytics** without purpose-bound permit.  
3. **Clinical diagnosis claims** from software alone (assurance A0–A4).  
4. **Model-Z as liveness depth** — forbidden (PAR is 2D \(x\) only).  
5. **Python / OpenCV product ABI** — weights may be zoo-sourced; runtime is Rust.

---

## How agents use this file

1. Pick a **TODO-*** from the progress-log queue or a Wave above.  
2. CLAIM exclusive paths in `coordination/NOTICES.md`.  
3. Single-function modules; registry honesty update.  
4. Progress-log entry with measured tests.  
5. Do not mark Present until excellence bar for that row is met (or honest Partial/Gated).
