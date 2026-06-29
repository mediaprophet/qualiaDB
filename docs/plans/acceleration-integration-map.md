# QualiaDB Acceleration Integration Map

Status: planning (inventory + wgsl-forge mapping) — **verified against the tree 2026-06-29**
Owner: Timothy Holborn
Companion to: [`deterministic-wgsl-forge.md`](deterministic-wgsl-forge.md)
Source root: `crates/qualia-core-db/src/` (the list below writes `src/…`; real paths are under this root)

This document inventories every QualiaDB compute hot-spot worth accelerating and maps each to a
concrete acceleration path, with priority on routing GPU-eligible work through the now-certified
**WGSL Forge** (the deterministic generate → validate → CPU/GPU-oracle → tune → certify pipeline).

**The key insight from the verification sweep:** QualiaDB already has an extensive hand-written
GPU stack — **~20 compute shaders** in `src/shaders/`, hand-dispatched via `include_str!` through
`gguf_bridge/`, `inference/`, `platform/`, `lora/`, `tensor/`, `modalities/`. The Forge's value is
**not** "add GPU acceleration" for those — it is to **replace hand-written, hand-tuned, mostly
un-oracle-tested shaders with generated kernels that carry a CPU-differential-oracle correctness
proof, an auto-tuned schedule, a topology-keyed cache, and MSL/HLSL/PTX/SPIR-V/CUDA portability.**
By contrast, the **solvers** and **specialized-science** libraries are still **CPU-only scalar** —
there the Forge genuinely *adds* GPU acceleration.

Scope honesty: the Forge accelerates **GPU compute via `wgpu`** (WGSL + MSL/HLSL/PTX/SPIR-V/CUDA-C
emission, a CUDA differential-oracle backend, and real tensor cores via CUDA WMMA). It does **not**
cover CPU-SIMD, NPU FFI, TEE, eBPF/XDP, or QPU — those rows are flagged **separate backends** (§5).

---

## 1. Certified Forge primitives (what exists today)

Certified + hardware-verified on an RTX A2000 (2026-06-29):

| Forge `BuiltinKernel` | Supersedes hand-written shader | Reuse for… |
|---|---|---|
| `TopK` | `shaders/topk_reduction.wgsl` (`inference/topk.rs`) | logit sampling, k-NN shortlist, any top-k/argmax |
| `TernaryGemv` | `shaders/ternary_gemm.wgsl` + `ternary_gemm_2bit.wgsl` (`inference/ternary.rs`) | BitNet b1.58 / 2-bit matmul |
| `FusedFfn` | `shaders/fused_ffn.wgsl` (`gguf_bridge/init.rs`) | SwiGLU MLP, dense matvec + activation |
| `P64Project` | (p64 manifold decode) | manifold/quin weight projection, packed-lane unpack |
| `RayProbe` (BLAS/TLAS ray-query) | (none — new RT capability) | BVH traversal, spatial queries, RT cores |
| `AffineF32` | element-wise prologue | scale/bias, activation prologues |
| CUDA `WMMA_GEMM_16X16` (tensor cores) | — | dense f16→f32 GEMM on tensor cores |
| `coopmat::matmul_tc_wgsl` (emit+validate) | — | WGSL tensor-core GEMM once wgpu ships #9741 |

Reusable IR ops for new kernels (no raw WGSL): `Load/Store/Mul/Add/Fma`, `Gelu/Relu`, `DotProduct`,
`Barrier` (+shared-memory), `Intrinsic::RayQuery`. New ops get an opcode + emitter arm and inherit
the whole certify/tune/cache path.

**Existing GPU dispatch infra the Forge supersedes** (from the sweep): a shared process-wide wgpu
device (`gpu_context::shared_gpu`), `include_str!` shaders composed by string-substitution/concat
(`dequant_template`→`fused_ffn`; `coop_gemv_subgroup`→`fused_transformer`), `gguf_bridge/pipeline_cache.rs`,
DirectML (`inference/directml_bridge.rs`) / Metal (`inference/metal_bridge.rs`) / cudarc backends,
ad-hoc CPU oracles (`topk_cpu`/`ffn_cpu`/`matmul_cpu`/`p64_project_cpu`), and a manifest cache. The
Forge unifies all of this behind one verified, tuned, portable pipeline.

