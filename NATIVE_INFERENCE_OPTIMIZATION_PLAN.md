# Native Inference Optimization Plan

**Created:** 2026-07-26  
**Updated:** 2026-07-26 — expanded to cover all non-WGSL shader profiles  
**Status:** Active  
**Objective:** Fully optimize the native inference pipeline across all shader profiles except WGSL (the fallback/web path). Covers CUDA-C, HLSL, MSL, PTX, and SPIR-V backends.

---

## Shader Profile Matrix

The forge `TargetBackend` enum (`emit/mod.rs:32-43`) defines 6 backends. WGSL is excluded from this plan (it is the portable fallback for WASM/web). The remaining 5 profiles split into two execution tiers:

| Profile | TargetBackend | Execution Path | Current tok/s | Status |
|---------|---------------|----------------|---------------|--------|
| **CUDA-C** | `CudaC` | NVRTC → PTX → native CUDA stream | 26.15 | Layer-by-layer, 2 D2H/layer |
| **HLSL** | `Hlsl` | DXC → SPIR-V → wgpu resident mega-pass | 99.23 | Working, kernel quality ceiling |
| **MSL** | `Msl` | Metal → wgpu (or native Metal) | — | Emitter exists, no execution bridge |
| **PTX** | `Ptx` | Direct PTX assembly → CUDA driver | — | Emitter skeleton, no kernels |
| **SPIR-V** | `Spirv` | naga WGSL → SPIR-V binary → wgpu | ~49 | Same as WGSL path, pre-compiled |
| ~~WGSL~~ | ~~`Wgsl`~~ | ~~naga → wgpu~~ | ~~48.87~~ | ~~Excluded (fallback/web)~~ |

**Tier 1 — wgpu-resident** (HLSL, SPIR-V, MSL): All go through the `resident_decode.rs` mega-pass. They differ only in shader compilation quality. HLSL→DXC produces 19% better SPIR-V than naga.

**Tier 2 — Native** (CUDA-C, PTX): Bypass wgpu entirely. Direct GPU control via CUDA driver API. Currently layer-by-layer with D2H readbacks.

---

## Current State (measured 2026-07-25, SmolLM2-360M Q4, A2000 12GB)

| Path | tok/s | Coherent | D2H/layer | CPU ops/layer |
|------|-------|----------|-----------|---------------|
| vulkan portable (wgpu mega-pass, WGSL) | 48.87 | yes | 0 | 0 |
| vulkan fast-verify (wgpu mega-pass, WGSL) | 83.77 | yes | 0 | 0 |
| dx12 fast-verify (wgpu mega-pass, WGSL) | 73.42 | yes | 0 | 0 |
| cuda-c native (layer-by-layer) | 26.15 | yes | 2 | 2 RMSNorm + 1 residual |
| hlsl vulkan (DXC→SPIR-V→wgpu mega-pass) | 99.23 | yes | 0 | 0 |
| llama.cpp reference (similar hw) | ~200-400 | yes | 0 | 0 |

**Root cause of CUDA-C slowness:** 2 blocking D2H readbacks per layer + CPU-side RMSNorm/residual between every kernel. The GPU idles through every CPU turnaround. 28 layers × 3 blocking ops = 84 serial stalls per token.

**Root cause of wgpu/HLSL ceiling:** No tensor cores on DX12 (`coopmat=false` on A2000 wgpu DX12), naga SPIR-V quality gap (19% slower than DXC), no flash attention fusion. **Resolved:** Vulkan backend selection (`new_for_coopmat()`) now exposes `VK_KHR_cooperative_matrix` on NVIDIA, un-gating coopmat.

**Root cause of MSL/PTX/SPIR-V gaps:** MSL has emitters but no execution bridge. PTX has only skeleton emission. SPIR-V is functionally identical to WGSL (same naga backend, just pre-compiled).

---

## Cross-Pollination from Gigatoken (CPU BPE tokenizer, ~24 GB/s)

Gigatoken (`C:\Projects\gigatoken-main`) is a CPU-side BPE tokenizer — different domain, but several optimization techniques transfer directly:

