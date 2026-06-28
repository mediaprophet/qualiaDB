# Qualia WGSL Forge — Deterministic Generator, Certifier, and Tuner

Status: implementation in progress  
Started: 2026-06-28  
Primary branch: `feature/p64-manifold-wal-eigensolver`

## 1. Purpose

Qualia WGSL Forge removes hand-written layout arithmetic and ad-hoc shader tuning
from the inference and scientific-compute pipelines. It deterministically generates
highly optimized target-specific shader code (WGSL, MSL, HLSL, or PTX) from a shared typed kernel description. It proves the generated module is structurally valid, checks GPU results against a CPU oracle, and searches a bounded hardware schedule space to lock in peak performance for the exact local hardware topology.

The Forge is designed for decentralized, heterogeneous deployment. It acknowledges that a statically compiled schedule cannot optimally serve all hardware. Therefore, the Forge acts as a local evaluator—using the system's specific adapter topology (e.g., unified memory vs. discrete, thermal constraints, compute unit counts) to prune the search space, run the hardware-specific "checker," and lock in the optimal schedule for that exact machine.

The system must preserve these project constraints:

- P64 remains the canonical on-disk format; GPU execution layouts are derived views.
- The IR understands native 64-bit types (e.g., `f64`, `u64`). Representing 64-bit values as paired `u32` words is an emission-level lowering handled strictly by the WGSL/SPIR-V fallback pipelines.
- Generated runtime kernels do not allocate from the heap.
- Search is bounded, reproducible, and safe under device loss or unsupported features.
- A shader is never called “certified” merely because it parses.
- Tuning results are adapter-, driver-, schema-, and shader-specific.

## 2. Architectural decision

The implementation is a reusable core module plus a thin CLI. A procedural macro is
not the source of truth: derive macros cannot reliably observe final Rust layout.
Build integration may be added later as a thin caller of the same deterministic API.

The compiler separates semantics from scheduling, using a `LoweringContext` pass that takes the `KernelSpec`, `HardwareTopology`, and `Schedule` to produce either a canonical `naga::Module` (for wgpu backends) or target-ready IR (for native backends like PTX). This keeps semantics pristine while schedules diverge, introducing these diverging execution paths:

```text
KernelSpec (typed buffers, operations, Hardware Intrinsics [MMA, RayQuery], CPU oracle)
    + Local Hardware Topology (Compute units, VRAM limits, Tensor/RT presence)
    + Schedule (workgroup, tile, vector width, warp sizing)
    -> constraint validation & schedule pruning
    -> Backend Emission Branch:
         ├─> MSL (Metal)       -> wgpu pipeline
         ├─> HLSL (DXC)        -> SPIR-V -> wgpu pipeline
         ├─> WGSL (Fallback)   -> Naga -> wgpu pipeline
         └─> PTX (Native NV)   -> cudarc (Native CUDA Driver API)
    -> Execution & CPU/GPU differential oracle
    -> robust timing samples
    -> Local Hardware CertificationManifest
```

This separation ensures that tuning changes execution strategy without changing the
kernel’s mathematical meaning.

If a target-specific native backend (e.g., PTX or MSL) fails to initialize on the host, the Forge must automatically fall back to the next available compilation target (e.g., SPIR-V or WGSL) to ensure the compute pipeline remains operational, albeit at a potentially reduced performance tier.

Stateless Execution & Persistent Memory: The execution layer is strictly stateless. Dynamic allocation in the hot loop is forbidden. The orchestrator manages a persistent, topology-aware ring buffer (larger persistent slabs with zero-copy for unified memory, pinned staging rings with async `copy_buffer` for discrete PCIe devices) and passes lightweight offsets to the target-specific `QualiaCompute` implementation. Every backend must enforce the invariant that "read/write heads never lap" using proper memory fences.

## 3. Repository layout

Planned implementation layout:

```text
crates/qualia-core-db/src/wgsl_forge/
    mod.rs          public API and error model
    ir/
        mod.rs          
        core.rs         # Universal math and memory operations
        intrinsics.rs   # Hardware-specific nodes (Warp, MMA, RayQuery)
        capabilities.rs # Hardware capability matrix (including 64-bit arithmetic) and lowering fallback logic
    schedule.rs     bounded schedule parameters and adapter constraints
    emit/           # Target-specific deterministic writers
        mod.rs
        wgsl.rs     # Deterministic WGSL writer (Fallback)
        msl.rs      # Metal Shading Language
        hlsl.rs     # High-Level Shading Language (compiled to SPIR-V via DXC)
        ptx.rs      # NVIDIA Parallel Thread Execution assembly
    execute/        # Runtime bridges for the differential oracle
        mod.rs          # Exposes the QualiaCompute trait and backend selector
        compute.rs      # QualiaCompute trait (requires pre-allocated BufferViews)
        memory.rs       # Topology-aware Ring Buffer & Slab Allocator
        wgpu.rs         # Handles MSL, SPIR-V, and WGSL pipelines
        cuda.rs         # Uses `cudarc` to load and launch PTX modules
    validate.rs     Syntax/Semantic validation for generated outputs
    oracle.rs       CPU reference execution and tolerance comparison
    tune.rs         grid/racing search, topology checks, and cost scoring
    manifest.rs     certification and tuning records

crates/qualia-cli/src/shader.rs
    generate, validate, certify, tune, list-kernels, profile-hardware, auto-tune-all
```

