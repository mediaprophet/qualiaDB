# Plan: upgrade wgpu 29 → 30, adopt new + experimental features, light up the tensor-core path

## Implementation record — 2026-07-13

Status: **implemented and build/test-verified on native + WASM**. Cooperative matrices remain
runtime-oracle-gated per backend; this is a measured driver capability, not an incomplete path.

Completed outcomes:

- All workspace wgpu/naga pins used by `qualia-core-db`, `qualia-extensions`,
  `webizen-render`, and `webizen-runtime` resolve to 30.0.0.
- Migrated the v30 API surface, including `PollType`, fallible mapped ranges,
  `RequestAdapterOptions::apply_limit_buckets`, optional vertex-buffer slots,
  `SurfaceConfiguration::color_space`, and `Queue::present`.
- The shared native inference device and browser inference device now intersect desired
  acceleration features with the selected adapter before requesting them. Stable useful
  capabilities are timestamp queries, in-pass timestamps, pipeline statistics/cache,
  `SHADER_F16`, `SUBGROUP`, and `SUBGROUP_BARRIER`.
- `EXPERIMENTAL_COOPERATIVE_MATRIX` remains explicit opt-in through
  `QUALIA_WGPU_EXPERIMENTAL_FEATURES=1`. wgpu 30's unsafe `ExperimentalFeatures` token is
  supplied only when an experimental feature was both opted into and advertised. Baseline-only
  rendering, LoRA, diffusion, test, and utility kernels deliberately keep empty feature sets.
- v30's new `SHADER_I16` is not requested: no current Qualia shader declares
  `enable wgpu_int16`, so enabling it would add no capability. `STRICT_WEBGPU_COMPLIANCE` is
  intentionally unset on native inference because it would hide useful native capabilities.
- The f32 selector no longer silently falls into lossy CUDA WMMA. `gemm_f32_tc` selects the
  measured f32 coopmat path or the exact f32 floor; `gemm_f32_tc_reduced` is the explicit
  reduced-precision WMMA entry point.
- The renderer's projector pipeline now has a real fragment stage; this was exposed by v30
  runtime validation and fixed rather than suppressed.
- Cross-backend capability benchmarking now uses a dedicated
  `qualia-device-benchmark-worker`: the parent enumerates stable adapter identities, then each
  `(backend, vendor, device)` owns its wgpu instance/device in a separate process. Responses are
  length-delimited CBOR, identity-checked, finite-metric-checked, and aggregated deterministically.
  This fixes the Windows access violation caused by successively owning Vulkan and DX12 devices
  in one long-lived process. Workers have a hard 120-second deadline. Production discovery
  supports the sibling worker executable, private worker entry points in the shipped Qualia CLI
  and Webizen Desktop hosts, and the `QUALIA_DEVICE_BENCHMARK_WORKER` override for other
  embedders; unit tests exercise the same protocol through an isolated test worker.

Measured verification:

- `cargo check -p qualia-core-db` — pass.
- `cargo check -p qualia-extensions` — pass.
- `cargo check -p webizen-render` — pass.
- `cargo check -p webizen-runtime` — pass.
- `cargo check -p qualia-core-db --target wasm32-unknown-unknown --no-default-features
  --features wasm-llm` — pass (pre-existing target-specific warnings remain).
- `coopmat_usable_respects_caps_and_is_cached` — pass.
- `stage4_forge_gemm_f32_and_tc_selector` — pass; `max_err=0.00e0` for 32×64×32.
- `webizen-render --lib` — 48 passed.
- `webizen-runtime --lib` — 2 passed.
- `gpu_context::tests` — 7 passed, 1 hardware-report test ignored by design.
- Device benchmark worker binary — `cargo check -p qualia-core-db --bin
  qualia-device-benchmark-worker` passes.
- Worker protocol round-trip and isolated multi-adapter capability matrix — pass; measured A2000
  Vulkan + DX12 and Intel HD 530 DX12 without shared driver lifetimes.
- Full `cargo test -p qualia-core-db --lib` — **5,365 passed, 0 failed, 9 ignored** in both
  single-threaded and default-parallel execution. The former blanket hardware ignores were
  replaced with capability-aware execution and a serialized GPU lane, confirming both worker
  isolation and parallel driver/context safety.
- Computational geometry library suites — **1,517 passed, 0 failed, 0 ignored**.
- Real SmolLM2-360M P64 layer-0 Forge decode — pass against the CPU oracle
  (`max_rel=3.28e-6`). The reader now enforces the compiler's layer-pack contract: page
  alignment at layer boundaries and 256-byte alignment within a layer.

