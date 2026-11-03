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

Initial contract checks: 17 qualification unit tests, 10 independent qualification
tests, 15 session unit tests and 9 independent session tests passed in both debug
and optimised builds. Three Node runner tests passed. Both crates passed locked
offline Cargo package testing, Clippy with warnings denied, and WASM compilation
with no default features. WASM compilation is not browser runtime execution.
