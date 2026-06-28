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
