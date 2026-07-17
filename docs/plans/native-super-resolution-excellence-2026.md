# Native Super-Resolution Excellence (2026)

**Branch:** `0.0.25`  
**Owner:** Qualia vision / `qualia-vision`  
**Ambition:** Production-grade SR **as a native Rust library** inside Qualia — not OpenCV product ABI, not Python, not NCNN as a required runtime. External projects supply **permissive weights + reference algorithms**; Qualia owns inference, tiling, GPU path, and honesty.  
**Status:** Plan ready for execute (swarm tracks SR0–SR8).  
**Related audit:** `vision-gpu-10d-geometry-gap-audit-2026.md` — confirms CV is **not yet** on `shared_gpu` / Forge; this plan’s GPU path is **target**, not as-is.

---

## 0. Design principles (immovable)

| # | Principle |
|---|-----------|
| 1 | **Native core first.** Bicubic / Lanczos / edge-directed / sub-pixel CNNs implemented in Rust under `cv/sr/` and `sr/`. Learned backends use **Qualia’s GPU backplane** (below) and/or optional `ort` — never “call OpenCV dnn_superres” or NCNN as the product. |
| 2 | **GPU is first-class, not an afterthought.** SR reuses the existing **renderer + compute backplane**: `gpu_context::shared_gpu()` (wgpu → Vulkan / DX12+DirectML / Metal / WebGPU), optional **CUDA** feature paths (same family as LLM decode), **WGSL Forge** for certified kernels, volumetric/`webizen-render` for display. No second adapter stack. |
| 3 | **Permissive licence only as defaults.** MIT / Apache-2.0 / BSD for **code and preferred weight files**. Training-data residual risk is a **DiligenceNote**, not a fake “commercial licence gate” on Apache zoo weights. |
| 4 | **Three compute tiers** (your taxonomy), one unified API, **device-selected** at runtime. |
| 5 | **Tiling is mandatory** for edge/GPU tiers — VRAM/RAM scales with **input** tile size; coordinate with `shared_gpu` VRAM accounting. |
| 6 | **No texture hallucination as truth.** GAN paths must label outputs as *generative enhancement*; medical/forensic paths prefer non-GAN or confidence maps. |
| 7 | **Caller-buffered hot paths** where possible; Tier-2 may use bounded scratch for tiles. |
| 8 | **Anti-monolith:** one primary algorithm per `.rs`; `sr/` as a library subdirectory. |

### 0.1 GPU / renderer backplane (already in-tree)

SR must **not** invent a parallel GPU story. Wire to what Qualia already runs for LLM, volumetric render, and Forge:

| Layer | Location / capability | SR use |
|-------|----------------------|--------|
| **Shared device** | `qualia_core_db::gpu_context::shared_gpu()` — one process-wide `wgpu` Device/Queue | All native GPU SR; same instance as inference + `PortalGpu` / volumetric renderer |
| **Backends via wgpu** | Vulkan, DX12 (**DirectML** on Windows), Metal, GL, WebGPU (WASM) | Portable compute shaders for classical SR + light CNN; `QUALIA_WGPU_BACKEND` env already selects |
| **CUDA** | Optional `cuda` feature on core (LLM fused decode / weight cache) | Heavy SR or custom GEMM/conv when CUDA present; **not** required for default product |
| **WGSL Forge** | Typed kernel IR, Naga validate, CPU oracle, adapter-keyed cache, CLI `shader` | Generate/certify `sr_bicubic`, `sr_depth2space`, tile blend; promote only after oracle match |
| **Renderer** | `render::gpu`, `webizen-render` volumetric / mesh | Preview enhanced frames; optional post-process pass in studio pipeline |
| **Thermal / universe** | `ThermalGovernor`, queue lanes / compute universe tags | Cap concurrent SR tiles when LLM decode is Active; fail closed or degrade to CPU classical |
| **VRAM accounting** | `gpu_context` VRAM used/total | Tile planner reads budget; refuse full-frame ESRGAN when headroom &lt; estimate |

**Anti-patterns (forbidden):**

