# 2026-06-29 — Gap Analysis (honest session accounting)

Scope: every barrier, unimplemented path, documented limitation, bug found, "claimed-done
but wasn't", and deferred item across the 2026-06-29 session (WGSL Forge completion →
acceleration rollout → audio transforms). Written to the measurement-honesty bar: say what's
true, including what's broken or unfinished. Branch `feature/p64-manifold-wal-eigensolver`,
verified on an NVIDIA RTX A2000 (sm_86), CUDA 13.3, wgpu/naga 29.0.3.

This document is deliberately the *problems* list. A one-paragraph "what's solid" balance is at
the end (§8) so the residuals are read in proportion.

---

## 1. Upstream / platform walls (real limits, NOT fixable in our code)

These are hard constraints in wgpu/naga/WGSL/drivers/toolchains. Each was root-caused, worked
around honestly, and documented in-code.

1. **WGSL cooperative-matrix multiply is broken on wgpu 29.0.3 (no published fix).**
   `coopMultiplyAdd` returns all-zeros on the 29.0.3 Vulkan execution path (matches gfx-rs/wgpu
   #9729/#9741 — coopmat emits Device-scope SPIR-V memory ops invalid unless
   `vulkanMemoryModelDeviceScope` is auto-enabled, a fix only on unreleased git `main`). **29.0.3
   is the newest wgpu on crates.io**, so there is no released fix. `coopLoadT`/`coopStoreT`
   round-trip fine; only the multiply fails. It is NOT geometry (`Scope::Subgroup` → `@workgroup_size(32)`
   is correct; 64 gives identical zeros) and NOT just dtype (all-f32 8x8x8, the only config 29
   supports, still zeros).
   → **Resolved**: tensor-core matmul delivered via **CUDA WMMA** instead (real f16→f32, verified).
   The WGSL coopmat emitter is correct + naga-validated and will light up when wgpu ships #9741.

2. **WGSL has no `f64`** (only f32/f16/i32/u32). The f64 numeric solvers cannot use WGSL kernels.
   → **Resolved**: f64 GEMM/GEMV best-path is **native CUDA-f64** (PTX has `double`/`fma.rn.f64`),
   with the CPU floor; see also #3.

3. **df64 (emulated double) is blocked by driver float reassociation on this stack.**
   The naga→SPIR-V→NVIDIA-Vulkan path reassociates f32 arithmetic — it simplifies `c-(c-a)`→`a`
   and `fma(x,y,-(x*y))`→`0`, which collapses the df64 error-free transforms to **f32 precision
   (~2e-7 instead of ~1e-12)**. Proven: switching `two_prod` from fma to the Veltkamp/Dekker
   split gave a **byte-identical** wrong result, ruling out a missing fused-FMA and confirming
   reassociation. WGSL exposes **no portable pragma** to forbid it.
   → **Resolved honestly**: `dispatch::df64_usable()` probes the kernel at runtime; df64 runs only
   on adapters where the transforms survive, else native-CUDA/CPU. No f32 masquerades as f64.

4. **WGSL `fma()` does not reliably lower to a true single-rounding FMA** on naga 29 → SPIR-V →
   NVIDIA. Same root as #3; relevant to any future error-free-transform or compensated-summation
   kernel. Documented in `emit/df64.rs`.

5. **CUDA toolkit newer than the installed driver** (NVRTC 13.3 emits PTX ISA the 13.2 driver
   rejects: `CUDA_ERROR_UNSUPPORTED_PTX_VERSION`).
   → **Resolved**: `downgrade_ptx_isa()` rewrites the PTX `.version` to a driver-accepted ISA.

6. **NVRTC's default include search list is empty** → `#include <mma.h>` fails.
   → **Resolved**: compile with `--include-path=<CUDA_PATH>/include` + the device's real
   `--gpu-architecture` (detected compute capability).

7. **wgpu 29 ray-query acceleration-structure limits default to 0** (a config gotcha, not a true
   upstream bug): `create_blas` fails until `max_blas_geometry_count` / `max_blas_primitive_count` /
   `max_tlas_instance_count` / `max_acceleration_structures_per_shader_stage` are raised from the
   adapter's reported limits, and `EXPERIMENTAL_RAY_QUERY` is requested.
   → **Resolved** in `execute/wgpu.rs`; ray-query then executes correctly (verified).

---

## 2. Documented limitations (genuine — need hardware, data, or out-of-band input)

Honestly recorded in-code and in the plan ledgers; not yet implemented because they require
something we don't have here.