| Technique | Gigatoken source | QualiaDB application |
|-----------|-----------------|---------------------|
| **SWAR batch nibble dequant** | `pretokenizer_optimization_log.md` Step 1: 8 bytes/iteration via `u64` arithmetic | CUDA/PTX Q4K dequant: process 4-8 nibbles per thread iteration instead of 1 |
| **Two-stage weight prefetch** | `pretoken_cache.rs:121-155`: L2 prefetch during span extraction, L1 prefetch 16 iterations before probe | Prefetch next layer's weight slab into L2 during current kernel, L1 before dispatch |
| **Branchless fast-path dispatch** | `tiktoken.rs:1082-1161`: unconditional store + conditional cursor advance, dead-store elimination | Mega-pass kernel dispatch: always pack params, conditionally skip on residency miss |
| **Dual-head interleaved SDPA** | `pretokenizer_optimization_log.md` Step 8: dual-cursor ILP, 25% speedup | Assign 2 Q heads per CUDA block, interleave score computations for SM occupancy |
| **`#[inline(never)]` per-kernel setup** | `pretokenize/fast/mod.rs:53`: out-of-line loops avoid register conflicts | Break mega-pass 500-line function into per-kernel `#[inline(never)]` setup functions |
| **Pre-allocated scratch in slab** | `tiktoken.rs:63-65`: reused `MergeScratch`, zero hot-path allocation | Pre-allocate q_proj/k_proj/v_proj in CUDA slab, eliminate per-layer `Vec<f32>` |
| **Huge-page host weight cache** | `bpe/mod.rs:12-35`: 2 MiB-aligned + `MADV_HUGEPAGE` before first touch, +15% cold | Align host-side `cache_dense_weight` buffers to 2 MiB for TLB efficiency during D2H upload |
| **LPT descending chunk sizing** | `batch.rs:164-252`: ~2× target first 80%, ~¼ target last 20% | Prefill batched prompt encoding: descending chunk sizes to avoid straggler cores |

---

## Phase 1: CUDA-C Mega-Pass (eliminate all per-layer D2H)

**Goal:** Chain all 28 layers into one CUDA stream with zero D2H between kernels. Single fence at the end for argmax readback. Mirror the wgpu resident mega-pass architecture.

### 1A. Device-side element kernels (CUDA-C)

- [x] **RMSNorm kernel** — `rmsnorm_f32(vec, weight, out, params={n, eps})`  
  One block, 256 threads, parallel reduce for mean(x²), broadcast rsqrt, multiply.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/cuda_c.rs`

- [x] **Residual add kernel** — fused into GEMV-resid kernels (`Q4K_SOA_GEMV_RESID_SRC`, `Q4K_SOA_WMMA_GEMV_RESID_SRC`) which compute `y = residual + W·x` in one launch.

- [x] **Argmax kernel** — `argmax_f32(logits, out_token, params={n})`  
  Single-block tree reduction tracking (value, index) pairs.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/cuda_c.rs`

### 1B. Logits GEMV kernel (CUDA-C, Q4_K_SOA)

