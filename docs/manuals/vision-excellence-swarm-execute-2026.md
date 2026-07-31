# Vision Excellence — Swarm Execute Plan (asset-backed)

**Date:** 2026-07-17  
**Branch:** `0.0.28`  
**Canonical tree only:** `C:\Projects\qualia-27062026`  
**Assets:** `vendor/vision/` + `MANIFEST.json` + `download.ps1`  
**Catalogue:** `vision-capability-catalogue-2026.md`  
**Progress log:** `native-vision-capability-excellence-PROGRESS-LOG.md`

## 0. Corrected gate language (mandatory for all agents)

| Old (wrong) phrasing | Correct |
|----------------------|---------|
| “Commercial licence gate” for YuNet/SFace/MediaPipe/YOLO-NAS | **False.** Those are **MIT / Apache-2.0**. |
| COMPLETE-WITH-GATE = “need to buy a licence” | **False** unless tag is `LicenceHostile`. |
| Waiting on principal training | **TrainingDeferred** — does **not** block adapters on published weights. |

**Actual remaining blockers for pack models:**

1. **WeightAbsent** — run `.\vendor\vision\download.ps1`  
2. **AdapterMissing** — wire ONNX/TFLite loader in Rust  
3. **Policy** — consent / deontic (biosense / CCTV)  
4. **TrainingDeferred** — fine-tune later; ship inference first  

---

## 1. Pre-flight (orchestrator or Timothy, once)

```powershell
cd C:\Projects\qualia-27062026
.\vendor\vision\download.ps1
# Expect: yunet, sface, mediapipe_*, present or manual notes for yolo_nas / emotions
```

Append CLAIM to `coordination/NOTICES.md` when spawning tracks.

**Principal off-machine:** Training corpora / fine-tunes wait. Swarm continues on **published PermissiveReady weights** + pure-Rust algorithms (EVM, PAR, rPPG — no weights).

---

## 2. Parallel tracks (exclusive paths — no collision)

| Track | TODO / scope | Exclusive paths | Depends on | Priority |
|-------|--------------|-----------------|------------|----------|
| **S0-ASSET** | Manifest resolve + path helpers | `qualia-vision/src/weights/` or `biosense/assets/` | — | **P0 now** |
| **S1-EVM** | TODO-EVM1 excellence EVM | `biosense/magnification/` | S0 optional | **P0 now** |
| **S2-ONNX** | TODO-ONNX1 ort/tract + load YuNet/SFace | `biosense/face/`, `biosense/biometrics/`, Cargo features | S0 + weights | **P0 now** |
| **S3-MESH** | TODO-MESH1 MediaPipe → `LandmarkFrame` | `biosense/face_mesh/` | S2 or TFLite path | **P0** |
| **S4-PAD** | Wire mesh into PAD; attest hooks | `biosense/liveness/` (careful merge) | S3 | P1 |
| **S5-DET** | YOLO-NAS / YuNet → detector path | `cv/detect/` or detector.rs adapters | S2 | P1 |
| **S6-POSE** | TODO-POSE / HAND MediaPipe | `biosense/pose/` | S0 + download | P1 |
| **S7-RPPG** | TODO-RR1 deepen respiration | `biosense/rppg/`, `respiration/` | — | P1 |
| **S8-P2P** | TODO-P2P-EMB local embeddings | `biosense/embeddings/` or vision embeddings | S2 | P2 |
| **S9-POL** | TODO-FED1 policy | sparql/deontic claimed files only | — | P2 |
| **S10-UI** | Studio/desktop load vendor path | desk/studio vision cmds | S2–S4 APIs | P2 |

**Serial only when sharing:** `commands/mod.rs`, `biosense/mod.rs` re-exports — one agent appends, or land via PR stack.

---

## 3. Wave schedule (can run same day without training)

### Wave A — Unblock (hours)

| Job | Acceptance |
|-----|------------|
| A1 | `vendor/vision` tree + MANIFEST + download.ps1 **done** (this commit) |
| A2 | `resolve_vision_asset(id) -> Path` in qualia-vision; tests with missing→err, present→ok |
| A3 | Download on build machine; FETCH.json sha256 recorded |
| A4 | Registry honesty: D2.01/D3.01/D3.10 = **CompleteWithGate → AdapterMissing** language (not commercial licence) |