### Legend — "Forge path"

- ✅ **Certified** — a forge kernel already covers this; migrate the call site / wire it.
- ♻️ **Migrate** — an existing hand-written GPU shader; re-express as a certified forge kernel (gains oracle+tune+cache+portability).
- 🧩 **Composable** — build from existing forge IR primitives (matmul/dot/reduce); little new code.
- 🔨 **New forge kernel** — CPU-only today; needs a new `BuiltinKernel` + emitter + CPU oracle.
- 👻 **Orphan shader** — a `.wgsl` exists but nothing dispatches it; formalize as a forge kernel.
- 🚫 **Not a Forge target** — CPU-SIMD / NPU / TEE / eBPF / QPU / graphics (§5).

---

## 2. Hand-written compute shaders → Forge status

All exist under `src/shaders/` and are dispatched today (except 👻). Render/viewport shaders are
graphics (🚫) and listed at the end for completeness.

| Shader | Purpose | Dispatched from | Forge status |
|---|---|---|---|
| `topk_reduction.wgsl` | block top-K argmax | `inference/topk.rs` | ✅ `TopK` |
| `ternary_gemm.wgsl` / `ternary_gemm_2bit.wgsl` | BitNet ternary GEMM | `inference/ternary.rs` | ✅ `TernaryGemv` |
| `fused_ffn.wgsl` | SwiGLU FFN (gate·up + silu) | `gguf_bridge/init.rs` | ✅ `FusedFfn` (extend for SwiGLU) |
| `gemm_substrate.wgsl` | portable f32 GEMM baseline | `platform/compute_bridge/gpu_gemm.rs` | ♻️ → forge GEMM / WMMA |
| `fused_attention.wgsl` | GQA + Q4_K dequant + RoPE + online softmax | `gguf_bridge/init.rs` | ♻️/🔨 fused-attention kernel (needs softmax op) |
| `coop_gemv_subgroup.wgsl` | subgroup-reduce GEMV | `gguf_bridge/init.rs` (concat) | ♻️ → forge GEMV (subgroup variant) |
| `dequant_template.wgsl` | per-role Q4_K/Q6_K dequant (string-substituted) | `gguf_bridge/init.rs` | ♻️/🔨 block-dequant kernel |
| `fused_tensor_contraction.wgsl` | fused contraction (Phase 5) | `gguf_bridge/init.rs` | ♻️ → forge GEMM / WMMA |
| `quantized_embedding.wgsl` | zero-copy quantized embed matmul | `gguf_bridge/init.rs` | ♻️/🔨 dequant+gather kernel |
| `fused_transformer.wgsl` | per-layer quantized-weight GEMM scaffold | `gguf_bridge/init.rs` | ♻️ → forge GEMM core |
| `wasm_elementwise.wgsl` | RMSNorm, SiLU×mul, residual-add | `gguf_bridge/init.rs` | ♻️/🧩 elementwise (Fma/Relu/Gelu ops; RMSNorm = reduce) |
| `f16_gemv.wgsl` | f16 GEMV baseline | `inference/ternary_gpu.rs` | ♻️ → forge GEMV |
| `gemv_bench.wgsl` | capability-probe GEMV | `platform/device_benchmark.rs` | ♻️ (or keep as a probe) |
| `lora_apply.wgsl` | LoRA additive delta (B·(A·x)·scale) | `lora/webgpu_lora.rs` | ♻️/🧩 two chained matvecs |
| `calculus.wgsl` | Simpson/trapezoid integration + tree-reduce | `platform/gpu.rs` | ♻️/🧩 reduce kernel |
| `sieve.wgsl` | GPU prime/quin sieve (bitmask out) | `platform/npu_ffi.rs` | ♻️ (u32 bit-ops; integer kernel) |
| `tensor_volume.wgsl` | 10D distance filter over quins | `tensor/volume_gpu.rs` | ♻️/🧩 batched distance + filter |
| `diffusion.wgsl` | discrete diffusion / graph denoise CA | `modalities/diffusion.rs` | ♻️/🔨 stencil kernel |
| `molecular_dynamics.wgsl` | Verlet MD + PBC | **👻 not dispatched** | 👻 → forge N-body kernel |
| `kinematics.wgsl` | N-body / Lennard-Jones / electrostatics | **👻 not dispatched** | 👻 → forge N-body kernel |
| `fluid_dynamics.wgsl` | Navier-Stokes cell update | **👻 not dispatched** | 👻 → forge stencil kernel |
| `quantum_bio.wgsl` | electron tunneling / radical-pair evolution | **👻 not dispatched** | 👻 → forge kernel (if used) |
| `viewport/*.wgsl`, `webizen-render/*.wgsl` | ambient/bloom/mesh/projector/screen/spectral/epistemic | viewport/render | 🚫 graphics (out of scope) |