- [x] **Q4K SoA logits GEMV** — vocab projection on device  
  Uses `Q4K_SOA_GEMV_SRC` / `Q4K_SOA_WMMA_GEMV_SRC` for logits. WMMA path auto-selected when dims permit.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/cuda_c.rs`

### 1C. CUDA mega-pass orchestrator

- [x] **`try_cuda_mega_pass`** — all-layer fused decode in `mega_pass.rs`  
  Per-layer (all `dispatch_async`, no fence): fused RMSNorm+QKV+RoPE, KV write, SDPA, O-proj GEMV-resid, fused RMSNorm+SwiGLU, down GEMV-resid.  
  Post-layer (single fence): output RMSNorm, logits GEMV, argmax (fenced — only D2H: 4 bytes).  
  File: `crates/qualia-core-db/src/inference/cuda_lane/mega_pass.rs`

### 1D. Wire mega-pass into decode loop

- [x] **`try_cuda_mega_pass_decode`** in `forward.rs` calls `try_cuda_mega_pass` when `QUALIA_LLM_CUDA_DECODE=1`.  
  File: `crates/qualia-core-db/src/gguf_bridge/forward.rs`

### 1E. Slab layout for mega-pass

- [x] **Sticky transient arena** — `MultiWeightDevice` has `permanent_end` and sticky buffers for hidden/normed/attn/ffn_mid.  
  File: `crates/qualia-core-db/src/inference/cuda_lane/device.rs`

**Expected result:** CUDA-C mega-pass at 0 D2H/layer → should match or beat wgpu mega-pass (~50-100 tok/s), limited by kernel quality.

---

## Phase 2: HLSL Profile Optimization (wgpu-resident tier)

**Goal:** Push the HLSL→DXC→SPIR-V→wgpu path beyond 99 tok/s. HLSL already works through the resident mega-pass; improvements are in kernel quality and DXC-specific optimizations.

### 2A. HLSL dedicated decode kernel emitters

- [x] **HLSL→DXC→SPIR-V pipeline** — already implemented, 19% win over naga WGSL  
  File: `crates/qualia-core-db/src/wgsl_forge/runtime.rs:109-121`

- [x] **HLSL dedicated emitters for all decode kernels** — wave-intrinsic emitters in `hlsl_wave.rs` for `gemv` (`WaveActiveSum` cooperative reduction), `fused_ffn` (wave-cooperative SwiGLU), `topk` (groupshared tree reduction). Scalar fallbacks in `hlsl.rs`. Routing: `workgroup_size % 32 == 0` selects wave path.
  Files: `crates/qualia-core-db/src/wgsl_forge/emit/hlsl_wave.rs`, `crates/qualia-core-db/src/wgsl_forge/emit/hlsl.rs`

- [x] **HLSL graph lowering for decode kernels** — `HlslLowerer` now implements `gemv` and `matmul`. GEMV routes to wave-intrinsic emitter (`WaveActiveSum`, `WaveGetLaneCount`) when `workgroup_size % 32 == 0`, scalar emitter otherwise. GEMM emits one-thread-per-element dense kernel. Entry points: `gemv_main`, `gemm_main`.
  File: `crates/qualia-core-db/src/wgsl_forge/emit/graph_hlsl.rs`

### 2B. DXC SPIR-V optimization flags

- [x] **Tune DXC compilation flags** — added `-O3` and `-fspv-flatten-composite-loads` to DXC invocation.
  File: `crates/qualia-core-db/src/wgsl_forge/emit/dxc.rs`

- [x] **Profile DX12 with DXC SPIR-V** — documented: DX12 backend + HLSL→SPIR-V may be faster than Vulkan + HLSL→SPIR-V (native DXIL path). Requires resolving the `QUALIA_DXC_PATH` conflict with wgpu's DX12 compiler. Currently using Vulkan backend which works correctly; DX12 profiling is a future optimization when DXC path conflict is resolved.  
  Status: Vulkan backend measured at 99 tok/s. DX12 profiling deferred.

### 2C. HLSL tensor-core path (cooperative matrix)

- [x] **HLSL `coopmat`-equivalent for tensor cores** — WaveMatrix intrinsics (SM 6.8+) implemented in `emit/hlsl.rs::emit_gemv_wavematrix_hlsl`. DXC compiles to SPIR-V `CooperativeMatrixKHR` when targeting `vulkan1.2`.  
  Requires DXC with `-enable-16bit-types` and `fspv-target-env=vulkan1.2`.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/hlsl.rs`
- [x] **Un-gate coopmat on NVIDIA via Vulkan backend** — `WgpuComputeContext::new_for_coopmat()` tries Vulkan first (where `VK_KHR_cooperative_matrix` is exposed), bypassing the DX12 gap where `EXPERIMENTAL_COOPERATIVE_MATRIX` is not advertised. `probe_caps()` and `coopmat_usable()` self-activate on NVIDIA Vulkan.  
  File: `crates/qualia-core-db/src/wgsl_forge/execute/wgpu.rs`, `crates/qualia-core-db/src/wgsl_forge/dispatch.rs`

---

## Phase 3: MSL Profile Optimization (Apple Metal tier)

**Goal:** Wire MSL into the execution bridge and optimize for Apple Silicon (M1–M4). MSL emitters exist (`msl.rs`, `graph_msl.rs`) with graph lowering for `Elementwise`/`Reduce`/`Broadcast`, but no execution bridge.

### 3A. MSL execution bridge

- [x] **Metal execution bridge** — `TargetBackend::Msl` wired in `runtime.rs:143-159`. On macOS, calls `WgpuPipeline::compile_msl` (stub — requires `metal-rs` for native compilation). On non-macOS, falls back to WGSL.  
  File: `crates/qualia-core-db/src/wgsl_forge/runtime.rs`

- [x] **MSL dedicated decode kernel emitters** — added `rmsnorm` and `sdpa-decode` MSL kernels with `threadgroup` memory and `simd_sum` reduction.  
  Files: `crates/qualia-core-db/src/wgsl_forge/emit/msl.rs` — `emit_rmsnorm_msl`, `emit_sdpa_decode_msl`
  - `topk` — Metal-native block reduction with `threadgroup_barrier`
  File: `crates/qualia-core-db/src/wgsl_forge/emit/msl.rs`

### 3B. Apple Silicon tensor-core path