The 9 remaining ignored tests are deliberate non-default operations, not missing implementations:
two ray-query tests and one cooperative-matrix load/store test requiring explicitly opted-in
experimental hardware features; the known DX12 cooperative-matrix multiply driver failure; one
GPU performance benchmark; one SmolLM2 full-model performance run; one DXC CLI integration; one
expensive production-parameter BFV smoke test; and one manual adapter-report diagnostic.

Coopmat result: wgpu 30 includes #9741's Vulkan memory-model fix, but the A2000 DX12 backend
still returns zero for the probe. `coopmat_usable()` therefore remains false on that backend and
correctly self-gates; the complete tiled implementation will activate automatically on a backend
that passes the oracle.

**Owner:** Claude (Timothy directed, 2026-07-13). Branch `0.0.24`.
**Status at plan time:** HEAD `6fe51db6`. This session already fixed the inference_agent
stack overflow + 3 masked bugs and library-ized every monolith; the *whole* `qualia-core-db`
lib test suite now runs to completion (5273 pass) with **one** remaining failure —
`inference::toolkit_probe::tests::stage4_forge_gemm_f32_and_tc_selector` — which is a direct
consequence of the wgpu 29.0.3 coopmat bug this upgrade targets.

Follow CLAUDE.md §13: don't just make it compile — **adopt the new version's better
capabilities**. Follow the "fix completely, no stubs" rule (memory
`feedback-fix-completely-no-stubs`).

---

## 0. Why now / the payoff