> The 👻 orphans are a quick win: four already-written compute shaders with no caller — formalizing
> them as forge kernels gives them a correctness oracle and a live, tuned dispatch path.

---

## 3. Call-site inventory (your list, verified + grounded)

All paths below were confirmed to exist. **"Now"** = current acceleration status from the code.

### 3.1 LLM inference — `gguf_bridge/` is already GPU; migrate to certified kernels

| File | Operation | Now | Forge path |
|---|---|---|---|
| `gguf_bridge/gemm.rs` | GEMM | GPU (dispatches `gemm_substrate`/`fused_transformer`) | ♻️ → forge GEMM + ✅ CUDA WMMA tensor cores |
| `gguf_bridge/attention.rs` | MHA / GQA | GPU (`fused_attention.wgsl`) | ♻️/🔨 fused-attention forge kernel (needs softmax reduce op) |
| `gguf_bridge/attention/fused_tail.rs` | softmax & pooling | GPU | 🔨 softmax reduce (new op); pooling reuses `TopK` tree. CPU path 🚫 SIMD |
| `gguf_bridge/quant_support.rs` | block dequant Q4_K/Q8_K | GPU (`dequant_template.wgsl`) | ♻️/🔨 block-dequant kernel (`TernaryGemv`/`P64Project` unpack are templates) |
| `gguf_bridge/ffn.rs` | FFN / SwiGLU | GPU (`fused_ffn.wgsl`) | ✅ `FusedFfn` (extend to SwiGLU gate/up) |
| `inference/ternary_gpu.rs` + `inference/ternary.rs` | ternary / 2-bit matmul | GPU (`ternary_gemm*`) | ✅ **`TernaryGemv`** — direct migrate, lowest-risk win |
| `inference/topk_gpu.rs` + `inference/topk.rs` | top-K logit sampling | GPU (`topk_reduction`) | ✅ **`TopK`** — direct migrate |
| `inference/ggml_quants.rs` | quantized dot products | **CPU scalar** (no SIMD yet) | 🚫 (CPU SIMD/NPU target) / 🧩 (a GPU dot/GEMV variant) |
| `inference/inference_awq.rs` | AWQ weight quant | **CPU scalar** | 🧩 fused contraction → CUDA WMMA / `fused_tensor_contraction` migrate |

### 3.2 Linear algebra — **all CPU-only scalar today** (`solvers/linear_algebra/`)

| File | Operation | Now | Forge path |
|---|---|---|---|
| `svd.rs` | SVD (via AᵀA eigen) | CPU scalar | 🔨 (one-sided Jacobi + rotation kernel) |
| `eigen.rs` | symmetric eigen (cyclic Jacobi) | CPU scalar | 🔨 (Lanczos/QR-iter; matvec is 🧩) |
| `cholesky.rs` | Cholesky | CPU scalar | 🔨 (blocked GEMM tiles + panel kernel) |
| `qr.rs` | Householder QR | CPU scalar | 🔨 (Householder + trailing-GEMM) |

> **Build a blocked-GEMM core once**, then SVD/QR/Cholesky/eigen (and PCA/k-means/SVM below) reuse
> it. This is the single highest-leverage solver investment.

### 3.3 Transforms & calculus (`solvers/transforms/`, `solvers/calculus/`)

