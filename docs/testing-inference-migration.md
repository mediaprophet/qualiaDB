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