If keeping the first slice smaller materially improves correctness, files may begin as
one module and be split along these boundaries before completion.

## 4. First supported kernel

The first end-to-end kernel is a deterministic vector transform:

```text
out[i] = input[i] * scale + bias
```

It is deliberately simple enough to certify exhaustively while exercising:

- typed storage and uniform buffers;
- workgroup and items-per-thread scheduling;
- bounds guards;
- WGSL generation;
- Naga validation;
- CPU oracle evaluation;
- adapter-backed execution;
- timing and tuning.

The next kernels are P64 descriptor projection, ternary dequant/GEMV, fused FFN, and
top-k reduction. Existing hand-authored shaders remain production defaults until their
generated equivalents have oracle-backed parity evidence.

## 5. Validation levels

Every result declares its achieved validation level:

1. `Generated` — deterministic source emitted.
2. `NagaValidated` — canonical `naga::Module` constructed, validated against `Capabilities` / subgroup settings, and passed `naga::valid::Validator`.
3. `PipelineCreated` — target adapter accepted shader and pipeline.
4. `OracleVerified` — GPU output matched CPU reference within declared tolerances.
5. `Profiled` — warm-up and robust timing samples completed.
6. `Certified` — all required levels passed and a reproducible manifest was written.

No level implies a later level.

## 6. Search and optimisation

Initial schedule space:

- workgroup size: `32, 64, 128, 256`;
- items per invocation: `1, 2, 4, 8`;
- vector width: initially `1`, then `2, 4` once vector emission is certified;
- optional tile dimensions for matrix kernels (`tile_mnk`);
- intrinsic flags (e.g., `use_subgroup`);
- memory access hints (e.g., `prefetch`);
- loop `unroll_factor`.

Candidates are pruned before compilation using:

- adapter workgroup limits;
- invocation and workgroup-storage limits;
- divisibility/alignment rules;
- kernel-specific constraints;
- memory architecture classification (e.g., heavily biasing tile dimensions differently for unified memory architectures versus discrete PCIe GPUs);
- local thermal and power profile constraints to avoid aggressive schedules that cause thermal throttling during the search;
- compute unit saturation thresholds derived from the local adapter's declared capabilities;
- warp-size alignment (strictly 32 for PTX/NVIDIA, flexible/64 for Vulkan/AMD/Apple);
- intrinsic availability checks: the checker must verify if the local adapter supports `SPV_KHR_cooperative_matrix` (Vulkan) or native Tensor/RT cores before exploring schedules that rely on them;
- hardware capability manifest: the local topology checker must query the adapter for advanced intrinsic support (e.g., Subgroup sizes, MMA/Tensor Core availability, async memory copy support, and RT core presence) before search begins;
- semantic lowering: if a kernel requires an intrinsic (like a warp reduction) that the local hardware lacks, the Forge must decide whether to gracefully lower that operation into a standard shared-memory equivalent, or exclude the schedule entirely;
- a simple roofline lower bound: estimating arithmetic intensity of the kernel vs device peak to reject clearly memory- or compute-bound bad schedules.

The first tuner uses deterministic grid search. The second stage uses successive
halving: all valid candidates receive a small sample budget, then only the strongest
receive additional samples. Ranking uses median latency with correctness as a hard
gate. Optional scoring includes p95 latency, throughput, memory, and estimated energy/thermal cost.

## 7. Measurement rules

- The differential oracle must only interact with the target hardware via the `QualiaCompute` trait; it must remain completely agnostic to whether the underlying backend is wgpu or cudarc.
- Timing mechanisms must be delegated to the backend implementation of the trait (e.g., using CUDA Events for PTX, and timestamp queries for wgpu), returning a standardized `Duration` to the tuning loop.
- Always perform warm-up dispatches.
- Prefer GPU timestamp queries when negotiated (or equivalent native event timing).
- Fall back to completion-timed batches and label the timing source honestly.
- Use multiple samples and median ranking; retain min, median, p95, and sample count.
- Never compare a profiled run’s end-to-end token throughput with an unprofiled run.
- Abort a candidate on validation error, pipeline error, timeout, device loss, or
  oracle mismatch. If a target adapter drops or a device is lost during execution, the trait must return a unified `DeviceLost` error rather than backend-specific panic codes, allowing the search space to safely prune the candidate and continue.
- Use fixed deterministic test vectors and record their seed/hash.

## 8. Tuning Signatures

Tuning records are reusable only when all signature fields match:

- `adapter_info` (name, vendor, device, device type, backend, driver and driver info);
- selected `limits` fields hash and `features` bits;
- WGSL Forge schema version;
- kernel semantic hash (canonical IR or op sequence);
- generated shader hash;
- P64 schema version where relevant;
- wgpu/Naga/cudarc version;
- correctness tolerance profile;
- tuning timestamp, source spec version/commit, and lightweight provenance note.