- Spinning a **second** `wgpu::Instance` / adapter for SR alone  
- Depending on **NCNN Vulkan** when wgpu already covers Vulkan/Metal/DX12/WebGPU  
- Treating CUDA as the only “real” GPU path (Windows DirectML + Vulkan are production paths today)  
- Bypassing Sentinel / thermal budget while LLM is mid-decode  

**Preferred dispatch order (runtime):**

```text
1. Classical GPU (Forge/WGSL bicubic·Lanczos) if shared_gpu Cool and shader certified
2. Classical CPU if no GPU / thermal Critical / WASM without WebGPU
3. CNN-Light: WGSL fused path if certified, else ort on GPU EP if available, else CPU ort/native
4. ESRGAN / Swin: tiled; prefer ort CUDA/DML/TensorRT-class EP when present, else wgpu upload+custom kernels when ready
5. Always: tile + blend; report device name + backend in SrReport
```

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
    pub device: SrDevicePolicy,   // Auto | Cpu | SharedWgpu | CudaIfAvailable
}

pub enum SrDevicePolicy {
    /// Prefer shared_gpu when Cool; else CPU classical / light CNN.
    Auto,
    CpuOnly,
    /// Explicit: `gpu_context::shared_gpu()` (Vulkan/DX12-DML/Metal/WebGPU).
    SharedWgpu,
    /// Optional core `cuda` feature + EP; fall back SharedWgpu then CPU.
    CudaPreferred,
}

pub fn super_resolve(req: &SrRequest, out: &mut [u8]) -> Result<SrReport, SrError>;
```

`SrReport`: backend id, **device/backend string** (e.g. `wgpu-vulkan`, `wgpu-dx12`, `cuda`, `cpu`), scale, tile count, ms, peak scratch/VRAM estimate, `generative: bool`, weight licence tag, thermal state at start.

---

## 3. Three tiers (mapped to Qualia)

### Tier U — Ultra-Light (real-time video / CPU)

| Item | Choice |
|------|--------|
| **Algorithms (native)** | Lanczos-3, bicubic, optional **edge-directed** (NEDI-lite or similar) |
| **Learned (weights)** | **ESPCN** / **FSRCNN** (OpenCV dnn_superres lineage) — **Apache-2.0** weights as **PermissiveReady** assets |
| **Runtime** | CPU pure Rust **and** WGSL on `shared_gpu` (Vulkan/DML/Metal) for real-time; optional `ort` EP; **not** OpenCV C++ |
| **Target** | 480p→720p-class multi-FPS on laptop CPU **or** integrated GPU via wgpu |
| **Honesty** | Sharper than bicubic; **does not invent textures** |

**Vendor path:** `vendor/vision/sr/fsrcnn/` · `vendor/vision/sr/espcn/`  
**Module path:** `cv/sr/lanczos3.rs`, `cv/sr/bicubic.rs`, `sr/cnn_light/` (depth-to-space, conv)

### Tier B — Balanced (edge GPU / stills + light video)

| Item | Choice |
|------|--------|
| **Reference quality bar** | Real-ESRGAN family (MIT ecosystem), **compact** anime/video variants for edge |
| **Qualia path** | ONNX (compact Real-ESRGAN) via `ort` **with GPU EP** where available **and/or** Forge-certified WGSL tile kernels on `shared_gpu` |
| **NCNN** | **Not** a product dependency. Offline convert NCNN→ONNX if needed; **do not** ship NCNN Vulkan next to wgpu |
| **Mandatory** | Overlapping **tile + feather blend**; VRAM estimate vs `shared_gpu` accounting; fail closed if over budget |
| **GPU** | **wgpu** (Vulkan / DX12+DirectML / Metal / WebGPU) is the portable path; **CUDA** optional for host EP / custom kernels when feature on |

**Vendor path:** `vendor/vision/sr/realesrgan_compact/`  
**Module path:** `sr/tile_policy.rs`, `sr/tile_blend.rs`, `sr/esrgan_session.rs` (feature `ort`)

### Tier H — Heavy (high-fidelity stills)

| Item | Choice |
|------|--------|
| **Reference quality bar** | **SwinIR** (MIT) — transformer restoration / SR |
| **Runtime** | ONNX via `ort` (recent opset) on **CUDA / DirectML / CPU**; progressive: Forge kernels for patch ops where certified against CPU oracle |
| **Use** | Medical stills, forensic *enhancement* (watermark/provenance), print/photo, high-end studio |
| **Not for** | Real-time video unless GPU headroom + thermal Cool and tile budget allows |
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
  RGB in
    │
    ▼
  Thermal / VRAM gate ──(Critical)──► Classical CPU only
    │ Cool/Warm
    ▼
  Tile planner (size from VRAM budget + scale)
    │
    ├─► Classical: CPU  ·or·  WGSL on shared_gpu (Vulkan/DML/Metal/WebGPU)
    ├─► CNN-Light:  WGSL fused  ·or·  ort (CPU/DML/CUDA EP)
    └─► ESRGAN/Swin: tiled ort EP  ·or·  staged Forge kernels
    │
    ▼
  Seam blend (CPU or compute shader)
    │
    ▼
  RGB out + SrReport { device, backend, tiles, ms, generative }
    │
    └─► optional: webizen-render / studio preview (same shared_gpu)
```