| File | Operation | Now | Forge path |
|---|---|---|---|
| `transforms/fourier.rs` | FFT | ✅ **WIRED** — `dft_accelerated()` routes the forward DFT through `dispatch::fft_f32` (WGSL forge radix-2, power-of-two N∈[2,1024]); CPU O(N²) floor + f64-exact `dft()` retained | ✅ done (forward; f32 spectral) |
| `transforms/laplace.rs` | Laplace | CPU scalar (Simpson) | 🧩 (quadrature as matvec) |
| `calculus/ode_solver.rs` | RK4/RK45 | CPU scalar (`PlatformGpuIntegrator` stub) | 🔨 (parallel state propagation; `Fma`) |
| `calculus/dense.rs` | Jacobian/Hessian | CPU scalar (finite diff) | 🧩 (batched column-parallel eval) |

### 3.4 Learning / classical ML (`solvers/learning/`)

| File | Operation | Now | Forge path |
|---|---|---|---|
| `clustering/kmeans.rs` | centroid distances | CPU scalar | 🧩 (‖x−c‖² = GEMM + reduce) |
| `dimensionality/pca.rs` | covariance / PCA | CPU scalar (`gemm()`+`symmetric_eigen()`) | 🧩 (XᵀX = forge GEMM; eigen → §3.2) |
| `classification/svm.rs` | RBF kernel matrix | CPU scalar (SMO) | 🧩 kernel matrix (distance+exp); SMO loop stays CPU |
| `trees/random_forest.rs` | ensemble traversal | CPU scalar | 🚫 (divergent) / 🔨 (GPU bucketed-eval, low priority) |

### 3.5 Audio (`audio/`) — real forward transforms now wired

> **2026-06-29:** the bake modules previously only *synthesized* / log-resampled / analytic-panned —
> the genuine forward transforms over real samples were missing. They now exist (additive; all prior
> fns + tests retained). STFT routes its per-frame FFT through the forge (§3.3 `dispatch::fft_f32`);
> CQT and HRTF are direct CPU (correct, cold-path ingest). 42 `audio::` tests pass (was 30).

| File | Operation | Now | Forge path |
|---|---|---|---|
| `stft.rs` (NEW) | forward STFT | ✅ **WIRED** — `forward_stft` Hann-windows each frame and runs it through `dispatch::fft_f32` (WGSL forge, CPU DFT floor); `stft_magnitudes`; `bake_stft_sidecar_from_samples` (real STFT → preview bins) | ✅ done (forward; uses §3.3 FFT) |
| `cqt_bake.rs` | Constant-Q | ✅ **REAL** — `forward_cqt` direct constant-Q (Hann-windowed complex inner product per log-spaced bin); `bake_cqt_sidecar_from_samples`. CPU (cold-path ingest) | 🧩 future: FFT + log-bin GPU path |
| `hrtf.rs` | HRTF convolution | ✅ **REAL** — `convolve_fir` (direct linear), `synthesize_hrir` (ITD fractional-delay + ILD gain + contralateral head-shadow LP), `binaural_render`. Measured KEMAR HRIRs remain an optional cold asset | 🔨 future: FFT-domain conv reuses §3.3 |

### 3.6 Geometric algebra & rendering

| File | Operation | Now | Forge path |
|---|---|---|---|
| `geometric_algebra/simd_kernel/operations.rs` | rotor/wedge products | CPU scalar (SIMD-capable backend, scalar dispatch) | 🚫 CPU AVX-512 (separate); GPU multivector 🔨 niche |
| `render/physics/aabb.rs` | BVH/AABB | CPU scalar (PGA motor per corner) | ✅ **`RayProbe`** — direct wire-up (RT cores) |
| `render/gpu/particles.rs` | particle kinematics | CPU staging (LCG) | 🔨 kinematics (`Fma`); collisions = spatial hash (new) |
| `render/gpu/bloom.rs` | bloom post-pass | GPU **render pass** (Kawase) | 🚫 graphics (not a compute kernel) |

### 3.7 Crypto — mostly **not** Forge (integer/modular or TEE)

| File | Operation | Now | Forge path |
|---|---|---|---|
| `crypto/zk_proofs.rs` | ZK (Halo2) NTT / commitments | CPU (Halo2 circuit) | 🔨 NTT (u32 modular) — strains f32 orientation; 64-bit care |
| `crypto/pq_kem_shim.rs` | PQ KEM (Kyber) | CPU (fips203) | 🚫 CPU SIMD |
| `crypto/sanctuary_crypto.rs` | AEAD / key mgmt | CPU (AES/ChaCha) | 🚫 not GPU-accelerable; TEE scope |