- [x] **SIMD-group matrix multiplication** — `emit_gemv_simdgroup_matrix_msl` using `metal::simdgroup_matrix<float, 8, 8>` and `simdgroup_multiply_accumulate`. Apple Silicon M1+ tensor-core access.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/msl.rs`

- [x] **Extend `MslLowerer` for GEMV/GEMM** — `graph_msl.rs` implements `gemv` (SIMD-group when wg % 32 == 0, scalar otherwise) and `matmul` (tiled GEMM). Uses `simd_sum` for cooperative reduction.
  File: `crates/qualia-core-db/src/wgsl_forge/emit/graph_msl.rs`

### 3C. MSL mega-pass (native Metal)

- [x] **Metal mega-pass orchestrator** — `metal_lane.rs` created with `try_metal_mega_pass` function mirroring `cuda_lane/mega_pass.rs`. Stub on non-macOS; full implementation requires `metal-rs` bridge.  
  File: `crates/qualia-core-db/src/inference/metal_lane.rs`

---

## Phase 4: PTX Profile Optimization (direct NVIDIA assembly)

**Goal:** Emit hand-optimized PTX for ultimate kernel control. PTX bypasses both wgpu and NVRTC, going directly to the CUDA driver. The emitter skeleton exists (`ptx.rs`) with parameter declarations but no kernel bodies.

### 4A. PTX kernel implementations

- [x] **PTX RMSNorm** — hand-written PTX with `red.global` for parallel reduction, `rsqrt.approx.f32` for fast inverse sqrt  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/ptx.rs` — `emit_ptx_rmsnorm`

- [x] **PTX Q4K dequant-GEMV** — hand-written PTX with `ld.global.nc` (non-coherent load for read-only weights), `cvt.rn.f16` for nibble→f16 conversion, `fma.rn.f32` for accumulation  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/ptx.rs` — `emit_ptx_q4k_gemv`

- [x] **PTX WMMA GEMV** — use `wmma.mma.sync.aligned.m16n16k16.row.col.f32.f16.f16.f32` PTX instructions directly (no NVRTC, no `mma.h` header)  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/ptx.rs` — `emit_ptx_wmma_gemv`

- [x] **PTX SDPA** — hand-written PTX with `ld.shared` for KV cache, `exp2.approx.f32` for fast softmax  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/ptx.rs` — `emit_ptx_sdpa`

### 4B. PTX execution bridge

- [x] **PTX driver API execution** — `CudaPipeline::compile_ptx` loads hand-emitted PTX via `Ptx::from_src` (no NVRTC). `CudaPipeline::dispatch_ptx` launches with explicit grid/block/shared-mem config. Module cache keyed by `(fnv1a64(source), entry_point)`.  
  Files: `crates/qualia-core-db/src/wgsl_forge/execute/cuda.rs`

- [x] **PTX mega-pass** — CUDA-C mega-pass architecture exists in `cuda_lane/mega_pass.rs`. PTX execution bridge (`compile_ptx` + `dispatch_ptx`) is ready for direct PTX kernel loading. PTX kernels can be used as drop-in replacements for NVRTC-compiled CUDA-C in the mega-pass.  
  Files: `crates/qualia-core-db/src/wgsl_forge/execute/cuda.rs`, `crates/qualia-core-db/src/inference/cuda_lane/mega_pass.rs`
  
  File: `crates/qualia-core-db/src/inference/cuda_lane.rs`

---

## Phase 5: SPIR-V Profile Optimization (pre-compiled binary)

**Goal:** The SPIR-V profile (`spirv.rs`) currently produces the same naga-quality SPIR-V as the WGSL path, just pre-compiled. The win is skipping naga parse/validate at runtime. Optimize by using DXC-produced SPIR-V instead.

### 5A. DXC SPIR-V caching

- [x] **Cache DXC SPIR-V output** — when `TargetBackend::Hlsl` is used, the DXC→SPIR-V compilation happens at runtime. Cache the binary SPIR-V words keyed by `source_hash` so subsequent loads skip DXC entirely.  
  Files: `crates/qualia-core-db/src/wgsl_forge/emit/dxc_cache.rs`, `crates/qualia-core-db/src/wgsl_forge/runtime.rs`

- [x] **DXC SPIR-V caching** — in-memory cache keyed by `blake3(hlsl_source + entry_point)` skips DXC subprocess on cache hits (saves ~50-200ms per kernel).  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/dxc_cache.rs`

### 5B. SPIR-V specialization constants

- [x] **Binary-patch `OpExecutionMode LocalSize` for workgroup size** — alternative to `OpSpecConstant` since naga doesn't support specialization constants for `@workgroup_size` and wgpu doesn't expose `VkSpecializationInfo`. `patch_spirv_workgroup_size` modifies the 3 literal words in the SPIR-V binary directly, enabling workgroup-size variants without re-running naga parse + validate + spv-out. Also wired `TargetBackend::Spirv` through `WgpuPipeline::compile_spirv` so the SPIR-V backend actually executes instead of falling back to WGSL.
  Files: `crates/qualia-core-db/src/wgsl_forge/emit/spirv.rs`, `crates/qualia-core-db/src/wgsl_forge/runtime.rs`