The `CertificationManifest` acts as a hardware-specific fingerprint. If a node boots up, it should first poll the cluster's manifest registry (or a local cache directory) for an identical topology manifest. Only if a match is absent does it drop into the `auto-tune-all` phase to generate and cache a new optimal manifest for that specific hardware profile.

The initial implementation serializes records as JSON. Runtime code may later use a
compact fixed-record format after the schema stabilizes.

## 9. CLI contract

Target interface:

```text
qualia-cli shader list-kernels
qualia-cli shader generate <kernel> [--target wgsl|msl|hlsl|spirv|ptx] [--schedule ...] [--out kernel.<ext>]
qualia-cli shader validate <file-or-kernel> [--target wgsl|msl|hlsl|spirv|ptx]
qualia-cli shader certify <kernel> [--manifest result.json]
qualia-cli shader tune <kernel> [--max-candidates N] [--manifest result.json]
qualia-cli shader profile-hardware [--export topology.json]
qualia-cli shader auto-tune-all [--budget-ms N] [--thermal-limit 75C] [--update-local-manifest]
```

Commands print human-readable summaries by default and support JSON output for CI.
Generation and Naga validation work without a GPU. Certification and tuning degrade
cleanly when no compatible adapter or timestamp-query feature exists.

## 10. Testing

Required tests:

- deterministic emission produces identical bytes for identical inputs;
- all generated baseline schedules pass Naga parse and semantic validation;
- invalid schedule and layout combinations are rejected before emission;
- WGSL layout uses paired `u32` words for every portable 64-bit field;
- bounds guards cover non-multiple dispatch lengths;
- CPU oracle produces known results;
- tolerance comparator handles finite values, NaN, infinity, absolute and relative error;
- search order and winner selection are deterministic for fixed measurements;
- manifest hashes and tuning signatures change when source/schema/schedule changes;
- headless GPU oracle test runs when an adapter is available and skips honestly otherwise;
- `profile-hardware` dumps rich, queryable JSON including limits, features, subgroup sizes, and memory architecture class;
- `tune` and `certify` support `--dry-run` for pruning analysis only;
- a small roofline visualizer or schedule search tree dump can be generated for debugging why a candidate was pruned;
- CLI smoke tests cover generation and validation without requiring a GPU;
- oracle tests must execute the exact same CPU reference vectors across all available `QualiaCompute` backend implementations on the host machine to ensure tolerance checks hold true across different compiler toolchains and hardware precision limits;
- oracle tests must explicitly verify that the read/write heads of the ring buffer do not lap each other during sustained, high-throughput asynchronous dispatches to guarantee memory safety across the `QualiaSlabAllocator`.

## 11. Continuation ledger

Update this section before ending any implementation session.

### Completed

- [x] Architecture and continuation plan created.
- [x] Repository inventory completed.
- [x] Typed kernel and schedule IR implemented (extended for multi-backend and intrinsics).
- [x] Deterministic emitters implemented (WGSL, MSL, HLSL; PTX affine-only, generic deferred).
- [x] `QualiaCompute` unified execution trait and `QualiaSlabAllocator` implemented
      (wgpu backend GPU-correct: 256-aligned, split read/read-write slabs).
- [x] CPU oracle and tolerance contract implemented.
- [x] Certification manifest implemented (topology fingerprint via `HardwareProfile`).
- [x] Deterministic tuner implemented.
- [x] Adapter cache identity and atomic manifest cache implemented (topology-keyed lookup).
- [x] CLI commands implemented: list-kernels, generate, validate, certify, tune,
      profile-hardware, auto-tune-all, plus `--target` flags and `--dry-run`.

### Next exact action

Top-k is GPU-certified, MSL/HLSL top-k emit + DXC-validate, certify/tune handle
top-k, and ray-query WGSL emits + Naga-validates (see evidence below). Remaining:

1. GPU execution of ray-query: build BLAS/TLAS via wgpu's acceleration-structure
   API and request `EXPERIMENTAL_RAY_QUERY` at device creation so `ray-probe`
   can be oracle-verified on the A2000 (today it is emitted + Naga-validated but
   not yet executed — this is the one genuinely separate subsystem left).
2. Tensor-core (cooperative-matrix) MMA emission for the fused-FFN matmul, gated
   on the detected `supports_coopmat` (the A2000 reports 6 coopmat configs).
3. Real fused-FFN math (it still uses placeholder high-level `DotProduct` nodes)
   with an oracle, then certify it on hardware.
4. MSL/HLSL ray-query bodies (Metal/DirectX RT use distinct APIs).

### Decisions still open

- Generated shaders remain command output; they are not automatically checked into
  source control.
- Whether the eventual build wrapper is `xtask`, `build.rs`, or a thin derive helper.
- How to execute the `auto-tune-all` process across a decentralized cluster: Should it run as an Ahead-Of-Time (AOT) installation script via the CLI, or as a Just-In-Time (JIT) initialization on the first launch of a new node? Furthermore, how do we distribute these manifests across a local cluster (e.g., Apple Silicon or Raspberry Pi swarms) of identical hardware to prevent redundant, expensive tuning benchmarks on every single node?

