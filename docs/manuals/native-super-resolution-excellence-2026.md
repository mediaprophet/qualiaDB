# Native Super-Resolution Excellence (2026)

**Branch:** `0.0.25`  
**Owner:** Qualia vision / `qualia-vision`  
**Ambition:** Production-grade SR **as a native Rust library** inside Qualia — not OpenCV product ABI, not Python, not NCNN as a required runtime. External projects supply **permissive weights + reference algorithms**; Qualia owns inference, tiling, GPU path, and honesty.  
**Status:** Plan ready for execute (swarm tracks SR0–SR8).

---

## 0. Design principles (immovable)

| # | Principle |
|---|-----------|
| 1 | **Native core first.** Bicubic / Lanczos / edge-directed / sub-pixel CNNs implemented in Rust under `cv/sr/` and `sr/`. Optional learned backends load **ONNX (ort feature)** or **WGSL Forge** kernels — never “call OpenCV dnn_superres” as the product. |
| 2 | **Permissive licence only as defaults.** MIT / Apache-2.0 / BSD for **code and preferred weight files**. Training-data residual risk is a **DiligenceNote**, not a fake “commercial licence gate” on Apache zoo weights. |
| 3 | **Three compute tiers** (your taxonomy), one unified API. |
| 4 | **Tiling is mandatory** for edge/GPU tiers — VRAM/RAM scales with **input** tile size, not marketing claims. |
| 5 | **No texture hallucination as truth.** GAN paths must label outputs as *generative enhancement*; medical/forensic paths prefer non-GAN or confidence maps. |
| 6 | **Caller-buffered hot paths** where possible; Tier-2 may use bounded scratch for tiles. |
| 7 | **Anti-monolith:** one primary algorithm per `.rs`; `sr/` as a library subdirectory. |

---

## 1. Problem framing (2026)

Production SR is a **cost × fidelity × licence** surface:

```text
                    fidelity
                       ▲
           SwinIR-class ●
                       │
         Real-ESRGAN ● │
                       │
              FSRCNN ● │
         bicubic/Lanczos ●────────────► FPS / edge budget
```

| Trap | Qualia response |
|------|-----------------|
| DIV2K / non-commercial train-data entangled GANs as **default** | Prefer weights with explicit commercial OK; document DiligenceNote; never ship NC as product default |
| GPL/AGPL viral runtimes | Avoid; NCNN as **optional research path only**, not core dependency |
| Full-frame Real-ESRGAN OOM | **Tiled inference** with overlap blend (default 256², overlap 16–32) |
| GAN oil-painting / fake detail | Dual mode: `EnhancementMode::Sharpen` vs `EnhanceMode::Generative`; medical uses Sharpen + abstain |
| OpenCV as product | Zoo **weights only** (FSRCNN/ESPCN ONNX/PB → convert once offline) |

---

## 2. Product surface (single entry)

```rust
// Conceptual ABI (to implement)
pub enum SrBackend {
    Classical(ClassicalKernel),   // always Present
    CnnLight,                     // FSRCNN / ESPCN — Ultra-Light
    EsrganEdge,                   // Real-ESRGAN compact — Balanced
    SwinRestore,                  // SwinIR-class — Heavy
}

pub struct SrRequest<'a> {
    pub rgb: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub scale: u8,                // 2 | 3 | 4
    pub backend: SrBackend,
    pub tile: TilePolicy,         // size, overlap, blend
    pub mode: EnhancementMode,    // Sharpen | Generative
}

pub fn super_resolve(req: &SrRequest, out: &mut [u8]) -> Result<SrReport, SrError>;
```

`SrReport`: backend id, scale, tile count, ms, peak scratch bytes, `generative: bool`, licence tag of weights used.

---

## 3. Three tiers (mapped to Qualia)

### Tier U — Ultra-Light (real-time video / CPU)

| Item | Choice |
|------|--------|
| **Algorithms (native)** | Lanczos-3, bicubic, optional **edge-directed** (NEDI-lite or similar) |
| **Learned (weights)** | **ESPCN** / **FSRCNN** (OpenCV dnn_superres lineage) — **Apache-2.0** weights as **PermissiveReady** assets |
| **Runtime** | Pure Rust CNN micro-engine **or** `ort` feature; **not** OpenCV C++ |
| **Target** | 480p→720p-class, multi-FPS on laptop CPU; zero GPU required |
| **Honesty** | Sharper than bicubic; **does not invent textures** |

**Vendor path:** `vendor/vision/sr/fsrcnn/` · `vendor/vision/sr/espcn/`  
**Module path:** `cv/sr/lanczos3.rs`, `cv/sr/bicubic.rs`, `sr/cnn_light/` (depth-to-space, conv)

### Tier B — Balanced (edge GPU / stills + light video)