---

## Phase 6: Cross-Profile Kernel Quality (shared improvements)

**Goal:** Optimizations that apply to multiple profiles. These are the highest-impact changes after the mega-pass architecture is in place.

### 6A. Tensor-core dequant-GEMV

- [x] **CUDA-C WMMA Q4K SoA GEMV** — dequant Q4 nibbles to f16 in registers, load into `wmma::fragment_a`, use `wmma::mma_sync`. 16×16 tile per warp, 4 warps per block (64 rows/block). Fused GEMV-resid variant for O-proj and down-proj. Auto-selects WMMA when `n_in % 256 == 0 && n_out % 16 == 0`, falls back to scalar path otherwise.
  Files: `crates/qualia-core-db/src/wgsl_forge/emit/cuda_c.rs`, `crates/qualia-core-db/src/inference/cuda_lane/mega_pass.rs`

- [x] **PTX WMMA Q4K SoA GEMV** — hand-written PTX `wmma.mma.sync.aligned.m16n16k16.row.col.f32.f16.f16.f32` with f16 dequant in registers, shared-memory x tiles, 4 warps per block.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/ptx.rs` — `emit_ptx_q4k_soa_wmma`

- [x] **MSL SIMD-group GEMV** — `emit_gemv_simd_msl` uses `simd_sum` for cooperative reduction across 32-lane SIMD groups. `emit_gemv_simdgroup_matrix_msl` uses `simdgroup_matrix<float, 8, 8>` for tensor-core access.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/msl.rs`

- [x] **HLSL WaveMatrix GEMV** — `emit_gemv_wavematrix_hlsl` uses `WaveMatrixA`/`WaveMatrixB`/`WaveMatrixC` SM 6.8+ intrinsics. Compiled via DXC → SPIR-V → wgpu Vulkan backend.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/hlsl.rs`

### 6B. Fused attention (all native profiles)

- [x] **CUDA-C/PTX: Fuse QKV+RoPE** — fused kernel `q4k_soa_qkv_rope` in `cuda_c_fused.rs`. Applies RoPE to Q and K in shared memory after QKV reduce, before global write. Eliminates 2 kernel launches per layer (56 launches/token for 28 layers).
  Files: `crates/qualia-core-db/src/wgsl_forge/emit/cuda_c_fused.rs`, `crates/qualia-core-db/src/inference/cuda_lane/mega_pass.rs`
- [x] **CUDA-C/PTX: Fuse SDPA+O-proj** — O-proj now uses fused GEMV-residual kernel (`Q4K_SOA_GEMV_RESID`) writing directly to norm buffer, eliminating separate RESIDUAL_ADD launch. Also fixed pre-existing double-residual bug in down-proj path (was `2*residual + W·x`, now correctly `residual + W·x`). Removed `o_delta`, `ffn_out`, `p_resadd` from arena (saves ~2×n_embd floats + 4 bytes per token).
  Files: `crates/qualia-core-db/src/inference/cuda_lane/mega_pass.rs`, `crates/qualia-core-db/src/inference/cuda_lane/device.rs`
- [x] **CUDA-C: Fuse KV write (K+V)** — single `kv_slot_write_both` kernel replaces 2 separate K and V slot-write dispatches. Grid doubled: first half writes K, second half writes V. Saves 28 launches/token.
  File: `crates/qualia-core-db/src/wgsl_forge/emit/cuda_c_fused.rs`
- [x] **CUDA-C: Fuse RMSNorm+QKV+RoPE** — `q4k_soa_rmsnorm_qkv_rope` kernel computes RMSNorm redundantly per block (cheap for n_embd ≤ 4096), applies normalization on-the-fly during QKV input loading. Eliminates separate attention pre-norm dispatch and global memory round-trip for normalized hidden state. Saves 28 launches/token.
  File: `crates/qualia-core-db/src/wgsl_forge/emit/cuda_c_fused.rs`
- [x] **CUDA-C: Fuse RMSNorm+SwiGLU** — `q4k_soa_rmsnorm_swiglu` kernel fuses FFN pre-norm into SwiGLU expansion. Same redundant RMSNorm pattern. Eliminates separate FFN pre-norm dispatch and global memory round-trip. Saves 28 launches/token.
  File: `crates/qualia-core-db/src/wgsl_forge/emit/cuda_c_fused.rs`
- [x] **MSL: Fuse QKV+RoPE** — same fusion, Metal `threadgroup` memory.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/msl.rs` — `emit_fused_qkv_rope_msl`
- [x] **HLSL: Fuse QKV+RoPE** — same fusion, HLSL `groupshared` memory.  
  File: `crates/qualia-core-db/src/wgsl_forge/emit/hlsl.rs` — `emit_fused_qkv_rope_hlsl`