### 2026-06-28 implementation evidence

- 14 deterministic Forge tests pass; one native-GPU test is opt-in.
- Full library binary: 2,133 passed, 0 failed, 2 ignored.
- `cargo tree -p qualia-core-db --no-default-features -e normal` contains neither
  `naga` nor `wgsl-forge`, confirming the lite dependency graph excludes Forge.
- The repository-wide native `--no-default-features` compile is not currently a valid
  gate: pre-existing platform/inference modules reference optional `wgpu` without
  feature guards and produce errors outside Forge.
- Naga validated scalar, `vec2`, and `vec4` generated variants.
- Real certification passed on NVIDIA RTX A2000 12GB for a 4,099-element tail case:
  median 7,616 ns and p95 9,728 ns in the observed run.
- A bounded eight-candidate real tuning run selected
  `workgroup=32, items=2, vector=1`: median 6,880 ns and p95 7,136 ns in the
  observed run.
- These timings are hardware/run evidence, not universal constants; the adapter-keyed
  cache prevents applying them to a different device or shader/schema hash.

### 2026-06-28 Phase 2 evidence (shared memory + barriers + top-k)

- IR gained reusable workgroup primitives: `Op::Barrier`
  (`workgroupBarrier()` / `threadgroup_barrier` / `GroupMemoryBarrierWithGroupSync`
  across WGSL/MSL/HLSL) and `SharedMemorySpec` + `SharedLen::{Fixed,WorkgroupSize}`
  on `KernelSpec.shared_memory`. The new fields are `serde(default,
  skip_serializing_if)` so existing kernels' semantic hashes are unchanged.
- New `BuiltinKernel::TopK`: one workgroup per `block_size`(= workgroup-size) block,
  emitting the `k` largest values per block in descending order via a barrier-
  synchronised tree arg-max reduction over `var<workgroup>` arrays. Shared-array
  decls are driven by the IR; the reduction control flow is WGSL-specialised.
- Verified offline (no GPU): generated top-k WGSL passes full Naga validation —
  including barrier **uniformity** analysis — for workgroup sizes 32/64/128/256;
  CPU oracle (`topk_cpu`) matches a brute-force reference for full and partial
  tail blocks. Forge suite: 17 passed / 0 failed / 2 ignored.
- Not yet done (honest): GPU OracleVerified for top-k is an opt-in `#[ignore]`
  test (`generated_topk_matches_oracle_on_real_gpu`) pending a hardware run on this
  machine's adapter; MSL/HLSL/PTX top-k emission returns a clear "WGSL-only this
  phase" error; `certify`/`tune` remain affine-only (non-affine oracle path is a
  named follow-up). cudarc was modernised 0.11→0.19 (official, `cuda-13030`).

### 2026-06-28 cooperative-matrix (tensor-core) emission — partial (superseded 2026-06-29)

- Forge can emit valid WGSL cooperative-matrix code: `emit/coopmat.rs` produces an
  8x8 GEMM tile (`enable wgpu_cooperative_matrix`, `coop_mat8x8<f32, role>`,
  `coopLoadT`/`coopMultiplyAdd`/`coopStoreT`). It passes full Naga validation with
  `Capabilities::COOPERATIVE_MATRIX` (`cooperative_matrix_tile_validates`).
- `coopLoadT`/`coopStoreT` round-trip **correctly** on the A2000 (load `a` as role
  C, store to `c` → `c == a`; `coopmat_loadstore_roundtrips_on_real_gpu`), but
  `coopMultiplyAdd` returned **~all-zero** output. The 2026-06-28 conclusion
  attributed this to a generic "experimental wgpu defect" — correct direction, but
  imprecise. Superseded by the 2026-06-29 root-cause + resolution below.

### 2026-06-29 tensor-core multiply — root-caused + delivered via CUDA WMMA

Re-opened under the completeness bar (don't dress a gap as a follow-up) with an
adversarial recon (3 agents: cudarc nvrtc API, a verified WMMA-via-NVRTC recipe, and
an adversarial check of the "upstream-blocked" claim). Two hypotheses were tested
empirically on the A2000 and the cause was pinned exactly from the installed
naga/wgpu 29.0.3 source:

- **Geometry is NOT the issue.** naga declares the coop-matrix SPIR-V type with
  `Scope::Subgroup` (`naga-29.0.3/back/spv/writer.rs`: `get_index_constant(spirv::Scope::Subgroup)`),
  so on NVIDIA one 32-lane warp is the full participation set. `@workgroup_size(32)`
  is correct; re-running at `@workgroup_size(8,8,1)` (64 invocations) produced the
  **identical** all-zero result, ruling out the "needs 64 lanes" theory.
- **dtype WAS half the issue, but not the whole story.** `wgpu-types-29.0.3/features.rs:1375`
  states EXPERIMENTAL_COOPERATIVE_MATRIX "currently only supports 8x8 **f32**
  matrices" (Vulkan gates on `vkGetPhysicalDeviceCooperativeMatrixPropertiesKHR`
  for 8x8x8 f32). The prior emitter used f16-in/f32-acc — an unsupported config.
  The emitter is now **all-f32 8x8x8** (the supported config) and naga-validates.