- **Topology-aware memory paths** (zero-copy persistent-mapped slabs for unified memory; pinned
  staging ring + async copy for discrete). The classification + lap-safe ring exist; the
  differentiated copy paths do not. **Unverifiable on a discrete-only A2000** — needs unified-memory
  hardware to implement+benefit-measure. (`execute/memory.rs`, `execute/wgpu.rs`, plan §2.)
- **Device-relative roofline *reject* + compute-unit-saturation pruning.** wgpu exposes no peak
  FLOPS/bandwidth/CU count, so the roofline is an *estimate that never rejects*. Would need a
  calibration micro-benchmark. (`roofline.rs`, `schedule.rs`, plan §6.)
- **Thermal pruning is NVIDIA-only** (via `nvidia-smi` in `auto-tune-all --thermal-limit`); no
  cross-vendor temp sensor.
- **Native f64/u64 IR** is reserved scaffolding, not exercised — WGSL has no f64 (see §1.2); the
  IR represents 64-bit as paired-u32 (`U64Words`). The §1 plan claim was downgraded to match.
- **Large-N FFT (N > 1024).** The FFT kernel is workgroup-local (single dispatch, N ≤ max
  workgroup). Multi-pass large-N FFT is a documented follow-up. (`emit/wgsl.rs` `emit_fft_wgsl`.)
- **Measured KEMAR HRIRs.** The HRTF path synthesizes a physically-plausible HRIR from the ITD/ILD
  model; a full measured HRIR dataset is an optional cold asset (data only Timothy can supply).
- **Tuning signature** omits a git-commit provenance field (needs build-time plumbing); explicit
  feature/limits hashing is subsumed by the adapter identity already in the cache key. (plan §8.)

---

## 3. Bugs / footguns found and fixed this session

- **`Op::MatrixMultiply` emitted a silent no-op comment** in WGSL/MSL/HLSL (would compute nothing
  for a custom KernelSpec). → now returns `Err`.
- **`validate_native` hardcoded `entry_points=["affine_f32"]`, `binding_count=3`** for every kernel.
  → now derives from the real `KernelSpec` (or empty for an opaque `--input` file).
- **The wgpu `device.poll()` results were ignored (`let _ = …`)** at all four poll sites — device
  loss was silently swallowed. → now mapped to a unified `ForgeError::DeviceLost`.
- **The ray-query lowering called `rayQueryProceed` once** (a multi-node BVH can be left partially
  traversed). → now loops to completion.
- **Dead schedule knobs** (`tile_mnk`/`use_subgroup`/`prefetch`/`unroll_factor`) advertised as
  search dimensions but never varied/consumed. → removed; `FORGE_SCHEMA_VERSION` bumped 1→2.
- **Stale doc/claims** corrected: `has_gpu_oracle` comment; the §1 "native 64-bit" claim; the
  coopmat "experimental defect" → precise root cause.

---

## 4. "Claimed done but wasn't" — caught by audit/verification, now resolved

- **The WGSL-Forge plan was ~80% implemented, not 100%**, despite the ledger/task list reading
  complete. An independent two-angle completeness audit found a real tail (§2 backend fallback,
  §2 topology memory, §4 ternary kernel, §6 dead knobs + unimplemented pruning, §7 DeviceLost +
  generic oracle, §7/§8 signature, §9 CLI flags, §1 native-64). **All implementable items were then
  built + verified**; the genuinely-platform-limited ones are in §2 above. (Recorded in plan §11.)
- **Audio "STFT/CQT/HRTF" were preview-synthesis, not transforms.** The files animated preview
  magnitude bins / log-resampled / analytic-panned — no actual DFT over audio samples. → the real
  `forward_stft` (windowed FFT), `forward_cqt` (constant-Q), and HRTF convolution are now
  implemented + correctness-gated (this is the gap you caught directly).
- **The acceleration map's audio→FFT rows were conceptual, not real op sites.** Corrected during
  verification (and the real transforms then built, above).
- **The map originally mis-stated the codebase** (claimed no `gguf_bridge/` dir; called the
  hand-written `.wgsl` shaders "targets not files"). A read-only verification sweep corrected it:
  `gguf_bridge/` is real, ~20 hand-written compute shaders exist, and 4 are orphaned (undispatched).

---

## 5. Process / tooling friction (not code defects, but real session problems)

- **Sub-agent `StructuredOutput` retry-cap failures (3×).** The WMMA-feasibility recon agent, the
  ray-query-feasibility agent, and the LLM-inference verification agent each overran the structured-
  output budget and returned nothing. Worked around (data partially recovered from previews / I
  filled the gaps directly), but it means a few findings were inferred rather than agent-reported
  (flagged where it mattered, e.g. the LLM `inference/*` per-file roles).