### 3.8 Specialized sciences & manifold

| File | Operation | Now | Forge path |
|---|---|---|---|
| `specialized_libs/chemistry_modeling/molecular_dynamics.rs` | N-body forces | CPU (+ 👻 `molecular_dynamics.wgsl`) | 👻/🔨 N-body kernel (`DotProduct`) |
| `specialized_libs/quantum_biology/quantum_state.rs` | DFT Hamiltonian | CPU (+ 👻 `quantum_bio.wgsl`) | 🚫 QPU (bridge) / 🧩 (Hamiltonian = GEMM + eigen §3.2) |
| `q42/p64_weight.rs` | manifold weight encode/decode | CPU scalar (serialize + CRC) | ✅ **`P64Project`** (decode/projection); pack/unpack = forge dequant |

### 3.9 Networking — not a Forge target

| File | Operation | Now | Forge path |
|---|---|---|---|
| `net/ebpf_filter.rs` | packet filter / telemetry (eBPF/WFP/XPC/VPN) | OS/kernel-space | 🚫 eBPF/XDP — entirely outside the GPU compute forge |

---

## 4. Omissions found in the sweep (verified to exist, not in your list)

| Path | Role | Forge fit |
|---|---|---|
| `specialized_libs/linear_algebra.rs` (+`computation.rs`) | matrix multiply/inverse/decompose lib | 🧩 prime GEMM consumer (feeds §3.2/§3.4) |
| `specialized_libs/machine_learning.rs` | inference/training engine, model zones | 🧩 GEMM + gradient reductions |
| `specialized_libs/physics_simulation.rs` | CFD/MD/kinematics solvers (owns the 👻 shaders) | 👻/🔨 formalize the orphan shaders here |
| `specialized_libs/statistical_computing.rs` | batched covariance/regression | 🧩 covariance = forge GEMM |
| `specialized_libs/engineering_analysis/thermal_conduction.rs` | tridiagonal Thomas / Fourier-law | 🔨 sparse/tridiagonal solve (lower priority) |
| `modalities/manifold.rs` | 10D manifold + `FixedLanczosEigensolver` | 🧩/🔨 **the manifold-WAL eigensolver on this branch** — unify with §3.2 |
| `tensor/resident_substrate.rs` + `tensor/volume_gpu.rs` | graph-tensor SOA + `tensor_volume.wgsl` (already GPU) | ♻️/🧩 distance filter + graph traversal (sieve + prefix-sum) |
| `inference/residency_planner.rs` | VRAM eviction ranking | 🔨 GPU prefix-sum/top-k for eviction (reuse `TopK`) |
| `inference/sparse_cache.rs` | sparse-attention mask / KV ring | 🧩 sparse gather/scatter (pairs with fused-attention) |
| `domains/biological/bioinformatics.rs` | sequence alignment, PSSM, quantum bio | 🔨/🧩 (owns 👻 `quantum_bio.wgsl`) |
| `governance/webizen.rs` (`execute_vm_frame`) | SLG resolution / forward chaining | 🧩 (only if VM ops do dense numeric work — likely logic-bound) |
| `audio/dsp_kernel.rs` | FM/τ modulation, spectral peaks | 🧩 (`Fma` synthesis + argmax reduce) |
| `lora/webgpu_lora.rs` | LoRA delta (owns `lora_apply.wgsl`, already GPU) | ♻️ two chained matvecs |
| `crypto/deontic_circuit.rs` | Groth16 R1CS (arkworks) | 🔨 (modular GPU arith; orthogonal to current scope) |
| `solvers/{optimization, special_functions, statistics, number_theory/modular, vector_calculus, interpolation}` | LM curve-fit, Bessel/Legendre, covariance, NTT, quadrature, least-squares | 🧩 mostly (GEMM/reduce/batched-eval); `number_theory/modular` underpins NTT 🔨 |

Out of forge scope but worth tracking as **non-forge backends**: `crypto/fiduciary_crypto.rs`
(ML-DSA, CPU), `solvers/graph_match/fuzzy_similarity.rs` (graph logic), `solvers/qpu/dispatcher.rs`
(QPU queue).

---

