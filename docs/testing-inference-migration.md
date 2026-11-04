# Inference migration qualification

This document records the incremental qualification history. The assembled
migration results below supersede earlier checkpoint status. Etalon owns reusable
inference execution and P64 model processing; QualiaDB retains authenticated
Webizen policy and token/artifact acceptance. Developer execution is explicit.

Run the dependency-free source tests against the intended Etalon checkout:

```powershell
node --test scripts/test-inference-migration.test.mjs
node scripts/test-inference-migration.mjs coordination/worktrees/etalon-migration
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

## Numerical kernel, runtime metadata and streaming writer consumer

This consumer pins four pure Etalon libraries at
`0f31a70166af7ec3d3d0975f17a1866d751561f0`: inference-kernel, inference, model and
resources. All nineteen shared GPU packages retain the qualified 5d3bce83 pin.
The existing qualia-inference-kernel public imports forward to the canonical
Etalon implementation. Budget, paged KV and prefix metadata public types also
forward, while existing core tests and Qualia graph identity derivation remain.
The extracted metadata does not copy physical KV contents or provide rollback.
Independent consumer fixtures check actual type interoperability and graph-derived
prefix keys through the canonical registry, store, table and reference counts.

The Safetensors Verbatim path delegates layout and streaming serialization to
Etalon. Existing owned APIs, report fields, semantic role/manifold construction
and error adapters remain in Qualia. Quantized and vision paths are preserved in
focused modules. The borrowed-plan API revalidates public plans and accepts
caller-owned bounded workspaces. Parser, serde and external sink allocations are
outside that workspace accounting; the historical peak-report field is not a
total-process memory measurement. I/O failure can leave a partial external sink.

Independent preservation checks cover 23 complete original binary outputs and
all five report fields, ten error cases, four partial-I/O cases, 372 role-mapping
rows and new workspace/sink/allocation boundaries. An initial measurement found
eight role-parser allocations; borrowing the ASCII digit prefix removed those
allocations without changing any of the 372 checked mappings. The canonical
kernel has 25 tests in each profile; canonical runtime metadata has 26 in each.
Those library counts are separate from the consuming core suite.

The first full consumer run passed 10,429 tests and failed the global
`push_tensor_search_hits_drains_to_ring` test, with three existing ignores. Its
push-count assertion passed; the first popped tensor index was 0 instead of 3.
Controlled prior-producer injection against the original published library
reproduced that queue ordering, while the original exact test passed in a fresh
process. This demonstrates shared-state susceptibility; it does not identify the
particular producer interleaving in the failed suite.

The test now executes its original public API assertions in a fresh native
process, checking both successful exit and exactly one executed passing test.
Its WASM body and production ring/producer lifecycle are unchanged. The initial
failure and baseline diagnostics remain retained. The repaired full suite passed
**10,430 tests, zero failures and three existing ignores**, in 418.58 seconds.
All 70 frozen source/retired-path identities remained unchanged. CPU-only and
wasm-llm compilation passed in 1m40s and 1m18s respectively. The split vision
module has one harmless unused CRC import warning; WASM also retains the two
existing renderer dead-code warnings. Actual native Vibe integration passed all
three tests in 3.19 seconds on RTX A2000 DX12, driver 32.0.16.1692, with required
GPU execution, exact CPU-oracle readback and invalid/destroyed-handle checks.
The consumer2-core/cpu/wasm/native receipts retain source hashes and commands.
Benchmark-only remote updates were preserved separately from the tested source.

### Mandatory human-centric governance boundary

Normal application inference requires a real Qualia Webizen/Sentinel decision in
the token acceptance loop, before each proposal is accepted or published.
Permission-independent execution belongs only to explicitly selected developer or
debug-test sessions. Turning on tracing, profiling or a debug build does not relax
a governed session. Missing, disconnected or failed controllers must not cause an
implicit transition into developer mode. Numerical/format parity does not qualify
normal-runtime authority or publication behavior.

This consumer leaves the existing Sentinel, Webizen, inference guard, inference
modes and application decode sources unchanged. The separately retained pretrained
CPU comparison is a developer verification harness. The existing browser Rust CPU
entrypoint has no local guard invocation; caller-layer enforcement needs an actual
end-to-end audit and test before that route can be described as a qualified governed
runtime. Existing FastVerify post-turn semantics do not satisfy that requirement;
bypass belongs only to explicit developer/debug/lab execution. The existing native
ring can advance without a decision bound to the current proposal, which is also
a required corrective integration item. Neither a mode name nor a checkpoint
digest establishes controller enforcement or physical numerical-state restoration.
New normal-runtime integration requires real Webizen approval/denial, controller-loss,
rejected-publication and recovery tests.

### Decoder partition preservation checks

The local decoder is now divided into thirteen focused modules for native and
browser execution, preparation, selection, speculative/forced paths, output
publication and finalization. Public entrypoints, numerical operations and
existing mode behavior are preserved. This mechanical partition does not claim
to correct the governance gaps described above.

Actual native, CPU-only, WASM and WASM-with-GPU-runtime Cargo checks pass. Native
retains the existing two renderer dead-code warnings; the partition also exposes
target-specific unused bindings/imports in disabled branches. These warnings
are recorded, rather than hidden under a crate-wide suppression.

A non-test original/migrated caller comparison used the same full SmolLM2 Q8_0
model, prompt, four-token budget and explicit portable/developer qualification
environment. Returned text, provenance, semantic output, streamed bytes/chunk
boundaries and stage records were byte-identical. This fixture did not use the
unit-test layer/vocabulary caps. It does not prove that a particular GPU fast
path ran, or that a current Webizen policy authorized the output.

Separate actual in-crate comparisons exercised both the plain decoder and copies
with observation hooks. The private original body and candidate body were called
explicitly in fresh processes; hooks captured selected token/score, context and
output token state, streaming and final output. The unit-test configuration
retains its two-layer/four-vocabulary-chunk limits, so that evidence is scoped
separately from the non-test comparison. Sampler RNG, DOMINO, provider KV/SSM and
opaque backend state were not observed. There is no timing claim.

The full serial core suite passed **10,470 tests, zero failures, four ignored and
zero filtered**, in 570.11 seconds. All 10,473 original test names were retained;
the fourth ignore was the added private oracle child. This run used the plain
test-only registration with exactly matching candidate source bytes, not an
untouched post-removal binary. All private registration was removed, and all
3,284 frozen sources were checked after restoration. The receipt parser initially
rejected the log because seventeen subprocess summaries preceded the final suite
summary; final anchored counts and the complete inventory independently
confirmed success. No test or production behavior was changed for that repair.

Private oracle copies and absolute test registrations are not shipped. The final
documentation changes remove a stale private-draft comment reference without
altering executable code. Qwen dense, routed/shared MoE, FTW/NVFP4, hybrid
recurrent and streamed Qwen paths require their own family-specific model,
state, provider and caller qualification; this dense fixture does not cover them.

### Qwen and MoE primitive consumers

Five existing numerical modules now forward through `qualia-inference-kernel`
to Etalon: MoE routing, NVFP4 decoding/GEMV, causal convolution/GatedDeltaNet,
Qwen vector operations, and Hyper-Connection arithmetic. Parent and submodule
imports retain the canonical error/configuration types. All thirteen original
tests remain, with four additional consumer tests covering public type identity,
real native error conversions, partial-error writes, output tails and operation
ordering. The separate trained streamed Hyper-Connection operations retain
their learned sigmoid gates and are not replaced by the simpler primitives.

The canonical extraction was qualified with independent scalar vectors,
positive-controlled zero-allocation checks and original-versus-library
comparisons in debug and optimized profiles. The two RMS implementations retain
their distinct floating-point associations, including a fixture whose first
output differs by one bit. Existing edge behavior, including routing's zero-k
panic for nonempty input and partial writes on some errors, is preserved for
separately reviewed repair; these exports do not claim new input validation.

The five pure-library dependencies share Etalon revision
`782a954c9c246cca45ceb1208f86c38528619bf9`. The nineteen GPU-family dependencies
retain their qualified revision. The lockfile update changes only those five
source revisions; locked/offline dependency validation passes. No model-I/O
dependency is introduced by this consumer change.

The actual consuming core binary passed **10,474 tests, zero failures, three
ignored and zero filtered**, in 525.66 seconds with serial execution. Its test
inventory retains all 10,473 original names and adds exactly the four consumer
tests. The 3,285 frozen source/manifest paths remained unchanged through this
run. This is an ordinary candidate binary without the earlier private decoder
oracle registration.

The actual native integration harness also passed all three cases: two CPU
cases and GPU compute/readback on the discrete NVIDIA RTX A2000 through Vulkan.
The GPU case executed ten vector-add dispatches and one sparse initialization,
checked exact reference outputs and rejected invalid requests. CPU-only, WASM
LLM and WASM LLM with GPU-runtime compilation all pass with the same frozen
sources. Existing native renderer warnings and target-specific unused warnings
remain visible; these consumer checks are not a strict Clippy claim. WASM
compilation does not establish browser execution or NPU/Qwen GPU qualification.

These checks qualify numerical ownership and consumer compatibility. Complete
Qwen dense, GGUF/FTW routed and shared experts, hybrid recurrent and streamed
Qwen execution still require retained model identities, full sequence/state
comparison and provider-specific evidence. A present optional model fixture or
an early-returning physical test is not sufficient evidence for those gates.

### Installed evidence policies and retained acceptance

The native policy host now supports explicit V3 evidence-policy installation
and admission through the existing authenticated authority and invalidation
gate. Subjects, selected identifiers, controllers and installation authority
remain distinct. A caller-supplied identity label or copied association evidence
cannot construct an admitted session.

Persistence dispatches explicitly between the existing record-v1/V2-policy
representation and the new record-v2 envelope. Loading does not upgrade a
policy, reinterpret a signature, or invent source/origin permissions. New
evidence admission requires an explicitly installed V3 policy. Existing V2
admission remains available under its original contract.

Tokens and artifacts share one exclusive caller-provided retained-byte slot.
Policy verification uses the complete retained proposal and payload, including
source, score presence/meaning and signed artifact-origin restrictions. A
control digest does not stand in for Webizen VM and graph evaluation. Bulk
payload verification precedes the trusted time used for live rule evaluation;
acceptance then rechecks the complete current association and policy state
before committing the contract record.

Accepted bytes are exposed through a one-use borrowed drain record after
authority locks have been released. The buffer cannot be reused while that
record is held. Dropping the record, unwinding, or acknowledging the wrong
delivery length terminates the session. A later revocation prevents subsequent
acceptance; it cannot retract an already accepted prefix. An acknowledgment is
the embedding caller's report, not independent proof of external delivery.

This is native authority and retained-record integration. Model/source labels
still need a qualified runtime adapter that binds actual retained model and
companion bytes, the prepared provider and the request. It does not yet replace
the local decoder's acceptance loop, implement browser authority transport,
restore numerical state, or qualify complete normal-runtime inference.

Focused qualification passed 115 tests in each of the debug and optimized
profiles, with strict warnings for the focused harness. The independent cases
exercise expiry after real bulk payload work, owner disconnect and policy
replacement/revocation, cross-session ticket refusal, graph-rule expiry,
payload tails beyond 4 KB, artifact event exhaustion and callback reentry.
Positive-controlled allocation measurements cover successful token/artifact
acceptance and denial. The 598-line schema test module is retained as one
cohesive fixture; production modules remain below 500 lines.

An immutable copy of the actual legacy host verifies old-write/new-read and
new-write/old-read compatibility, including byte-exact installed and revoked
V2 records, signatures and anchors at the same canonical store path. One
positive compile probe and eight negative probes check API usability, private
ticket construction, non-copyable records, borrowed-data lifetime, exclusive
storage and the absence of a caller-supplied production clock. The oracle
copies and absolute test registrations are private qualification artifacts.

The actual consuming core suite passed **10,510 tests, zero failures, three
existing ignores and zero filtered**, in 512.63 seconds. All 10,477 prior test
names remain, with exactly 36 new evidence-policy cases. The 3,309 frozen source
and manifest paths remained unchanged through the build and serial run, with
`QUALIA_REQUIRE_GPU_TESTS=1`. These are native core results; the focused strict
warning setting does not imply a warning-free workspace build.

The consuming client policy suite passed all four cases and the CLI policy
suite passed all three. CPU-only compilation, WASM LLM compilation and WASM
LLM with GPU-runtime compilation also passed with the same 3,309 source paths
unchanged. Existing target-specific unused warnings remain visible. The final
reconciliation matches all 72 independently reviewed authority/evidence source
paths, the complete test inventory and the unchanged lockfile. These WASM
checks establish compilation, not browser authority transport or execution.

### Engine state and model-byte access preparation

G3-P2A separates the tensor engine's state definition and centralizes borrowed
model bytes and retained byte handles. Public mapping fields keep their existing
types, and ownership transfer remains available. The 17 existing retained-map
clones remain 17 retained-handle clones; ordinary reads borrow the same bytes.
This mechanical step changes neither backing admission nor numerical algorithms.

Three additional tests cover pointer/ownership identity, replacement and drop
lifetimes, absent mappings and positive-controlled zero-allocation access. The
actual core suite passed **10,513 tests, zero failures, three existing ignores
and zero filtered**, in 548.87 seconds with serial execution and
`QUALIA_REQUIRE_GPU_TESTS=1`. All 10,513 prior test names remain, with exactly
three additions. The 3,313 frozen source/manifest paths remained unchanged.

Native, CPU-only, WASM LLM, WASM LLM with GPU runtime and CUDA compilation pass.
An initial WASM unused parent-alias warning was removed by gating that alias on
native targets; the final WASM checks pass with the existing unrelated warnings.
These compilation results do not establish CUDA or browser execution.

A separately linked non-test caller ran the complete 32-layer SmolLM2 fixture
for four generated tokens. Its retained report is byte-identical to the original
caller report, including token/text output and observed stage records. Hidden
sampler, DOMINO, provider KV/SSM and opaque backend state were not observed by
that report. The candidate additionally inherited the GPU-test requirement flag,
which is used only by renderer test fixtures; the numerical environment values
were equal. No performance or new governance claim follows from this comparison.

The first capture completed inference but failed report promotion because its
artifact parent included compiler/source evidence exceeding the 2 MiB budget.
The failed receipt is retained. A fresh capture used a separate marker-owned
parent under the unchanged budget and passed; production code and reference
expectations were unchanged. Actual retained-model preparation and numerical
commit through Webizen remain subsequent integration gates.

### Assembled migration checks, 2026-11-04

The assembled canonical CPU package run passed 374 distinct tests. Subsequent
independent lifecycle and diagnostics suites passed 12 new Rust tests; their
combined package run also included two previously counted SAB tests. The ignored
diagnostics child entrypoint was explicitly executed by eight parent subprocess
scenarios. The native Qwen feature profile separately passed 73 tests; this count
overlaps the CPU package run and must not be added to it as unique coverage.
The canonical inference library also compiled with its CUDA feature enabled.
Compilation alone does not establish CUDA hardware execution.

Actual Chromium full-WASM CPU validation passed for both shared rings under
cross-origin isolation and nonisolated transferable messages. Each run recorded
14 observations and 12 explicit passing assertions, with no page errors. It used
the local native P64 SmolLM2 model, generated two tokens directly and one through
the production dedicated worker, and exercised explicit developer admission,
governed refusal without authority, model transfer/detachment, release and mode
invalidation. The browser test found that Chromium rejects shared-backed input
to TextDecoder. The transport now copies only the bounded message payload into
an ordinary buffer; the updated Node transport suite passes seven tests.
Run `node scripts/test-qualia-browser-cpu.cjs`, with `NO_SAB=1` for fallback.
Receipts are retained under `target/qualia-browser-runtime` with source, model
and generated-artifact hashes. These CPU checks do not qualify browser GPU use.

The standalone Etalon P64 lab ran the local 239,914,752-byte SmolLM2 fixture in
release mode with Rust 1.96.1: prompt IDs `1,42,17`, context 32, four generated
tokens, one warm repeat, 1 GiB caller buffer budget and 64 MiB artifact budget.
Reversible and direct execution, cold and warm, produced exactly the same IDs
`31,4093,31,4093` and maximum-logit bits. Its 9,580-byte retained evidence is in
the Etalon checkout's `target/p64-lab-qualification-20261104/ablation-release-001`;
no temporary run directory remained. Model SHA-256 is
`7d597fecbbbd8aa89d3f4c4ef82f3f497a08e336efe1ef055b48832ca53a5292`.
This establishes bounded execution/parity, not semantic quality or throughput
qualification. An earlier unoptimized run completed the first configuration
but reached its 300-second deadline during the second; its failed receipt is
retained separately as `ablation-001`.

Both full and portal browser distributions were rebuilt with pinned toolchain
and wasm-bindgen versions. No public exports were removed. The provenance file
is `target/qualia-browser-build-20261104/final-build-provenance.json`; generated
closure-symbol changes are distinguished from public API changes.

The consuming core run completed with **10,156 passed, one failed, three
ignored and zero filtered**, in 817.70 seconds. The physical Qwen NVFP4 fixture
called the normal entrypoint without authenticated authority and received
`[native-governance-required]`, so its existing token-generation assertion
failed. The retained failed run remains evidence. The test caller now explicitly
selects `NativeDecodeRequest::Developer`, preserving the original assertions.
Two missing test imports were also corrected. The final affected suite passed
**38 tests**, zero failures/ignores and 10,124 filtered, in 113.45 seconds;
two restored original renderer byte-accounting tests separately passed. These
scoped checks followed the test corrections and canonical Forge formatting/P64
compatibility repair. There was no subsequent whole-core rerun, so this is not
a claim that the final complete 10,162-test inventory passed in one run.

The final inventory accounts for all 383 removed original full test names:
382 map to executed, passing canonical Etalon tests and one was renamed and
passed within core. The final inventory contains 10,162 names versus 10,516
before extraction, with 29 additions and two restored original names. The
mapping establishes retained executed coverage by name; it does not establish
hardware execution for every mapped test. Two optional P64 real-weight bridge
cases returned early because their fixture search path was absent and are
excluded from model qualification. The physical Qwen unit fixture retains its
layer/vocabulary caps and checks generation/provenance, not full-model numerical
equivalence or text quality. Core receipts, inventory deltas, exact name mapping
and the canonical JUnit identity are retained under
`target/inference-migration-baseline/final-core-repaired-12a674facedb475184a91079c8dda2cd/`;
`final-summary.json` records these counts and limits. The frozen canonical
source check covered 592 files with zero changes.

The canonical Etalon checkout is `coordination/worktrees/etalon-migration`.
Its final main nextest run passed **4,706 tests, zero failures and 49 skipped**
in 820.515 seconds. `target/nextest/default/junit.xml` records 4,706 executed
passing tests; its zero skipped attribute excludes tests omitted before
execution. The 49 selection skips are recorded by `full-xtask.log`, including
36 excluded by the default profile filter. The GPU summary distinguishes
2,034 successful dispatch outcomes from 37 expected failure outcomes
(10 backend-specific and 27 always-failing cases); passing negative tests are
not successful GPU dispatches. Adapter evidence includes NVIDIA RTX A2000 and
Intel HD Graphics 530 native backends and the Microsoft software adapter.

Separate required-hardware CUDA qualification passed four tests and the migrated
DP4A CPU/GPU differential passed one test on the physical NVIDIA RTX A2000 12GB,
compute capability 8.6, PCI `0000:01:00.0`, driver 616.92 and NVCC 13.4.92.
These establish execution, not a performance qualification. The canonical
checkout retains the summary, adapter outcome counts, source/receipt hashes and
CUDA logs under `target/gpu-migration-qualification-20261104/`. These native
results do not qualify browser GPU inference.

A separately linked non-test native caller generated four tokens in explicit
Developer mode. Its report matched the retained original caller report byte
for byte, including text, provenance, stream chunks, token count and observed
stage records. Normal entry without authority separately refused with zero
tokens and no semantic Quin. The receipt is
`target/inference-migration-baseline/real-native-current-569fd21f3d6b434db474e4f0a843584e/receipt.json`;
it identifies the model and binary hashes and its unchanged source snapshot.
Shared Vulkan adapter observation does not identify the actual numerical
provider. Token IDs/logits, sampler RNG, DOMINO, provider KV/SSM and opaque
backend state were not captured. This capture precedes subsequent final shader
text repairs; it does not qualify those changed bytes. No independently
provisioned authenticated authority session, admitted identifier association,
signing credential and installed policy host were supplied for a real governed
positive run, so complete governed model execution remains unqualified.

Actual Chromium WebGPU qualification exposed two failures, both retained and
repaired. Chromium rejected the rounded `-3.4028235e38` shader constant; the
canonical shader now uses the exact finite f32 minimum. The browser projection
adapter then interpreted WebGPU's absent native submission index as failure,
although the command had been queued. The browser-only adapter now proceeds to
the existing asynchronous map/readback completion gate. Native admission guards
are unchanged. Failed receipts remain as
`target/qualia-browser-runtime/browser-webgpu-before-topk-fix.json` and
`browser-webgpu-after-topk-argmax-failure.json` in that directory.

The rebuilt final-pin full WASM distribution passes three actual Chromium GPU
checks: physical NVIDIA adapter compute/readback, exact top-k tie ordering and
negative-score reduction, and one-token native P64 model inference with no CPU
fallback. The model has 290 tensors and 32 layers. The retained successful
receipt is `target/qualia-browser-runtime/browser-webgpu.json`. The production
path uses GPU GEMM followed by CPU streaming argmax. Existing `gpuTop1`/384-byte
receipt metadata describes the planned compact path, not this actual production
path; it must not be used as execution evidence. This one-token explicit
Developer run is not a model-quality, throughput or browser governed-authority
qualification.

The publication dependency graph pins all 19 Etalon workspace dependencies and
compatibility patches to `cf5c195d68ba805065e42c6ac74420245a7d6db6` from
`https://github.com/mediaprophet/etalon.git`. Windows-filtered Cargo metadata
resolves 25 Etalon/Naga/wgpu packages to this single source. All 1,477 package
versions are unchanged from the tested local-path lockfile; gpu-allocator keeps
its Windows 0.62.2 dependency. Locked offline Rust 1.96.1 checks passed for
qualia-core-db, qualia-client-core, qualia-cli and qualia-mcp in 6m28s.
The graph and build receipts are retained under
`target/inference-migration-publication`. Two remote benchmark-result commits
were preserved by fast-forward before the migration commit.