- **§10 lane overlap.** The solver `linear_algebra/gemm.rs` (gemm + matvec) wiring is in the
  `0.0.21-la` lane's claimed files. Proceeded under your explicit "no coordination needed, get it
  all done"; changes are strictly additive + trivially revertible; CLAIM/RELEASE were announced.
  The **LLM lane** (`gguf_bridge`/`inference`/`shaders`) is being deliberately deferred (LLM last).
- **`NOTICES.md`** was earlier reported "broken — proceed without it" but is in fact readable; this
  session's sub-agents have been appending CLAIM/RELEASE lines to the canonical
  `C:\Projects\qualiaDB\coordination\NOTICES.md`.
- **Pre-existing crate warnings (~684).** The forge work added none of note, but the crate carries
  a large pre-existing warning load (mostly dead-code in `specialized_libs`) — out of scope here,
  flagged for a future hygiene pass.

---

## 6. Deferred / not-yet-done (the remaining rollout)

Acceleration rollout status: **foundation + 3 keystone kernels + dispatcher + several consumers
done & verified**; the rest is queued.

- **LLM path migration — deferred to LAST per your instruction.** Migrating the hand-written LLM
  shaders (`gemm_substrate`/`fused_transformer`/`dequant_template`/`fused_attention`) to certified
  forge kernels + routing through the dispatcher. Needs a **softmax/attention kernel** and a
  **block-dequant kernel** (neither built yet), and care not to regress the LLM lane's resident-
  weight hot loop (the `ForgeRuntime` Vec-returning API is wrong for the hot decode path).
- **Orphan physics kernels** (`molecular_dynamics` / `kinematics` / `fluid_dynamics` / `quantum_bio`)
  — already-written shaders, not yet formalized as certified forge kernels. (Up next.)
- **More solver/ML consumers** not yet explicitly wired: k-means distances, SVM RBF kernel matrix,
  the manifold WAL eigensolver, `specialized_libs/{linear_algebra,machine_learning,statistical_computing}`.
  Several already benefit **transitively** via the shared `gemm()`/`matvec()` (PCA, SVD/eigen panels).
- **Dispatcher ops not yet added**: `dot`, `reduce/sum` (gemm + gemv are done).
- **New kernels are WGSL-only emission** (+ CUDA-f64 for gemm/gemv). The general `gemm`/`gemv`/`fft`/
  `ternary-gemv` kernels do **not** have MSL/HLSL/PTX emitter arms — they fall to those backends'
  generic path or are unsupported there. Multi-backend emission for the new kernels is a follow-up.
- **GEMV f64 has no df64 tier** (CUDA-f64 → CPU only); df64 is GEMM-only today.
- **Non-forge backends** (correctly out of forge scope, tracked separately): CPU-SIMD (incl.
  `inference/ggml_quants.rs` which is currently *scalar*, not even SIMD), NPU FFI, TEE, eBPF/XDP, QPU.

---

## 7. Items needing YOUR input (out-of-band)

- **Measured HRIR dataset** for full HRTF (vs the synthesized ITD/ILD HRIR shipped).
- **Whether to pin wgpu to a git commit** for the WGSL coopmat fix (#9741) vs. staying on the
  certified `29` pin (recommended: stay; CUDA WMMA already delivers tensor cores).
- **Priority of the remaining rollout** (orphan kernels → solver/ML consumers → LLM last, unless
  you re-order). The audio case showed mechanical map-grinding hits no-op rows — your domain
  steering avoids wasted effort.

---

## 8. Balance — what is solid (so the residuals read in proportion)

The WGSL Forge is complete and hardware-verified: 6 original builtin kernels + GEMM/GEMV/FFT, a
CPU-differential oracle, multi-backend emission (WGSL/MSL/HLSL/PTX/SPIR-V/CUDA-C), CUDA WMMA tensor
cores, ray-query GPU execution via BLAS/TLAS, a successive-halving correctness-gated tuner, a
topology-keyed cache, the `ForgeRuntime` consumer API, and a capability-aware best-path dispatcher
(f32-WGSL / f64-CUDA / df64 / CPU). Real consumers (solver GEMM/GEMV, `transforms/fourier`, the new
audio transforms) are wired and verified. Non-GPU suite green (75/0 in `wgsl_forge`); every GPU/CUDA
path was checked on the A2000. The residuals above are genuine and disclosed — none are hidden
behind a passing test or a "done" that isn't.
