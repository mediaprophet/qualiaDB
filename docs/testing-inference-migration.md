# Inference migration contract tests

The first migration increment adds independent evidence validation and explicit
execution-session contracts in Etalon. Existing QualiaDB inference callers,
Webizen decisions, accelerator providers and dependency pins remain unchanged.

Run the dependency-free source tests against the intended Etalon checkout:

```powershell
node --test scripts/test-inference-migration.test.mjs
node scripts/test-inference-migration.mjs C:/Projects/etalon
```

The checkout must contain `crates/etalon-qualification` and
`crates/etalon-inference-contract`. A development checkout can be passed instead.
Node.js and Rust with a working native linker are required. No QualiaDB account,
service, model, network access or GPU is needed by this runner.

The runner compiles the actual dependency-free Rust source, unit tests and
external `tests/*.rs` with `rustc`. It executes debug and optimised builds,
rejects empty/ignored/filtered test suites, hashes source packets before and after
execution, bounds child output and deadlines, and cleans its owned temporary
directory on success or failure. Test stdout is captured; the final JSON reports
the suite counts, compiler version and source digests. Retention is caller-owned
by redirecting that JSON to a chosen evidence location. A failed command produces
no success report.

The runner is for these small, reviewed dependency-free test sources. Its process
limits are not a sandbox for hostile code or detached descendant processes.

This is source-level contract testing, not Cargo workspace, package, feature,
GPU, full numerical LLM, Webizen controller or application qualification. It
does not establish that producer-supplied receipts describe real execution.
The evidence validator checks declarations against independently frozen expected
values; independent observations and supervisory review remain necessary.

The initial evidence schema does not bind controller identity, diagnostic
instrumentation or intervention cadence. These require the subsequent runtime
receipt integration; accepting a v1 declaration cannot qualify execution-mode
behavior. Governed declarations must at least carry a valid policy identity.

The developer contract requires no controller. Governed contracts require an
application-provided controller verifier; that verifier is a trust boundary, not
an implementation of Qualia authentication. Test controllers are fixtures.
Instrumentation is independent of mode, session identity is fixed, and controller
failure cannot silently select developer execution. The first contract does not
replace existing mid-decode or post-turn Qualia modes or implement restoration
of model KV, recurrent, sampler or semantic state.

Before connecting production callers, run the real Qualia library and integration
tests against the intended immutable dependency pair, including real graph guard
Allow/Steer/Deny decisions and Webizen VM tests. A fixture controller cannot satisfy
those gates. Runtime rollout additionally requires numerical state restoration,
exactly-once publication, real developer inference and each supported provider's
preservation tests.

The initial baseline attempt on 2026-11-03 used:

```powershell
cargo +1.96.1 test --locked -p qualia-core-db --lib --no-run -j 1
```

It stopped before compilation because Cargo required a lockfile update. A dry
run identified 19 Etalon packages still locked to `ceb79c6`, despite the manifest
already requiring `072b4d9`. The separately reviewed repair aligns only those
19 source and 18 version fields with the existing manifest; dependency arrays
are unchanged. It does not select the migration branch's newer provider code.
The reconciled full library run uses one compiler job, debug information disabled,
incremental compilation disabled and one test thread. Cached historical
executables do not qualify this reconciled source/dependency pair.

The first reconciled build exposed a missing `RadioPermissionContext` import in
the existing radio receipt tests. Restoring that import allowed the suite to run:
10,404 tests passed, one failed and three existing external-data/network tests
were ignored. The failing alpha-mask mip test found actual GPU alpha bytes below
the material cutoff, rather than an overly strict assertion.

Alpha-mask mip generation now explicitly rounds corrected alpha to the byte
selected by the CPU coverage planner before render-target conversion. A spare
component of the existing 16-byte uniform enables this only for alpha masks with
a coverage plan. The cutoff, CPU planner and regression assertions are unchanged.
Ordinary colour, normal and linear-data mip generation retain their existing
behavior. The GPU fix and lock reconciliation are baseline repairs, separate from
moving inference execution into Etalon.

The corrected full baseline passed on 2026-11-03: **10,405 passed, 0 failed,
3 ignored, 0 filtered out** in 452.04 seconds. The exact command was:

```powershell
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_INCREMENTAL = '0'
$env:QUALIA_REQUIRE_GPU_TESTS = '1'
cargo +1.96.1 test --locked -p qualia-core-db --lib -j 1 -- --test-threads=1
```