- **Real root cause:** even the all-f32 8x8x8 multiply returns all-zeros on the
  29.0.3 *execution* path. This matches wgpu #9729/#9741 (coopmat emits Device-scope
  SPIR-V memory ops that are invalid/no-op'd unless `vulkanMemoryModelDeviceScope`
  is auto-enabled — a fix that landed on git `main` **after** 29.0.3). **29.0.3 is
  the newest wgpu on crates.io**, so there is *no published release* that fixes it;
  the only WGSL-path fix would be pinning wgpu to an unreleased git commit (a
  core-dependency supply-chain decision, deferred to Timothy). naga's own
  cooperative-matrix test is a WGSL→SPIR-V *translation* test, not a GPU-execution
  test, so its passing never implied the multiply executes.
- **Delivered the tensor-core goal concretely via CUDA WMMA** (the path wgpu 29
  cannot take at all): `emit/cuda_c.rs` `WMMA_GEMM_16X16_SRC` emits a single-warp
  `nvcuda::wmma` GEMM `C(16x16,f32) = A(16x16,f16) * B(16x16,f16)`, compiled by
  NVRTC for the device's real compute capability (`compute_86` on the A2000) with
  the toolkit include dir on the search path (NVRTC's default include list is empty,
  so `<mma.h>` needs `--include-path` explicitly). This is the **genuine
  reduced-precision tensor-core path** (f16 inputs, f32 accumulate). It runs and
  matches `matmul_cpu` to f16 input precision on the A2000:
  `wmma_matmul_certifies_on_cuda_tensor_cores` — **passing**. The emitted PTX
  contains a real `HMMA mma.sync` (verified during recon), i.e. it lowers to tensor
  cores, not scalar FMA.
- The `compile_cuda_c` path was modernised (cudarc `compile_ptx_with_opts` with
  detected `arch` + include paths) and the affine/ffn/top-k cross-backend CUDA
  oracle tests **still pass** (regression-clean).
- **Net:** tensor-core matmul is **emitted, validated, and hardware-verified** —
  bit-approximately correct on real tensor cores via the CUDA backend. The WGSL
  coopmat path is emitted + naga-validated + load/store-verified; its multiply
  *execution* is blocked by wgpu 29.0.3 with no published fix, documented precisely
  and ready to light up (`evaluate_matmul_tc`) the moment wgpu ships #9741. Nothing
  faked, nothing dressed up as a follow-up.

### 2026-06-29 ray-query GPU execution via BVH (BLAS/TLAS) — done, hardware-verified

The second frontier item. Applied the same recon-first playbook to de-risk wgpu 29's
*other* experimental subsystem before building, given the coopmat lesson that 29.0.3's
experimental execution paths can be silently broken.

- **Feasibility settled empirically — ray-query EXECUTES on the A2000** (unlike
  coopmat). The adversarial-feasibility recon agent crashed before emitting its verdict,
  but its preview ("fully implemented, not stubbed" — wgpu-hal 29.0.3 Vulkan) plus the
  decisive on-hardware test confirmed it. The one blocker was **config, not upstream**:
  the device was created with `Limits::default()`, where the acceleration-structure
  limits (`max_blas_geometry_count`/`max_blas_primitive_count`/`max_tlas_instance_count`/
  `max_acceleration_structures_per_shader_stage`) default to **0**, so `create_blas`
  failed validation. Fixed by raising them from the adapter's reported limits.
- **End-to-end GPU execution implemented:**
  - `execute/wgpu.rs`: requests `EXPERIMENTAL_RAY_QUERY`; `build_triangle_scene()`
    creates + builds an `OPAQUE` triangle BLAS and a 1-instance TLAS (identity
    transform); `WgpuPipeline::dispatch_rayprobe()` binds the TLAS as the
    `acceleration_structure` at binding 0 alongside the rays/hits buffers (the generic
    dispatch only binds buffers, so this is a dedicated path).
  - `emit/wgsl.rs`: fixed the ray-query lowering to **loop `rayQueryProceed` to
    completion** — a single call can leave a multi-node BVH partially traversed (§13
    fix-along-the-way).
  - `oracle.rs`: `evaluate_rayprobe` builds the scene, dispatches the emitted
    `ray_probe` WGSL over a fixed 12-ray set (7 clean interior hits, 5 clean misses),
    and checks committed hit distances against a **Möller–Trumbore** CPU reference
    (`rayprobe_cpu`). Wired into `evaluate_builtin` + `has_gpu_oracle`, so `certify`/
    `tune` treat RayProbe as a first-class builtin; non-RT adapters skip it via the
    existing `supports_kernel` gate (no regression).
- **Verified on the A2000:** `rayprobe_certifies_on_real_gpu` (committed t = 3.0 for the
  7 hits, −1 for the 5 misses, within 1e-2) and `rayprobe_certify_builtin_on_real_gpu`
  (full certification manifest) — both **passing**; `rayprobe_cpu_reference_is_sane`
  (non-GPU). Full forge GPU+CUDA regression after enabling the feature + raising limits:
  **11 passed / 0 failed** (affine/topk/ffn/p64 certify, coopmat load/store, WMMA, 3×
  CUDA oracle, 2× ray-probe) — device-creation change is regression-clean.