The final shader revision passed the full Etalon suite again: 4,706 passed,
zero failed and 49 existing skips in 1,166.316 seconds. Its independent log and
summary are under the Etalon checkout's
`target/gpu-topk-final-qualification-20261104`. This supersedes the previous
full-suite checkpoint for the changed top-k shader bytes.

The pinned native production library built successfully in 455.031 seconds.
The unchanged original public caller was recompiled against that library and
replayed its four-token SmolLM2 comparison in 10.532 seconds. The complete
observed report is again byte-identical to the original baseline. Normal entry
without authority still refuses with zero tokens and no semantic Quin. This
final capture is under
`target/inference-migration-publication/native-public-4ca6a9b7038140018067fe42cdf0a864`.
Its numerical-provider, hidden-state and positive governed end-to-end limits
are the same as the earlier capture; adapter presence is not execution proof.

Both final browser packages were rebuilt from the published pin after the
browser-only submission repair, using Rust 1.96.1 and wasm-bindgen 0.2.129.
Full WASM SHA-256 is
`64a6b0016d299d9b6c995a1ea68397b9aad2cd5ab1b993334348d2b08f6f7b71`;
Portal WASM SHA-256 is
`c43f55e6fd99848372bf3a1f6f0fda08b48a09293459ba926ce343d5a4631de6`.
The final Full bundle also reran both actual Chromium CPU transport modes:
12 assertions passed in each, zero failures. Earlier receipts are retained
separately. The final annotated GPU receipt SHA-256 is
`9aba05bedf4ed50c1bc7cb44a58b97f14adb9034160e52bb5c4a7240cc3d499c`.
Run `node scripts/test-qualia-browser-webgpu.cjs` for the GPU checks;
the CPU command and `NO_SAB=1` fallback are documented above. This is browser
application coverage, not WebGPU CTS or VibeScript integration coverage.