The GPU-required setting strengthens the earlier baseline run's availability
check; features, dependency revision and build profile are otherwise unchanged.
The three existing ignored cases require a 440 MiB Monarch shard, a local 8+ GiB
Monarch volume set, or a real localhost federation service. They were not newly
ignored by this repair. The build retains one unrelated dead-code warning for
`source_motion_mesh_counts`.

Independent production-source readbacks on NVIDIA RTX A2000 12GB, DX12 driver
`32.0.16.1692` and OpenGL driver `616.92`, checked 22 alpha-mask mip rows per
backend, including odd dimensions and cutoff/opacity extremes. All matched CPU
alpha bytes and coverage. All 28 checked RGB rows and six non-mask RGBA rows per
backend matched the pre-fix outputs. The original failing fixture now stores
alpha byte 205 throughout its six-level mip chain. The rebuilt original test also
passed separately with DX12 and GPU availability required.

Local bounded logs and frozen input hashes are retained under
`target/inference-migration-baseline/alpha-fixed-f092a8d5f0f646e6aadd47cbf147c256`;
the independent GPU matrix is under `alpha-probe-20261103-01` in the same parent.
These application checks do not qualify an untested accelerator, browser runtime,
full new inference mode integration or retired WebGPU CTS coverage.

Initial contract checks: 17 qualification unit tests, 10 independent qualification
tests, 15 session unit tests and 9 independent session tests passed in both debug
and optimised builds. Three Node runner tests passed. Both crates passed locked
offline Cargo package testing, Clippy with warnings denied, and WASM compilation
with no default features. WASM compilation is not browser runtime execution.

## Candidate provider and Forge compatibility qualification

The next provider candidate is immutable Etalon
`5d3bce83ab57630c6dd77c7a0585fce789a15b6b`. All eleven manifest references and
nineteen lock source entries move together; package versions and dependency
arrays remain unchanged. The original `072b4d9` baseline remains reproducible.

The first build exposed two new Etalon IQ-power enum variants missing from the
public reverse conversion. Qualia now represents and emits those kernels through
Etalon, validates their complete canonical specifications and rejects unsupported
local oracle evaluation explicitly. The original nine builtin IDs, discriminants,
serialization and complete specification hashes remain unchanged. Builtin
construction moved into focused modules; existing backend generator behavior is
retained. CLI TuneAll already skips kernels without a local GPU oracle.

A full candidate run initially passed 10,412 tests and failed one new independent
fixture: the fixture expected TopK serialization `topk`, while immutable old
source proves it is `top-k` (its display name remains `topk`). Correcting only that
fixture produced **10,413 passed, zero failed, three existing ignored tests** in
537.58 seconds with the same full-core command/profile above. All eight new IQ
fixtures pass. Frozen source/manifest/lock hashes were checked independently.

Additional candidate gates completed:

- The actual Qualia/Vibe integration target passed all three tests on NVIDIA RTX
  A2000 DX12 driver 32.0.16.1692: ten vector dispatches, one sparse dispatch, exact
  numerical readback, graph parity and invalid/destroyed-handle rejection.
- The original CPU-only package check passed after a narrow existing build-target
  defect was repaired. The benchmark worker calls a GPU-gated function; its
  explicit binary declaration now requires `gpu-runtime`. Its default GPU build
  also passes, so the worker remains available. No library behavior changed.
- `wasm-llm` compilation for `wasm32-unknown-unknown` passed. This is compilation,
  not browser LLM execution.
- The clean Etalon candidate's full `cargo xtask test` passed 3,948 tests with 46
  nextest skips. Its main GPU harness included 1,525 physical-backend Executed
  rows and 351 software-renderer Executed rows; expected failures, internal skips
  and unsupported capabilities are recorded separately.
- Nine explicitly selected ignored Etalon tests also passed with CUDA 13.4 on the
  RTX A2000 (compute 8.6, PCI 0000:01:00.0): CUDA resources, IQ, supplied dispatch,
  resident tensor kernels, fused FFN, NVCC positive/negative compilation and
  prepared Vulkan IQ. This does not qualify Qualia's separate captured Q8 lane.