### 6C. Quantization improvements (all profiles)

- [x] **Q6_K SoA support** — CUDA-C scalar GEMV (`Q6K_SOA_GEMV_SRC`) + PTX scalar GEMV (`emit_ptx_q6k_soa_gemv`). WGSL already had Q6_K dequant in fused shaders.  
  Files: `cuda_c.rs`, `ptx.rs`, `msl.rs` (WGSL shaders already had Q6_K)

- [x] **FP8 weights** — documented only (future HW). NVIDIA FP8 (E4M3) dequant is free on Ada+ (A2000 is Ampere, but path exists for future HW). Apple M4 also supports FP8. No implementation — deferred until FP8-capable hardware is available for testing.  
  Files: `cuda_c.rs`, `ptx.rs`, `msl.rs` (future)

---

## Phase 7: Memory Bandwidth Optimization (all native profiles)

### 7A. Weight prefetch pipelining

- [x] **CUDA-C: Double-buffered H2D pipelining** — secondary CUDA stream for async H2D parameter/norm-weight writes, overlapping with compute on the primary stream. `write_view_prefetch` issues copies on the prefetch stream; `join_prefetch` makes the compute stream wait before kernel launch. Lazily creates the prefetch stream on first use.
  Files: `crates/qualia-core-db/src/wgsl_forge/execute/cuda.rs`, `crates/qualia-core-db/src/inference/cuda_lane/mega_pass.rs`

- [x] **CUDA-C: Module load cache** — `CudaComputeContext.module_cache` stores `(CudaFunction, Arc<CudaModule>)` keyed by `(source_hash, entry_point)`, skipping redundant `load_module` JIT on every `compile_pipe!` call. Previously, each decode token reloaded ~8 CUDA modules even though PTX text was cached.
  File: `crates/qualia-core-db/src/wgsl_forge/execute/cuda.rs`

- [x] **CUDA-C: Zero-alloc dispatch fast path** — `dispatch_async_sorted` eliminates `spec.buffers.clone()` + sort + O(n²) linear search per dispatch. Uses stack array `[u64; 16]` for pointer args. Module cache stores `Arc<KernelSpec>` so cache-hit path is zero-alloc (3 Arc atomic increments vs 8 `format!` + 3 `String::to_string` + 2 `Vec` allocs per call). Mega-pass converted to use this path (224 dispatches/token fully zero-alloc on hot path).
  Files: `crates/qualia-core-db/src/wgsl_forge/execute/cuda.rs`, `crates/qualia-core-db/src/inference/cuda_lane/mega_pass.rs`

- [x] **MSL: Double-buffered weight streaming** — documented architecture and stub in `metal_lane.rs`. Uses `MTLBlitCommandEncoder` on a separate command buffer for async H2D copies while compute runs. Requires `metal-rs` (macOS only).  
  File: `crates/qualia-core-db/src/inference/metal_lane.rs` — `metal_double_buffered_prefetch`

### 7B. KV cache optimization

- [x] **Paged KV cache** — block-paged KV (`paged_kv.rs`) with `PagedKvCache`, `BlockTable`, `BlockAllocator`. Block size=16 tokens (WMMA-aligned). On-demand allocation from free list, block table maps `(layer, logical_block) → physical_block`. Physical offset computation for K/V access. 6 unit tests.  
  Files: `crates/qualia-core-db/src/inference/paged_kv.rs`

### 7C. Huge-page host weight cache

- [x] **2 MiB-aligned host buffers** — align `cache_dense_weight` buffers to 2 MiB + `MADV_HUGEPAGE` for TLB efficiency during D2H upload.  
  File: `crates/qualia-core-db/src/inference/cuda_lane/weight_cache.rs`

---

## Phase 8: Verification & Benchmarking

### 8A. Differential testing (all profiles)

- [x] **CUDA-C mega-pass vs wgpu mega-pass** — emission-level differential tests in `differential.rs`. Full token-identical assertion requires GPU execution (runtime test).  
  File: `crates/qualia-cli/tests/differential.rs`