## 5. Non-Forge backends (separate work, flagged honestly)

Each is its own engineering effort, **not** GPU-compute-forge work:

- **CPU SIMD** (AVX2/512/NEON): `inference/ggml_quants.rs` (currently *scalar* — SIMD is itself a gap),
  `crypto/pq_kem_shim.rs`, `geometric_algebra/simd_kernel`, the CPU columns throughout.
- **NPU** (`platform/npu_ffi.rs`): neural-accelerator GEMM/dot — note it currently dispatches `sieve.wgsl` (wgpu), so the "NPU FFI" is partly a wgpu path today.
- **TEE** (`tee_ffi`, Secure Enclave/TrustZone): `sanctuary_crypto.rs`, key management.
- **QPU** (`solvers/qpu/dispatcher.rs`, `specialized_libs/qpu_bridge`): `quantum_state.rs` DFT.
- **eBPF/XDP**: `net/ebpf_filter.rs`.

A future **compute dispatcher** could pick Forge-GPU vs SIMD vs NPU vs QPU per op using the same
capability-manifest the Forge already uses for schedule pruning — the natural unifying abstraction,
but each backend is real, separate work.

---

## 6. Suggested rollout order (leverage × low-risk-first)

1. **Three direct ✅ migrations** — `inference/ternary*.rs → TernaryGemv`, `inference/topk*.rs → TopK`,
   `render/physics/aabb.rs → RayProbe`. Plus `q42/p64_weight.rs → P64Project`. Existing certified
   kernels; this validates the call-site migration contract end-to-end with near-zero risk.
2. **FFN** — `gguf_bridge/ffn.rs` `fused_ffn.wgsl → FusedFfn` (extend the kernel to SwiGLU gate/up).
3. **Tensor-core GEMM** — route `gguf_bridge/gemm.rs` + `fused_tensor_contraction` to CUDA WMMA
   (certified); keep the WGSL coopmat tile ready for wgpu #9741.
4. **Block dequant** — `gguf_bridge/quant_support.rs` `dequant_template.wgsl` → a certified
   block-dequant kernel (`TernaryGemv`/`P64Project` unpack are templates); unblocks Q4_K/Q8_K.
5. **Fused attention** — `gguf_bridge/attention.rs` + `fused_tail.rs`: the biggest new kernel; needs
   a softmax reduce op. Highest value for the forward pass.
6. **Adopt the 4 orphan shaders** (`molecular_dynamics`/`kinematics`/`fluid_dynamics`/`quantum_bio`)
   as forge kernels — quick wins: already-written compute, just needs an oracle + live dispatch.
7. **Blocked-GEMM core → linear algebra + ML** (§3.2/§3.4): build once, reuse for SVD/QR/Cholesky/
   eigen and PCA/k-means/SVM, plus `modalities/manifold.rs` (the WAL eigensolver) and the
   `specialized_libs/{linear_algebra,machine_learning,statistical_computing}` consumers.
8. **One FFT kernel → transforms + all audio** (§3.3/§3.5) — replaces the naive O(N²) DFT and serves
   FFT/STFT/CQT/HRTF.
9. Lower priority: render particles, bloom-as-compute, ZK-NTT (integer-modular caveat), thermal/PDE.

---

## 7. Honest caveats

- "Certified" means oracle-verified + timed on **this** topology, not a universal guarantee; each new
  target needs its own CPU reference.
- Migrating a shader to the Forge is mostly **risk-reduction** (correctness oracle + tuning +
  portability), not raw speed — the hand-written shaders may already be fast; the Forge makes them
  *provably correct, auto-tuned, and multi-backend*.
- Integer/modular (NTT, sieve) and 64-bit-heavy work strain the Forge's f32/u32 orientation (64-bit
  is paired-u32 today) — feasible but flagged per-row.
- This map is an inventory + strategy, **not** an implementation. Each ✅ is a quick migration; each
  🔨 is a real kernel with its own oracle and certify pass.
- One region (`inference/*` per-file roles) was partly inferred from the shader-dispatch map because
  the LLM verification agent overran its output budget; the `gguf_bridge/` structure, shader
  dispatch sites, and CPU-scalar status of `ggml_quants.rs`/`inference_awq.rs` were directly confirmed.