The test-only [provider CI run](https://github.com/mediaprophet/etalon/actions/runs/41621515244)
passed all 40 jobs at `b9e9a1b8`, whose provider sources match `5d3bce83`.
Windows passed 2,239 tests with 22 skipped on software DX12/OpenGL adapters; Linux
passed 2,178 tests with 22 skipped on llvmpipe Vulkan/OpenGL; macOS passed 1,741 tests with 16 skipped on
Apple's hosted paravirtual Metal adapter. The WASM job passed 438 tests with 1,305 skipped,
with its executed GPU subset reporting GL. These results do not establish
bare-metal Apple, physical Linux GPU, browser WebGPU or NPU qualification.

Local receipts are retained under `target/inference-migration-baseline/`, with
separate candidate, native, CPU, worker, WASM, CUDA and CI directories. The
optional Qualia CUDA library feature check also passed in 7m03s. This establishes
compilation coverage, not captured-decode execution.
Actual resource/model consumer activation, real Webizen-controlled inference,
numerical rollback and standalone developer LLM execution remain later gates.

Reproduce the consuming candidate checks with Rust 1.96.1 and the same disabled
debug-info/incremental profile used above:

```text
cargo +1.96.1 test --locked -p qualia-core-db --test etalon_vibe_harness -j 1 -- --test-threads=1 --nocapture
cargo +1.96.1 check --locked -p qualia-core-db --no-default-features -j 1
cargo +1.96.1 build --locked -p qualia-core-db --bin qualia-device-benchmark-worker -j 1
cargo +1.96.1 check --locked -p qualia-core-db --no-default-features --features wasm-llm --target wasm32-unknown-unknown -j 1
cargo +1.96.1 check --locked -p qualia-core-db --features cuda --lib -j 1
```

The native harness used `QUALIA_WGPU_BACKEND=dx12` and a caller-selected
`QUALIA_ETALON_HARNESS_EVENTS` file. The explicit Etalon test command, from its
clean candidate checkout with CUDA 13.4 and the native compiler configured, was:

```text
cargo +1.96.1 test --locked -p etalon-cuda -p etalon-tensor -p etalon-wgsl-forge -p etalon-sense-gpu --all-features --lib -j 1 -- --ignored --test-threads=1 --nocapture
```


## First live resource and P64 consumer migration

The consumer packet delegates classified byte reservation mechanics and residency
metadata to `etalon-resources`, and P64 layout/structural/CRC reading to
`etalon-model`, both pinned at `f4523adec4716e2cb32db2babc6a9562476947bd`.
All nineteen shared GPU-provider packages remain at the qualified `5d3bce83`
revision. Cargo graph inspection confirms only these two dependency-free packages
at the new pure-library revision; no second GPU type universe is introduced.

Qualia retains six resource-class identities, universe/viewport budget policy,
error fields and mode refresh behavior. Reservation drop releases the Etalon guard
before refreshing mode. The public residency types forward to Etalon while their
existing tests remain in Qualia. This is accounting, not physical resource retirement.

P64 retains owned manifest fields, Clone, public Q42 aliases, environment-selected
integrity defaults, semantic role/manifold interpretation and GGUF synthesis. The
adapter maps typed structural errors to the legacy strings. Weight bytes remain
caller-owned; the cold owned manifest still allocates. No activation speed or
total-memory reduction is claimed by this change.

The actual full core suite passed **10,424 tests, zero failures and the same three
existing ignores**, in 454.41 seconds, with GPU tests required. All eleven new
consumer fixtures passed, including real compiler roundtrip, exact malformed-input
errors, integrity policy in thirteen isolated subprocess cases, owned Clone/aliases,
unknown semantic roles, six-class accounting, unwind and release/mode behavior.
Nested subprocess summaries are not added to the outer suite count. Source hashes
were unchanged throughout the run. The CPU-only package check also passed.

The wasm-llm check passed in 1m35s, retaining the same two existing dead-code
warnings. Actual native Vibe integration passed three tests on RTX A2000 DX12
driver 32.0.16.1692 in 3.45s: ten vector dispatches, one sparse dispatch, exact
CPU-oracle readback and invalid/destroyed-handle rejection. The source hashes
remained unchanged in all four runs. Receipts are retained in the consumer-core,
consumer-cpu, consumer-wasm and consumer-native directories under the local
migration evidence parent. The core receipt also contains the resolved 21-package
Etalon graph inventory.
Real Webizen-controlled inference, standalone developer model execution and
physical numerical-state rollback remain separate migration gates.