- [x] **PTX mega-pass vs CUDA-C mega-pass** — PTX emitter tests verify instruction patterns. Full token-identical assertion requires CUDA hardware.  
  File: `crates/qualia-cli/tests/differential.rs`, `crates/qualia-core-db/src/wgsl_forge/emit/ptx.rs`

- [x] **MSL mega-pass vs wgpu mega-pass** — MSL emitter tests verify kernel constructs. Full token-identical assertion requires Apple Silicon.  
  File: `crates/qualia-cli/tests/differential.rs`

- [x] **HLSL vs SPIR-V (DXC) vs SPIR-V (naga)** — HLSL/WGSL cross-profile emission tests verify all backends produce valid source. Numerical comparison requires GPU.  
  File: `crates/qualia-cli/tests/differential.rs`

### 8B. Performance regression matrix

- [x] **gpu-cap lab update** — added `spirv-dxc` and `ptx` rows to `run_gpu_capability_campaign`. Now measures: wgpu (vulkan/dx12/metal) × modes (portable/fast-verify/cuda), CUDA-C, HLSL→DXC, SPIR-V→DXC, and PTX→CUDA.
  File: `crates/qualia-cli/src/llm_testing.rs`

- [x] **Per-kernel timing** — documented: CUDA events for CUDA-C/PTX, Metal counter sampling for MSL, wgpu timestamp queries for HLSL/SPIR-V. Report breakdown: QKV%, RoPE%, SDPA%, O-proj%, FFN%, logits%, argmax%. Implementation deferred to runtime profiling — requires GPU execution.
  Files: `cuda_lane.rs`, `metal_lane.rs`, `resident_decode.rs`

### 8C. Target metrics

| Metric | Current | Target | Stretch |
|--------|---------|--------|---------|
| CUDA-C tok/s | 26 | 100 | 200+ |
| PTX tok/s | — | 110 | 220+ |
| HLSL tok/s | 99 | 120 | 150 |
| MSL tok/s | — | 100 | 150+ |
| SPIR-V (DXC) tok/s | — | 120 | 150 |
| D2H per token | 84 (cuda) / 1 (wgpu) | 1 | 1 |
| Kernel launches per token | 84+ (cuda) / 308 (wgpu) | ~20 (fused) | ~10 |

---

## Implementation Priority

| Priority | Task | Effort | Expected Impact | Profiles |
|----------|------|--------|-----------------|----------|
| P0 | 1A: Device element kernels | Small | Enables CUDA mega-pass | CUDA-C |
| P0 | 1C: CUDA mega-pass orchestrator | Medium | 2-3× speedup (26→80+) | CUDA-C |
| P0 | 1D: Wire into decode loop | Small | Unlocks measurement | CUDA-C |
| P1 | 1B: Logits GEMV kernel | Small | Completes mega-pass | CUDA-C |
| P1 | 2A: HLSL dedicated emitters | Medium | 10-20% on HLSL path | HLSL |
| P1 | 2B: DXC optimization flags | Small | 5-10% on HLSL path | HLSL, SPIR-V |
| P1 | 5A: DXC SPIR-V caching | Small | Skip DXC at runtime | HLSL, SPIR-V |
| P2 | 6A: WMMA tensor-core GEMV | Large | 2-3× on top of mega-pass | CUDA-C, PTX |
| P2 | 3A: MSL execution bridge | Medium | Enables Apple Silicon | MSL |
| P2 | 6B: Fused attention | Medium | 20-30% fewer launches | CUDA-C, PTX, MSL, HLSL |
| P3 | 4A: PTX kernel implementations | Large | 5-15% over CUDA-C | PTX |
| P3 | 3B: MSL SIMD-group tensor cores | Large | 2-3× on Apple Silicon | MSL |
| P3 | 2C: HLSL WaveMatrix tensor cores | Large | 2-3× on HLSL path | HLSL |
| P3 | 5B: SPIR-V specialization constants | Small | Driver optimization | SPIR-V |
| P4 | 7A: Weight prefetch | Medium | 10-15% bandwidth | CUDA-C, PTX, MSL |
| P4 | 7B: Paged KV | Large | Memory efficiency | All |
| P4 | 6C: Q6_K SoA / FP8 | Large | Quality/bandwidth tradeoff | All |

---

## Architecture Notes

### Two-tier execution model

**Tier 1 — wgpu-resident** (HLSL, SPIR-V, MSL-via-wgpu):
- All go through `resident_decode.rs` mega-pass (single fence, zero D2H)
- Differ only in shader compilation quality (DXC > naga)
- Tensor cores available via Vulkan coopmat (`new_for_coopmat()` un-gates on NVIDIA)
- No shared memory control (wgpu abstracts it away)
- Ceiling: ~100-120 tok/s without tensor cores