- wgpu **30.0.0** is released (docs.rs/wgpu/latest). The forge tensor-core path is gated on
  wgpu carrying the **coopmat multiply fix (upstream #9741)** — on 29.0.3 the coopmat matmul
  returns zeros, so `wgsl_forge::dispatch::coopmat_usable()` (a real 8×8 all-ones probe that
  checks the result == 8.0) returns `false` and the portable WGSL tensor-core GEMM stays
  dormant, falling back to the CPU floor. If 30 carries the fix, `coopmat_usable()`
  **self-activates** and the portable f32 tensor-core GEMM becomes real.
- Timothy also wants the **new stable features** wgpu 30 exposes AND its **experimental
  features** (which in wgpu 30 require express opt-in — `InstanceFlags`/`Features` bits and/or
  a Cargo feature) wired up so the project can use them.

## 1. Current state (exact)

Version pins to bump (all `= "29"` → `= "30"`, matching naga):
- `crates/qualia-core-db/Cargo.toml:147` `wgpu = { version = "29", features = ["webgpu"], optional = true }`
- `crates/qualia-core-db/Cargo.toml:148` `naga = { version = "29", features = ["wgsl-in","spv-out"], optional = true }`
- `crates/qualia-core-db/Cargo.toml:250` `naga = { version = "29", ... }` (dev-dep)
- `crates/qualia-extensions/Cargo.toml:24` `wgpu = { version = "29", optional = true }`
- `crates/webizen-render/Cargo.toml:9` `wgpu = "29"`
- `crates/webizen-runtime/Cargo.toml:10` `wgpu = "30"` ← bump
- `cudarc = "0.19"` (`crates/qualia-core-db/Cargo.toml:150`) — leave unless it conflicts; the
  CUDA WMMA tier is independent of wgpu.
- Cargo features: `wgsl-forge = ["dep:naga","dxc","cuda"]`; `cuda = ["dep:cudarc"]`; `dxc = []`
  (DX12 via DXC — confirm the `dxc`/`Dx12Compiler` selection still exists in wgpu 30, see §3).

Docs to update at the end: `docs/WGPU_UPSTREAM_TRACKING.md` (exists), and (if present)
`DEPENDENCY_MODERNIZATION.md`, and the memory `wgsl-forge-gpu-backend-status.md`
(currently says "coopmat fix #9741 merged upstream (unreleased) → soft-fork test-patch path" —
update once 30 carries it).

Key GPU surface (where wgpu API breaks will land):
- `crates/qualia-core-db/src/gguf_bridge/init.rs` — device/adapter request, all pipeline &
  bind-group-layout creation (`layout: None` auto layouts for "Fused Transformer Pipeline" and
  "Mock Fused Contraction Pipeline"; explicit layouts for coop_gemv; `native_pipeline_cache`).
- `crates/qualia-core-db/src/gguf_bridge/*.rs` — every dispatch site (embedding/attention/ffn/
  gemm/output/prefill_*/resident_decode/async_dispatch/verify_arena/…). These call
  `get_bind_group_layout`, `create_bind_group`, `begin_compute_pass`, `dispatch_workgroups`,
  `poll`/`Maintain`, buffer mapping (`map_async`), `write_buffer`, `submit`.
- `crates/qualia-core-db/src/wgsl_forge/` — `dispatch.rs` (`coopmat_usable`, `gemm_f32_tc`,
  `gemm_f32_tc_coopmat`, CUDA WMMA tier), `execute.rs`, `emit/coopmat.rs` (**the WGSL coopmat
  code generator** — most likely to need syntax changes for wgpu-30 naga), `oracle.rs`.
- `crates/qualia-core-db/src/shaders/*.wgsl` — `fused_transformer.wgsl`, `coop_gemv_subgroup.wgsl`,
  `dual_gemv.wgsl`, `fused_ffn.wgsl`, and the coopmat/subgroup-matrix WGSL. wgpu-30 naga may
  change subgroup/`enable` directives or the coopmat matrix WGSL surface.
- CLAUDE.md §13 flags pre-existing 29-era debt: some code still on the ~0.20 surface
  (`wgpu::Maintain` → `PollType`). Sweep for any remaining `Maintain`, deprecated enum variants,
  `Features`/`Limits` field renames.

## 2. First step for the executor (do this before touching code)

Read the wgpu 30 migration guide + CHANGELOG so the breaking-change list is exact, not guessed:
- WebFetch `https://docs.rs/wgpu/30.0.0/wgpu/` (types/enums that changed).
- WebFetch the wgpu repo `CHANGELOG.md` section for `v30.0.0` (breaking changes + new features +
  which experimental features gained express flags).
- Note especially: `PollType`/`poll` signature, `InstanceDescriptor`/`InstanceFlags`,
  `Features` bit renames/additions (subgroup matrix / coopmat, ray-query, f16, push/immediate
  data), `RequestAdapterOptions`, `DeviceDescriptor` (`required_features`/`required_limits`,
  `memory_hints`, new `experimental_features`-style gate if present), `PipelineCompilationOptions`,
  `Dx12Compiler`/DXC selection, `ShaderModuleDescriptor`/naga source, bind-group-layout APIs.

## 3. Migration (phase A — compile clean on 30)

1. Bump all pins in §1 to `"30"`; `cargo update -p wgpu -p naga`.
2. `cargo build -p qualia-core-db` (default features) and iterate on breaking changes across the
   files in §1. Fix to the **new** API surface (don't shim). Likely hotspots:
   - `device.poll(...)` / any residual `wgpu::Maintain` → wgpu-30 `PollType`.
   - `Features`/`Limits` field/variant renames.
   - `DeviceDescriptor` field set (memory hints / trace / experimental gate).
   - naga WGSL front-end changes affecting `emit/coopmat.rs` output and the `.wgsl` shaders
     (subgroup/coopmat directives, `enable` blocks).
   - `Dx12Compiler` / DXC path (the `dxc` feature) — confirm the selection API and keep the
     DX12=DXC decision (memory `forge-produces-engine-runs`: DX12 works via DXC, byte-identical
     to Vulkan; do NOT regress to FXC).
3. Also build `qualia-extensions`, `webizen-render`, `webizen-runtime` (all pin wgpu) and the
   wasm target (`--no-default-features --features wasm-llm` or the project's wasm feature) —
   the `webgpu` feature path in init.rs must still compile.

## 4. Light up the tensor-core path (phase B — the payoff)

1. After it compiles, run the coopmat probe + the currently-`#[ignore]`d reference test:
   - `wgsl_forge::dispatch::tests::coopmat_usable_respects_caps_and_is_cached`
   - un-`#[ignore]` `wgsl_forge::dispatch::tests::gemm_f32_tc_coopmat_matches_cpu_reference`
     (it's ignored *only* because of #9741) and run it. If wgpu 30 carries the fix it now passes
     (real 8×8 tile tensor-core GEMM == CPU reference); keep it enabled. If it still fails,
     `coopmat_usable()` will (correctly) stay `false` — the probe self-gates — and you leave the
     `#[ignore]` with an updated note. Either way the code is correct.
2. Resolve `stage4_forge_gemm_f32_and_tc_selector` (the one remaining suite failure,
   `max_err=1.22`). Diagnose *which tier* fired (see `gemm_f32_tc` in `dispatch.rs:517`):
   Tier 1 = WGSL coopmat (gated on `coopmat_usable()`), Tier 2 = **CUDA WMMA (f16-input, the
   `cuda` feature is ON by default via `wgsl-forge`)**, floor = `gemm_f32`.
   - `max_err=1.22 ≈ max(|plain|)` is the signature of the coopmat path returning **zeros** — i.e.
     it fired despite being broken. On wgpu 30 with the fix, coopmat returns correct f32 → this
     passes. **Verify this is the actual cause** by logging the chosen tier.
   - If instead Tier 2 CUDA WMMA (f16) is producing the 1.22 (f16 precision on f32 inputs), the
     **selector** is wrong for an f32-correctness caller: `gemm_f32_tc` should prefer the exact
     f32 coopmat tile (once usable) or the f32 floor over the lossy f16 WMMA tier when the caller
     wants f32 accuracy — OR the probe test's tolerance is wrong for a genuine f16 path. Decide
     honestly: fix the selector so `gemm_f32_tc` returns f32-accurate results (coopmat-f32 or
     floor), and reserve the f16 WMMA path for callers that opt into reduced precision. Do NOT
     just widen the test tolerance to hide a real precision mismatch.
   - Whichever it is, the acceptance test is: `stage4_forge_gemm_f32_and_tc_selector` passes with
     `max_err < 1e-2` because a **correct** path was chosen (not because the assertion was loosened).

## 5. Adopt new + experimental features (phase C — Timothy's explicit ask)

wgpu 30 gates several capabilities behind express opt-in (a `Features` bit and/or an
`InstanceFlags`/experimental toggle). From the CHANGELOG (§2), enumerate what 30 newly offers
and wire the ones this project wants:
- **Subgroup / cooperative-matrix (tensor core) features** — the portable coopmat path; ensure
  the required `Features` bit(s) are requested in `init.rs`'s `DeviceDescriptor.required_features`
  when the adapter advertises them, and that `caps()` (in `wgsl_forge`) reflects them.
- **Ray query** (memory says ray-query already works) — keep enabled; confirm the 30 flag name.
- **f16 in shaders** (`SHADER_F16`) — relevant to the f16 tensor-core / Q4_K paths.
- Any **push-constant / immediate-data**, **pipeline cache**, **timestamp/pipeline-statistics
  query**, or **experimental** extensions 30 adds that benefit decode throughput.
- For genuinely *experimental* features that 30 puts behind an instance/experimental flag, add
  the express enable (and, if the project should be able to toggle it, a Cargo feature or a
  runtime setting — respect the "software provides MEANS" principle: make it opt-in, not forced).
- ⚑ **Timothy decision:** which experimental features to switch ON by default vs. leave behind a
  flag. List them once the CHANGELOG is read; default to *capability-present → enable* for the
  tensor-core/subgroup/f16 set (they only help), and gate anything with stability/portability
  caveats behind an explicit feature.

## 6. Verification (must all hold before done)

- `cargo build` clean on: `qualia-core-db` (default), `qualia-extensions`, `webizen-render`,
  `webizen-runtime`, and the wasm target. **Zero warnings** on touched files.
- `cargo test -p qualia-core-db --lib` runs to completion and is **green** (this session made it
  runnable; the only known failure, `stage4_forge_gemm...`, must now pass per §4).
- The un-ignored coopmat reference test passes (or self-gates correctly with an honest note).
- **No inference regression:** the real decode path still produces correct output at the prior
  ~18.8 tok/s on Vulkan, and DX12 stays byte-identical via DXC (don't regress to FXC). Spot-check
  with the toolkit probe / a real `.p64` if a model is available.
- Manual/docs updated: `docs/WGPU_UPSTREAM_TRACKING.md` (mark #9741 resolved-in-30),
  `docs/manuals/qualia_db_functionality_manual.md` if any capability status changes,
  memory `wgsl-forge-gpu-backend-status.md`.

## 7. Sequencing

A. Bump pins + compile-clean on 30 (all crates + wasm). Commit `chore(deps): wgpu/naga 29→30`.
B. Light up coopmat: probe, un-ignore reference test, fix the `gemm_f32_tc` tier selection so
   `stage4_forge_gemm` passes with a correct path. Commit `feat(forge): real coopmat tensor-core
   GEMM on wgpu 30 (#9741 resolved)`.
C. Adopt new/experimental features + express flags. Commit `feat(gpu): adopt wgpu 30 features`.
D. Update tracking docs + memory. Each code change carries its same-commit manual/status update.

## 8. Watch-outs

- The `layout: None` (exclusive auto-layout) pipelines in `init.rs` are a known trap (this
  session hit it in `embedding.rs`); if wgpu 30 changes auto-layout behavior, re-verify bind
  groups match their dispatch pipeline everywhere.
- `cudarc 0.19` / CUDA WMMA tier is orthogonal to wgpu but shares the `wgsl-forge` feature — keep
  the `catch_unwind` around `gemm_tc_cuda` (missing NVRTC/DLLs panic, not Err).
- Respect other instruments' lanes (§10) — check `coordination/NOTICES.md` before starting; the
  GPU/decode area has historically been contested. Announce a CLAIM.
- Don't loosen a test to make it pass (rule): fix the path, or self-gate honestly.