| Item | Choice |
|------|--------|
| **Reference quality bar** | Real-ESRGAN family (MIT ecosystem), **compact** anime/video variants for edge |
| **Qualia path** | ONNX export of compact Real-ESRGAN → `ort` **or** native WGSL Forge operator graph when certified |
| **NCNN** | **Not** a product dependency. Optional offline conversion note: NCNN param/bin → ONNX for Qualia loaders |
| **Mandatory** | Overlapping **tile + feather blend**; fail closed if estimated scratch > budget |
| **Vulkan** | Prefer **wgpu** (already Qualia path) over linking NCNN Vulkan |

**Vendor path:** `vendor/vision/sr/realesrgan_compact/`  
**Module path:** `sr/tile_policy.rs`, `sr/tile_blend.rs`, `sr/esrgan_session.rs` (feature `ort`)

### Tier H — Heavy (high-fidelity stills)

| Item | Choice |
|------|--------|
| **Reference quality bar** | **SwinIR** (MIT) — transformer restoration / SR |
| **Runtime** | ONNX via `ort` (recent opset); later Forge if kernels certified |
| **Use** | Medical stills, forensic *enhancement* (with watermark/provenance), print/photo |
| **Not for** | Real-time video without datacenter GPU |
| **Honesty** | Lower “oil paint” than classic ESRGAN; still **not ground truth** |

**Vendor path:** `vendor/vision/sr/swinir/`  
**Module path:** `sr/swin_session.rs`

---

## 4. Native classical excellence (ship first — zero weights)

These establish the floor every other backend must beat in A/B tests:

| Kernel | File (proposed) | Notes |
|--------|-----------------|-------|
| Nearest | existing `ops/resize2d` | baseline |
| Bilinear | `cv/sr/bilinear_u8.rs` | video-safe |
| Bicubic (Keys) | `cv/sr/bicubic_u8.rs` | default classical |
| Lanczos-3 | `cv/sr/lanczos3_u8.rs` | stills |
| Edge-aware lite | `cv/sr/edge_directed_lite.rs` | optional; no learned weights |
| YUV upsample | `cv/sr/upsample_chroma.rs` | video 4:2:0 paths |

**Acceptance:** PSNR/SSIM on synthetic down→up vs nearest; visual smoke; zero heap in hot loops (caller buffers).

---

## 5. Learned pipeline architecture

```text
                    ┌──────────────┐
  RGB in ──────────►│ Tile planner │── tile rects + overlap
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
         Classical    CNN-Light     ESRGAN/Swin
         (always)     (ONNX/ort)    (ONNX/ort)
              │            │            │
              └────────────┼────────────┘
                           ▼
                    ┌──────────────┐
                    │ Seam blend   │── Hann / linear feather
                    └──────┬───────┘
                           ▼
                    RGB out + SrReport
```

### Tiling policy (production default)

| Parameter | Default | Notes |
|-----------|---------|-------|
| `tile` | 256 | Edge; 128 if RAM critical |
| `overlap` | 32 | ≥ receptive field / 4 |
| `blend` | linear or raised-cosine | Avoid seams |
| `max_tiles` | hard cap | Fail closed if exceeded |
| `budget_bytes` | caller or 256 MiB | Sentinel-friendly |

**Formula:** peak activations ∝ `tile² × channels × scale²` — document in MANIFEST.

### Colour handling

1. Optional convert to Y (or YUV): SR on **luma** first for light tier (ESPCN tradition).  
2. Heavy/GAN: full RGB if model expects it.  
3. Never SR in linear-light by accident without documenting gamma.

---

## 6. Licence & asset pack (PermissiveReady)

| Asset | Claimed licence | Role | Gate tag |
|-------|-----------------|------|----------|
| FSRCNN / ESPCN (OpenCV zoo lineage) | Apache-2.0 ecosystem | Tier U | PermissiveReady |
| Real-ESRGAN compact / animevideo (MIT community builds) | MIT | Tier B | PermissiveReady + DiligenceNote |
| SwinIR official / ONNX exports | MIT | Tier H | PermissiveReady + DiligenceNote |
| DIV2K-trained weights with NC terms | — | **Not product default** | LicenceHostile or exclude |

**Layout:**

```text
vendor/vision/sr/
  README.md
  MANIFEST.json
  download.ps1              # extend existing vendor/vision/download.ps1
  classical/                # no weights — docs only
  fsrcnn/
  espcn/
  realesrgan_compact/
  swinir/
  licenses/
```

**Diligence:** For each weight file record SPDX, source URL, sha256, train-data note, commercial redistribute yes/no.

---

## 7. Integration with Qualia

| Consumer | Use |
|----------|-----|
| `biosense` / medical stills | Prefer Classical + CNN-Light; Generative off by default |
| Desktop / Studio | “Enhance” control: tier picker + tile size |
| Library catalogue | List SR weights like MediaPipe pack |
| `wgpu` / Forge | Phase 2: certified WGSL SR kernels with CPU oracle |
| Provenance | Generation receipt quins: backend, scale, weight hash |

**Registry (proposed):**