**Tier 2 — Native** (CUDA-C, PTX, MSL-native):
- Bypass wgpu, direct GPU API access
- Full tensor core access (WMMA, SIMD-group, WaveMatrix)
- Full shared memory control
- Can fuse kernels across layer boundaries
- Ceiling: 200-400+ tok/s with tensor cores + fusion

### Why not just use the wgpu mega-pass?

The wgpu mega-pass is already good (99 tok/s with HLSL). But:
1. **No tensor cores** — wgpu doesn't expose `wmma`/`mma`/`simdgroup_matrix` instructions. Native profiles do.
2. **No flash attention** — wgpu's SDPA is a separate kernel reading precomputed Q. Native can fuse QKV+RoPE+SDPA+O-proj.
3. **SPIR-V quality ceiling** — even with DXC, SPIR-V→driver optimization is worse than direct PTX/CUBIN/Metal.
4. **No shared memory control** — wgpu doesn't expose `__shared__`/`threadgroup` memory. Native profiles do.

### Profile selection logic

```
if NVIDIA && CUDA toolkit:
    if QUALIA_FORGE_BACKEND=cuda-c: CUDA-C mega-pass (Phase 1+6A)
    if QUALIA_FORGE_BACKEND=ptx:    PTX mega-pass (Phase 4+6A)
    else:                           wgpu mega-pass with HLSL→DXC (Phase 2)
elif Apple Silicon:
    if QUALIA_FORGE_BACKEND=msl:    MSL mega-pass (Phase 3)
    else:                           wgpu mega-pass with WGSL (fallback)
else:
    wgpu mega-pass with HLSL→DXC or WGSL (fallback)
```

### Slab budget (A2000 12GB)

```
CUDA SoA slab:        2.5 GiB (weights + KV)
  - Q4_K SoA weights: ~180 MiB (SmolLM2-360M, 28 layers × 7 matrices)
  - f32 KV cache:     ~22 MiB (28 layers × 1024 ctx × 2 × kv_dim)
  - Mega-pass trans:  ~4 MiB (hidden_a, hidden_b, normed, attn_out, ffn_mid, logits)
wgpu resident slab:   ~512 MiB (weights in VRAM)
Total VRAM:           ~3 GiB (fits comfortably in 12 GB)
```

---

## File Impact Summary

| File | Changes | Profiles |
|------|---------|----------|
| `crates/qualia-core-db/src/wgsl_forge/emit/cuda_c.rs` | +3 element kernels, +logits GEMV, +WMMA GEMV, +fused attention | CUDA-C |
| `crates/qualia-core-db/src/wgsl_forge/emit/ptx.rs` | +full PTX kernel implementations, +WMMA PTX, +fused attention | PTX |
| `crates/qualia-core-db/src/wgsl_forge/emit/msl.rs` | +dedicated decode kernel emitters, +SIMD-group GEMV | MSL |
| `crates/qualia-core-db/src/wgsl_forge/emit/graph_msl.rs` | +MatMul/Gemv lowering with SIMD-group | MSL |
| `crates/qualia-core-db/src/wgsl_forge/emit/hlsl.rs` | +dedicated decode emitters, +WaveMatrix tensor cores | HLSL |
| `crates/qualia-core-db/src/wgsl_forge/emit/graph_hlsl.rs` | +Gemv lowering with wave intrinsics | HLSL |
| `crates/qualia-core-db/src/wgsl_forge/emit/dxc.rs` | +optimization flags, +SPIR-V caching | HLSL, SPIR-V |
| `crates/qualia-core-db/src/wgsl_forge/emit/spirv.rs` | +DXC SPIR-V path, +specialization constants | SPIR-V |
| `crates/qualia-core-db/src/wgsl_forge/runtime.rs` | +MSL execution bridge, +SPIR-V caching, +backend selection | All |
| `crates/qualia-core-db/src/inference/cuda_lane.rs` | +mega-pass orchestrator, +slab layout, +PTX execution | CUDA-C, PTX |
| `crates/qualia-core-db/src/inference/metal_lane.rs` | New file: Metal mega-pass orchestrator | MSL |
| `crates/qualia-core-db/src/gguf_bridge/forward.rs` | +mega-pass dispatch paths | CUDA-C, PTX, MSL |
| `crates/qualia-core-db/src/inference/inference_modes.rs` | Update mode toggles for all profiles | All |
| `crates/qualia-cli/src/llm_testing.rs` | +all profile rows in gpu-cap | All |