### Wave B — Pure Rust excellence (parallel, no weights)

| Job | Acceptance |
|-----|------------|
| B1 | **TODO-EVM1** a–e (pyramid, bandpass, colour/motion, SNR) |
| B2 | RR1 deepen if free |
| B3 | PAR/PAD tests stay green |

### Wave C — ONNX adapters (weights on disk)

| Job | Acceptance |
|-----|------------|
| C1 | Feature `onnx` / `ort` (or `tract`) optional; fail closed if missing |
| C2 | YuNet detect → boxes into existing detector/NMS path |
| C3 | SFace embed → fixed buffer → template_hash / vault path |
| C4 | MediaPipe landmarker → `LandmarkFrame` + blend proxies → `evaluate_landmark_pad` |
| C5 | Integration test: **if weight present** run; else `#[ignore]` with reason WeightAbsent |

### Wave D — Pose / detect expand

| Job | Acceptance |
|-----|------------|
| D1 | Pose + hands landmarker load |
| D2 | YOLO-NAS when file placed (manual_place) |
| D3 | MOT association improvement (algorithmic) |

### Wave E — Surfaces

| Job | Acceptance |
|-----|------------|
| E1 | Desktop/Studio pick `vendor/vision` path |
| E2 | Library catalogue lists PermissiveReady assets + licence tags |

---

## 4. Single-track agent prompt (copy-paste)

```text
You are Track [S#] on Qualia vision excellence swarm (branch 0.0.28).
Canonical tree: C:\Projects\qualia-27062026 only. No worktrees.

Read:
- docs/plans/vision-excellence-swarm-execute-2026.md
- vendor/vision/README.md + MANIFEST.json
- docs/plans/vision-capability-catalogue-2026.md (status only)

Rules:
- MIT/Apache pack models are PermissiveReady — NEVER call them commercial-licence gated.
- Weights live under vendor/vision/<subdir>/; resolve via MANIFEST.
- Single-function .rs files; library subdirs; zero-heap hot paths.
- CLAIM/RELEASE coordination/NOTICES.md exclusive paths.
- No Python product path. OpenCV is weight source only.
- TrainingDeferred is OK to leave; do not block on principal fine-tune.
- cargo test -p qualia-vision --lib must pass; ignored tests only for WeightAbsent.

Scope: [fill from track table]
Done when: acceptance row for your track + progress-log entry + honest registry string.
```

---

## 5. Dependency DAG (spawn order)

```text
        S0-ASSET
       /    |    \
   S1-EVM  S2-ONNX  S7-RPPG
             |
          S3-MESH
         /   |   \
      S4-PAD S5-DET S6-POSE
             |
          S10-UI
   S8-P2P ──┘   S9-POL (independent)
```

Recommended **first swarm spawn (3 agents):** S0-ASSET + S1-EVM + S2-ONNX (S0 finishes first if serial bottleneck on `lib.rs` exports).

---

## 6. What waits for Timothy (machine off / later)

| Item | Tag |
|------|-----|
| Custom fine-tunes / private corpora | TrainingDeferred |
| Force-git-LFS multi-hundred-MB packs | Principal choice |
| YOLO-NAS export if not published as single ONNX URL | manual_place |
| Clinical SpO₂ validation corpus | Gated corpus, not licence |
| Policy text for CCTV jurisdictions | Policy |

---

## 7. Registry flip rules (after adapters)

| Gate | Flip Present when |
|------|-------------------|
| D2.01 | YuNet (or YOLO-NAS) loads + boxes tested with weight **or** honest Partial if only grid |
| D3.01 | MediaPipe → LandmarkFrame tested |
| D3.06–07 | EVM1 a–e green (no weight) |
| D3.09 | Already Present (PAR); stays Present |
| D3.10 | SFace embed into vault path tested |

Never flip on “licence cleared” alone — licence was already permissive.

---

## 8. Orchestrator checklist

- [ ] `download.ps1` run; list which FETCH.json exist  
- [ ] CLAIM S0, S1, S2  
- [ ] Land A2 path resolver  
- [ ] Land B1 EVM excellence  
- [ ] Land C2–C4 adapters  
- [ ] Progress log wave entry  
- [ ] Push `0.0.28`  

**Execute phrase:** `execute vision swarm` / `execute S0 S1 S2`