| ID | Name | Target |
|----|------|--------|
| D1.16 | classical_sr | Present after Lanczos/bicubic |
| D2.10 | sr_cnn_light | CompleteWithGate → Present when FSRCNN/ESPCN loads |
| D2.11 | sr_esrgan_edge | CompleteWithGate + tile |
| D2.12 | sr_swin_heavy | CompleteWithGate |
| D2.13 | sr_tiling_runtime | Present with tile blend |

---

## 8. Honesty & non-claims

1. SR is **not** evidence recovery of ground-truth lost frequencies; Generative mode is **synthesis**.  
2. Medical / forensic UI: default **Sharpen**, show “enhanced” badge + backend id.  
3. Real-ESRGAN-class can invent textures — never use as sole PAD / identity evidence.  
4. OpenCV / Real-ESRGAN / SwinIR names in docs = **algorithmic lineage**, not bundled runtime.  
5. Training custom models = **TrainingDeferred** (principal); inference on published MIT/Apache weights proceeds without it.

---

## 9. Swarm execute plan (awesome path)

### Wave SR0 — Classical excellence (1 session)

- [ ] `cv/sr/` bicubic, Lanczos-3, bilinear  
- [ ] Unified `super_resolve` classical path  
- [ ] Synthetic PSNR tests vs nearest  
- [ ] Registry D1.16 Present  

### Wave SR1 — Tiling runtime (1 session)

- [ ] `tile_plan`, `tile_extract`, `tile_blend`  
- [ ] Budget fail-closed  
- [ ] Tests: seamless ramp image, max_tiles  

### Wave SR2 — CNN-Light (1–2 sessions)

- [ ] Vendor FSRCNN/ESPCN ONNX (download.ps1)  
- [ ] Depth-to-space + conv runner **or** ort session  
- [ ] Luma SR + chroma upsample  
- [ ] CPU FPS smoke on 480p  

### Wave SR3 — ESRGAN-edge (2 sessions)

- [ ] Compact Real-ESRGAN ONNX in vendor  
- [ ] Tiled ort inference + blend  
- [ ] Generative mode flag + report  
- [ ] OOM budget tests  

### Wave SR4 — SwinIR heavy (2 sessions)

- [ ] SwinIR ONNX + ort (document opset)  
- [ ] Large still path only; refuse if scale×area too big without tiles  
- [ ] Quality note vs ESRGAN oil-paint  

### Wave SR5 — GPU (optional, Forge)

- [ ] WGSL bicubic / sub-pixel shuffle certified against CPU oracle  
- [ ] Adapter-keyed cache like WGSL Forge  

### Wave SR6 — Surfaces

- [ ] Studio / desktop “Enhance”  
- [ ] Library MANIFEST shelf  
- [ ] Recipe: `enhance_frame` / `enhance_still`  

**Spawn order:** SR0 ∥ SR1 → SR2 → SR3 ∥ SR4 → SR5 → SR6  

**Copy-paste track prompt:**

```text
Track SR[n] on Qualia 0.0.25. Canonical C:\Projects\qualia-27062026 only.
Read docs/plans/native-super-resolution-excellence-2026.md
Native Rust; OpenCV/NCNN are weight/algorithm references only.
PermissiveReady tags; tiling for learned backends; no Python.
Single-function files under cv/sr/ and sr/.
cargo test -p qualia-vision --lib sr
```

---

## 10. Success criteria (2026 excellence)

| Criterion | Metric |
|-----------|--------|
| Classical beats nearest | PSNR↑ on synthetic down/up |
| Light tier usable live | Documented FPS on reference laptop CPU for 480p→2× |
| Edge GAN/CNN safe | 4K still tiles without OOM under default budget |
| Licence clean defaults | No NC/GPL as default backend |
| API unified | One `super_resolve` / report for all tiers |
| Honesty | Generative vs sharpen modes enforced in medical recipe |

---

## 11. Explicit non-goals (this programme)

- Shipping OpenCV or NCNN as a **required** native dependency  
- Claiming “lossless recovery” of detail  
- DIV2K-NC weights as installer defaults  
- Full SwinIR real-time 4K video without dedicated GPU cluster  
- Replacing pad/rPPG evidence with SR “clarity”

---

## 12. Principal decisions (optional, non-blocking)

| ID | Question | Default if silent |
|----|----------|-------------------|
| SR-P1 | Default product tier? | Classical + CNN-Light; ESRGAN opt-in |
| SR-P2 | Medical default generative? | **Off** |
| SR-P3 | Bundle which weights in installer? | FSRCNN/ESPCN only until size review |
| SR-P4 | Enable `ort` in default desktop feature? | Optional feature until size/CI OK |

Training custom SR models remains **TrainingDeferred** — does not block SR0–SR2.

---

## 13. Next action

Say **`execute SR0 SR1`** to land classical + tiling immediately, then **`execute SR2`** for FSRCNN/ESPCN ONNX path.

*End of plan. Native, permissive, tiled, honest — 2026 excellence without selling the core to OpenCV or NCNN.*