### Authenticated real-model and legacy browser follow-up

An isolated SoftwareAgent authenticated through the production NativeAuthority
and installed NativePolicyHost APIs. Two independently signed identifiers for
the same subject were selected in separate governed model runs. Both matched
explicit Developer execution for four tokens of text, provenance, semantic
result and all stream bytes/boundaries. An installed deny policy published no
tokens, text, stream or semantic artifact; disconnect refused a new request
binding. The run took 145.54 seconds against a 180-second bound. The retained
model lease verified SHA-256
`c004ac34cce45e03e4452bca472535bc371fcb0d07ae560bba2c57ce4a284b7b`.
See `target/inference-migration-publication/governed-real-20261104` for the signed
test-actor fixture, public caller, source/binary freeze, report and receipt.
This closes the earlier unprovisioned positive test prerequisite. It does not
claim enrollment of a real person, hidden-state equivalence or all model families.

The legacy public WASM LocalLlmAgent now uses the same explicit developer
admission as the async browser exports, before either local GPU execution or
extension-bus offload. Its socket response text is not evidence of governed
token acceptance. Reconfiguration invalidates captured completion generations,
including developer/governed/developer round trips. A missing engine returns a
diagnostic instead of panicking. Failed vocabulary projection no longer falls
back to a readback byte count or hidden-state coordinate as a token. These were
pre-existing legacy defects, not introduced by the extraction.