- **Net:** ray-query GPU execution via BVH is real and hardware-verified — task done.
  Both frontier items (tensor cores #18, ray-BVH #19) are now delivered on real hardware.

### 2026-06-29 completeness audit — the plan is NOT fully implemented (honest correction)

An independent two-angle audit (plan→code requirement-by-requirement, and code→plan
stub sweep) was run against this plan. **Correction to earlier "fully implemented"
claims in this ledger / the task list: that was overstated.** The findings:

- **Code→plan: clean.** No `todo!()`/`unimplemented!()`/reachable `unreachable!()` on any
  runtime path. All 5 built-ins (affine/ffn/p64/topk/ray-probe) are fully emitted on WGSL,
  naga-validated, and GPU-graded by real differential oracles; native backends (MSL/HLSL/
  PTX/CUDA-C) cover documented subsets with explicit `Err` returns, not silent stubs. The
  one silent footgun found — `Op::MatrixMultiply` emitting a no-op comment in WGSL/MSL/HLSL
  — was changed to return `Err` (it is unused by every built-in spec; tensor-core GEMM is
  delivered via coopmat/CUDA WMMA). One stale doc comment on `has_gpu_oracle` was fixed.
- **Plan→code: a real unfinished tail beyond the two frontier items.** Genuinely DONE:
  deterministic multi-backend emission, validation levels, the successive-halving
  correctness-gated tuner, topology-keyed manifest cache, the ring-buffer slab + lap test,
  real ffn/topk/p64 math with wgpu+CUDA cross-backend oracles, and both frontier items.
  Still **partial or missing** (normative requirements, not the two accepted items):
  - §2 **automatic native→fallback backend selection** — missing (no PTX/MSL→WGSL drop on
    init failure).
  - §2 **topology-aware memory paths** — tags only; no zero-copy-unified / pinned-staging+
    async-copy-discrete differentiation (`wgpu.rs` treats both identically).
  - §4 **ternary dequant/GEMV kernel** — absent (no built-in, emitter, or oracle).
  - §6 **four dead schedule knobs** (`tile_mnk`, `use_subgroup`, `prefetch`, `unroll_factor`)
    — `Schedule` fields never varied in the search and never consumed by an emitter.
  - §6 **device-relative roofline reject, memory-arch tile biasing, CU-saturation, thermal/
    power pruning** — unimplemented. These were honestly flagged here earlier but **task #14
    was marked "completed"** — that task is reconciled to reflect reality (warp-alignment +
    roofline *estimate* done; biasing/CU/thermal not). Peak-FLOPS/bandwidth and CU count are
    not exposed by wgpu (need a calibration micro-benchmark); thermal needs `nvidia-smi`.
  - §7 **unified `DeviceLost` error + per-candidate timeout** — missing.
  - §7 the **oracle is not written generically over `QualiaCompute`** (concrete
    `&mut WgpuComputeContext`; parallel `evaluate_*_cuda`; ray-probe bypasses the trait).
  - §7/§8 **test-vector seed/hash and the full tuning signature** (limits hash, feature bits,
    cudarc version, tolerance profile, P64 schema, timestamp/commit/provenance) are not
    recorded in the manifest/cache key — the fingerprint is coarser than specified.
  - §9 CLI **`--budget-ms`, `--thermal-limit`, and a `spirv` target** — missing.
  - §12 no enforced pre-tuning **setup gate** / no auto-fallback (degradation is "WGSL is
    default", not "fall back on native failure").
  - §1 **native f64/u64 IR type** — `ScalarType` has only `U64Words` (paired-u32); the
    `LoweringContext::policy_64bit`/`supports_f64` scaffolding is never exercised.
  - §10 targeted test gaps (CLI-binary smoke test; "invalid schedule rejected *before*
    emission"; cache-key sensitivity; general paired-u32 64-bit-field WGSL layout).
  - `validate.rs::validate_native` returns hardcoded report metadata
    (`entry_points=["affine_f32"]`, `binding_count=3`) for any kernel — the compile is real,
    the metadata is wrong for non-affine (disclosed in code + CLI).

**Honest status:** the core + both frontier items are real and hardware-verified; the plan
is **~80% implemented**, with the tail above outstanding. The remaining work is tracked as
explicit tasks (see the task list) and is the subject of a scope/priority decision with
Timothy — several items are implementable by the agent; a few (peak-FLOPS/CU/thermal
pruning) need platform probes or are honest documented limitations.

### 2026-06-28 generic CUDA via NVRTC CUDA-C (affine/ffn/top-k)

- Generic CUDA execution now mirrors the HLSL->DXC path: a CUDA-C emitter
  (`emit/cuda_c.rs`, `TargetBackend::CudaC`) is compiled to PTX by NVRTC at
  runtime and run via cudarc. Covers affine, fused-ffn, and top-k (CUDA-C
  `__shared__` + `__syncthreads__`). `cuda.rs` dispatch is now generic: storage
  buffers become pointer args in binding order, the uniform block is passed by
  value last (derived from the kernel spec).
- Another real environment bug, found by running: the installed toolkit (NVRTC
  13.3) is newer than the driver (CUDA 13.2), so the driver rejected NVRTC's PTX
  with CUDA_ERROR_UNSUPPORTED_PTX_VERSION. Fixed by rewriting the PTX `.version`
  directive down to a driver-supported ISA (8.0; our kernels use only long-stable
  instructions). The hand-written affine PTX worked earlier precisely because it
  was already ISA 7.5.
- Verified on the A2000: affine/ffn/topk `*_oracle_matches_across_cuda_backend`
  all pass — the differential oracle now cross-checks three kernels across wgpu
  and CUDA. p64 still needs a count uniform for CUDA/MSL (no arrayLength).

### 2026-06-28 CUDA backend works; cross-backend oracle (§7/§10)

- First real execution of the native CUDA/PTX backend surfaced two bugs in the
  previously compile-only path:
  1. The affine PTX emitted a duplicated parameter name
     (`.param .align 4 .b8 params[16] params`) — ptxas rejected it with a syntax
     error. Fixed the emitter to build the whole declaration once; also marked the
     entry `.visible` so it loads by name. PTX now assembles to a CUBIN via ptxas.
  2. `cuda.rs` dispatch resolved buffers positionally as [params, input, output]
     but the emitter/oracle layout is input@0/output@1/params@2 — now resolved by
     binding number.
- `evaluate_affine_cuda` runs the affine kernel through cuda.rs against the same
  CPU reference vectors used for wgpu. Verified: `affine_oracle_matches_across_cuda_backend`
  passes on the A2000 with CUDA 13.3 — the differential oracle is now genuinely
  backend-agnostic (wgpu + CUDA), per plan §7. Generic-kernel PTX is the next step.

### 2026-06-28 §6 search pruning: warp alignment + roofline

- Warp/wavefront alignment: `AdapterConstraints.warp_size` (32 NVIDIA, 64 AMD by
  vendor); `ScheduleSpace::candidates` prunes workgroup sizes that aren't a
  multiple of it. Generation is unaffected — only the tuning search narrows.
- Roofline (`roofline.rs`): per-kernel FLOP/byte estimate + memory-vs-compute
  classification. `shader roofline <kernel> [--n]` dumps it; `tune --dry-run` now
  prints the roofline plus a per-workgroup search-tree breakdown (plan §10
  "roofline visualizer / search tree dump"). Verified: affine 0.25 FLOP/byte
  (memory-bound), fused-ffn 33.7 (compute-bound).
- Honest limits (genuine, not stubs): a *device-relative* roofline reject needs
  peak FLOPS/bandwidth, and compute-unit-saturation needs CU count, neither of
  which wgpu exposes — these would require a calibration micro-benchmark. Thermal
  throttling needs a temp sensor (e.g. nvidia-smi) not yet wired. Memory-arch tile
  biasing applies once a tiled matrix kernel exists.

### 2026-06-28 real p64-project + doctor (§12)

- `p64-project` is real: out[r] = sum_w weights[w] * f32(p64[r].word[w]), one
  invocation per record, bound via arrayLength(&output). WGSL + HLSL (GetDimensions);
  MSL guarded honestly (device buffers carry no length). Oracle `p64_project_cpu`
  + `evaluate_p64`. Certified on the A2000 (median ~6.6 µs); HLSL→DXIL via DXC.
- `qualia-cli shader doctor` (plan §12): checks the wgpu adapter, DXC
  (QUALIA_DXC_PATH/PATH), and the CUDA toolkit (CUDA_PATH/nvcc), printing install
  guidance and the graceful-degradation path. On this box: adapter ok, CUDA 13.3
  ok, DXC reported missing-from-PATH (it is — invoked by absolute path elsewhere).

### 2026-06-28 real fused-FFN (was placeholder)

- `fused-ffn` now emits real math instead of placeholder `DotProduct` nodes:
  `out[o] = sum_h w2[o,h] * gelu(sum_i w1[h,i] * input[i])`, one invocation per
  output element, dimensions from a uniform params block (semantic_version bumped
  to 2). Emitted for WGSL, MSL, and HLSL.
- CPU oracle `ffn_cpu` matches the kernel's op order; `ffn_tensors` scales weights
  by 1/sqrt(fan_in) to keep pre-activations O(1). FFN added to
  `BuiltinKernel::has_gpu_oracle` and the `evaluate_builtin` dispatch.
- Verified: Naga validation + CPU oracle unit tests pass; HLSL FFN compiles to
  DXIL via DXC; `qualia-cli shader certify fused-ffn` → CERTIFIED on NVIDIA RTX
  A2000 12GB, median ~810 µs (tolerance 2e-3 for tanh/accumulation differences).
  Forge suite: 27 passed / 0 failed / 3 ignored.

### 2026-06-28 ray-query emission + non-affine certify/tune

- Ray-query WGSL now emits and Naga-validates: `BuiltinKernel::RayProbe` +
  `BufferElement::AccelerationStructure`; `Op::Intrinsic(RayQuery)` lowers to
  `enable wgpu_ray_query;` + `rayQueryInitialize/Proceed/getCommittedIntersection`
  over a `RayDesc`. Validator runs with `Capabilities::RAY_QUERY`. Test:
  `generated_ray_probe_passes_naga_validation`. GPU execution still needs a BVH
  (acceleration structure) — named as the next step, not emitted-as-stub.
- Intrinsic pruning was correctly separated from emission: schedule emission is
  hardware-agnostic, so the RT/coopmat availability check moved out of
  `Schedule::validate` into `AdapterConstraints::supports_kernel`, invoked by the
  execution/certify/tune paths. (Generation of a ray kernel on a non-RT host now
  succeeds; only running it is pruned.)
- `certify`/`tune` generalised beyond affine: `evaluate_builtin` dispatches top-k
  to `evaluate_topk` (which now returns full timing), so the existing CLI
  `certify`/`tune` work for top-k unchanged. Verified on hardware:
  `qualia-cli shader certify topk` → CERTIFIED on NVIDIA RTX A2000 12GB, median
  14,080 ns / p95 14,144 ns (run evidence, not a universal constant).
- Forge suite (non-GPU): 22 passed / 0 failed / 2 ignored. HLSL top-k compiles to
  DXIL via DXC (`dxc -T cs_6_0`).

### 2026-06-28 GPU execution fixed + top-k certified on hardware

- Running the opt-in GPU oracle tests on the RTX A2000 surfaced two real bugs in
  the wgpu execute layer that broke **all** slab dispatches (so the earlier
  "affine passed on A2000" note did not reproduce under the wgpu-29 slab path):
  1. `QualiaSlabAllocator` handed out unaligned bind offsets (e.g. 16396); wgpu
     requires `min_{storage,uniform}_buffer_offset_alignment` (256 here). The
     allocator now aligns every `BufferView` to 256 and floors capacity to a
     multiple of the alignment.
  2. A single slab was bound as both read-only and read-write storage in one
     dispatch, which wgpu forbids (read-write is an exclusive usage). Split into a
     read/uniform slab and a read-write output slab; `BufferView` carries a
     `BindingUsage` that selects the backing buffer.
- Result: `generated_affine_certifies_on_real_gpu` and
  `generated_topk_matches_oracle_on_real_gpu` both pass on the A2000 — top-k is
  now GPU OracleVerified against the CPU reference, not just Naga-validated.
- MSL and HLSL top-k emitters implemented (threadgroup / groupshared + barriers),
  removing the WGSL-only restriction; PTX still defers generic emission.
- Forge suite (non-GPU): 21 passed / 0 failed / 2 ignored.

### 2026-06-28 RT-core awareness evidence

- IR: added `Intrinsic::RayQuery { acceleration_structure, origin, direction,
  t_min, t_max, destination }` and an `IntrinsicClass` ({Subgroup,
  CooperativeMatrix, RayTracing}) so every intrinsic declares the hardware family
  it needs.
- Capability matrix: `HardwareCapabilityMatrix` and `AdapterConstraints` gained
  `supports_rt_cores` (alongside the existing `supports_coopmat`). The wgpu probe
  now populates all three intrinsic flags from the adapter's real feature set —
  `SUBGROUP`, `EXPERIMENTAL_COOPERATIVE_MATRIX`, `EXPERIMENTAL_RAY_QUERY` (wgpu 29).
- The §6 checker: `HardwareCapabilityMatrix::intrinsic_support` returns
  Native / LowerToSharedMemory / Exclude — subgroup ops degrade to the Phase 2
  shared-memory reduction; cooperative-matrix and ray-query are excluded when the
  hardware is absent. `Schedule::validate` now prunes any candidate whose kernel
  requires RT or tensor-core hardware the adapter lacks
  (`KernelSpec::required_intrinsics`).
- Verified: `rt_intrinsic_excluded_without_rt_cores`,
  `coopmat_excluded_but_subgroup_lowers`, and `rt_kernel_pruned_without_rt_cores`
  pass. Forge suite: 20 passed / 0 failed / 2 ignored.
- Honest scope: RT kernels are represented, capability-gated, and pruned, but the
  actual `ray_query` WGSL body is not yet emitted (needs an acceleration-structure
  binding type, an RT-capable adapter, and a test BVH).

## 12. Deployment & Setup Process

To support the native execution backends (PTX, HLSL), the node requires external toolchains (e.g., NVIDIA CUDA Toolkit, DirectXShaderCompiler) that cannot be bundled directly in the Rust binary. 

We will need:
1. **An Installer / Human Helper**: A CLI wizard (e.g., `qualia-cli setup` or `qualia-cli doctor`) to tell the user exactly where to go get the correct packages (CUDA, DXC), and how to install them for their OS.
2. **Setup Checker**: A validation process that verifies the toolchains exist in the expected paths (`CUDA_PATH`, `dxc.exe`) and are correctly wired before the Forge attempts native tuning.
3. **Graceful Degradation**: If the user has not run the setup or lacks the dependencies, the system must gracefully skip native compilation and use the `wgpu` WGSL fallbacks.