**Concurrency with LLM / render:** SR submits on the appropriate `QueueLane` / universe when wired; if inference is `Active` and thermal ≠ Cool, Auto policy **degrades** tier (H→B→U→classical) instead of fighting for VRAM.

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

### Wave SR5 — GPU backplane (not optional polish — core path)

- [ ] Classical bicubic/Lanczos **WGSL** via Forge: Naga validate + CPU oracle + adapter-keyed cache  
- [ ] Bind to `gpu_context::shared_gpu()` only (no second device)  
- [ ] Tile blend compute shader (or CPU blend with GPU tiles)  
- [ ] DirectML / Vulkan / Metal smoke (whatever adapter is present)  
- [ ] Optional: CUDA EP path when `cuda` feature + ort CUDA provider  
- [ ] Thermal degrade policy unit tests  
- [ ] Preview hook: hand RGBA8 to volumetric/studio path without re-upload thrash  

### Wave SR6 — Surfaces

- [ ] Studio / desktop “Enhance” (device picker: Auto / GPU / CPU)  
- [ ] Library MANIFEST shelf for SR weights  
- [ ] Recipe: `enhance_frame` / `enhance_still`  

**Spawn order:** SR0 ∥ SR1 → **SR5 classical GPU early** (parallel after SR1) → SR2 → SR3 ∥ SR4 → SR6  

**Note:** SR5 is pulled **forward** relative to a “GPU last” plan: Qualia already pays for the backplane; classical GPU SR should land before heavy ONNX so video paths use Vulkan/DML/Metal immediately.

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
| Light tier usable live | Documented FPS on laptop **CPU and shared_gpu** for 480p→2× |
| GPU portable | Same classical/light path on at least one of Vulkan / DX12-DML / Metal when adapter present |
| Edge GAN/CNN safe | 4K still tiles without OOM under default **VRAM** budget from `shared_gpu` accounting |
| Licence clean defaults | No NC/GPL as default backend |
| API unified | One `super_resolve` / report for all tiers |
| Honesty | Generative vs sharpen modes enforced in medical recipe |

---

## 11. Explicit non-goals (this programme)

- Shipping OpenCV or NCNN as a **required** native dependency  
- A second GPU context parallel to `shared_gpu` / the renderer
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
| SR-P5 | Prefer CUDA EP vs wgpu for heavy SR when both present? | **Auto:** CUDA if Cool+headroom else shared_gpu |

Training custom SR models remains **TrainingDeferred** — does not block SR0–SR1–SR5 classical GPU.

---

## 13. Next action

Say **`execute SR0 SR1 SR5`** to land classical + tiling + **shared_gpu WGSL** immediately, then **`execute SR2`** for FSRCNN/ESPCN.

*End of plan. Native, permissive, tiled, honest — on Qualia’s existing Vulkan/wgpu/DirectML/CUDA/renderer backplane, without selling the core to OpenCV or NCNN.*