The browser receipt is now `qualia.browser-execution.v2`. It reports the actual
GPU GEMM/CPU streaming-argmax path with `gpuTop1=false`; total GPU device-to-host
bytes are explicitly `null`/unmeasured. CPU mode reports zero device transfers.
The full-logits byte field is labelled as a tokenizer-vocabulary estimate, not
a measured total. The earlier v1 receipt limitation above is historical.

Both Full and Portal packages rebuilt with the same pinned toolchain. Actual
Chromium CPU shared/transferable and real P64 GPU checks passed again, including
the v2 assertions. A separate Rust-to-WASM consumer linked the production Full
rlib and called the public LocalLlmAgent and execution-mode APIs in Chromium.
It verified governed and revoked-developer refusal with zero tokens/callbacks,
explicit developer/no-engine handling without panic, and stale public-worker
token rejection after a mode round trip. Newly added WASM unit-test source was
not itself executed; these are separate observed consumer checks.

Evidence: `target/qualia-browser-gate-20261104-legacy-v2`, including exact commands,
source stability, both CPU receipts, GPU receipt, external Rust-consumer source
and receipt, and artifact hashes. Full WASM SHA-256 is
`36fd9a83c64936a22f2620ce802f907d14f39808724e77e06c6cbdafd634388a`;
Portal WASM SHA-256 is
`6bb2608581c70cf867abf7e4b7a2ba1e7e9a8f115d93b64db8b5ffc777635ec4`.

The stricter real-model denial gate is now directly observed. A signed installed
Webizen context-input-only policy allowed actual prefill of 13 prompt tokens,
then computed `Deny` for a `SpeculativeToken` proposal. The run published zero
tokens, text, streaming bytes or semantic artifact; disconnect again refused
new binding. It completed in 69.718 seconds within the unchanged 180-second
child bound. The decisive diagnostic is
`webizen-proposal-decision|source=SpeculativeToken|decision=Deny`, emitted only
after the owner/policy closure releases its locks and without token contents.
The subsequent `Revoked` is terminal state following this actual Deny, not
evidence of an unrelated continuity defect. Earlier ambiguous attempts remain
retained and are not retroactively labelled passing.

See `target/inference-migration-publication/governed-token-decision-20261104`:
the receipt records `OBSERVED_ACTUAL_WEBIZEN_SPECULATIVE_TOKEN_DENY`, together
with exact build/fixture/model identities and cleaned temporary directories.
The speculative path still does not emit a `Quarantined` stage; that historical
field remains false, with the actual computed policy decision supplying proof.
The production library rebuilt successfully with locked offline Rust 1.96.1;
native changes in this follow-up add refusal diagnostics only. No policy,
authority, numerical or publication behavior was weakened to obtain the result.
