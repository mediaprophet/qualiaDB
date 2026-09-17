# Validation matrix and completion evidence

**Status:** Required future implementation checks, not recorded passes.
Use the [task register](./task-registry.json) and [evidence template](./templates/evidence-result.md).
Domain checklists supply precise behaviors; this document supplies the shared acceptance standard.

## 1. Definition of done

A package is complete only when its child checks have evidence, dependencies are accepted,
owned implementation and public outputs match the reviewed contracts, and an independent reviewer
accepts the integrated result. Tests must observe meaningful behavior, not mirror implementation
expressions. No required check can be replaced with a compilation-only result, empty feature,
mock of the boundary under test or a comment saying the property holds.

Every result identifies the exact source state, command/harness, feature set, target, profile,
test workload, expected property, observed outcome and artifact location. A dirty workspace
requires a reproducible diff/file fingerprint as well as HEAD. A result from another revision
does not automatically certify the integrated code. Keep retained artifacts scoped, redacted as
needed and byte-budgeted; a digest is not a substitute for retaining required evidence bytes.

Conditional checks may be checked only with an explicit reviewed applicability decision and
its evidence. Record “not selected” or “unsupported” separately from “passed.” Required failures,
unresolved reviewers' findings and unmeasured required targets keep a package open. Existing
unrelated baseline failures are recorded distinctly; prove that targeted validation still
covers the changed boundary before proposing acceptance.

## 2. Acceptance gates

| Gate | Owning packages | Required observations |
|---|---|---|
| G-SEM — meaning and authority | FND-02–03, SEM-01 | Pinned interpretation; entity/claim/handle/instrument separation; purpose/relationship/consent; uncertainty and expiry; no authority from aliases or similarity |
| G-CORE — core, format and recovery | CORE-01–04 | Exact ABI/parity; caller-owned Webizen; scoped cache collisions/eviction; bounded range work; atomic records/effects; crash recovery; QNF decision with evidence |
| G-CRYPTO — actual security profile | CRY-01–02 | Independent vectors; key ownership/zeroization; transcript/domain binding; both hybrid proof components; downgrade/replay rejection; profile limits |
| G-NATIVE — independent networking | NET-01–05, RT-03 | Actual non-IP bearer; libp2p absent; discovery, routing, resolution, policy and session cooperation; loss, churn, withdrawal and application delivery |
| G-RESOURCE — admitted execution | RT-01–02, CORE-01–02 | Lease conservation; construction/error-path allocations; full pass/cell/host accounting; saturation control progress; bounded work; exceptions isolated |
| G-SERVICE — useful services | SVC-01–03 | Resumable sync; authorized projections; monotonic durable receipts; cancellation; custody and compute recovery; no duplicate effects |
| G-ECON — funded roles and commons | ECO-01–02 | Typed units; measured/estimated/unknown; atomic aggregate caps; funding/withdrawal; dispute/finality; gifts and community operation without wallets |
| G-COMPENSATION — finite humanitarian commons | SEM-01, ECO-01–02, OPS-02 | Per-operation personal/corporate/humanitarian classes; shared target cap; positive recovery allocation; last payment, non-cash discharge and terminal fulfilment; no hidden personal debt |
| G-PROTECTION — vulnerable-person safety | SEM-01, NET-04–05, EVD-01–02, OPS-02, QA-02 | Private child/PEP discovery; independent contact consent; freshness-gated delivery; abusive-guardian help/recovery; non-enumeration; protected evidence disclosure |
| G-EVIDENCE — preservation and examination | EVD-01–02 | Hold/GC races; original bytes and semantic closure; provenance/custody; scoped disclosure; remote acknowledgements; offline verification and renewal |
| G-INTEROP — selected external profiles | OPS-01–02 | Gateway isolation; browser target behavior; declared carrier dependencies; application migration; downgrade/rollback compatibility |
| G-SCALE — enterprise qualification | QA-01–02 | Deterministic multi-cell work; bounded IPC and aggregate residency; large datasets; independent oracles; latency/throughput/resource distributions |
| G-RELEASE — supported claims | REL-01 | Accepted selected scope; dependency closure; tested recovery; operator documentation; precise limitations and independent sign-off |

## 3. Test lanes

**Pure contracts and codecs:** independent expected values, maximum boundaries, integer overflow,
noncanonical encodings, unsupported versions, unknown fields under the selected policy, malformed
lengths and partial input. Fuzzing has fixed resource/time budgets and preserves minimized
reproducers. Cross-implementation vectors must not be generated and checked solely by the same
encoder. Proof tests include substitution of context, role, target, operation, profile and key.

**State machines and adversarial schedules:** deterministic clocks and bounded fault schedulers
exercise duplicate/reordered/lost events, stale generations, queue exhaustion, time uncertainty,
expiry, revocation, cancellation and restart. Check liveness under admitted assumptions as well
as safety. A finite fuzz run is evidence of exercised cases, not proof of every schedule.

**Memory and allocation:** use the real thread-local zero-allocation counter on all Tier-1
construction, success, rejection and cancellation paths. No global serial-test requirement is
implied by that counter. Separate logical reservations, allocated/committed bytes, resident
memory, mapped virtual address space, shared pages, DMA/GPU and OS/socket buffers. Report what
each platform can observe and what the governor can actually enforce. Big fixed stack arrays
are not free; charge thread stacks and crypto scratch.

**Persistence and effects:** inject failure before/after every publication, flush and durable
commit boundary; distinguish process crashes from tested power-loss guarantees. Verify recovery
across graph state, exact artifacts, receipts, reservations, holds and external intent. An
acknowledged payment request is not payment finality. Remote preservation is pending until the
required acknowledged state exists.

**Semantics and privacy:** test positive, negative and uncertain decisions against pinned examples,
including revoked delegated roles, co-attestations with common controllers, disclosure narrowing,
sensitive identifiers, semantic bundle substitution and sameAs misuse. Test metering and
preservation without introducing universal person correlation or automated legal conclusions.

**Scale and useful work:** exercise 1, 2, 4 and more cells only where hardware can admit them.
Keep equivalent workloads and seeds, then vary routing churn, packet sizes, graph selectivity,
dataset size, crypto handshakes and preservation backlog. Include stored graphs far larger
than RAM; test worst-case scan work and global-query limitations, not just small result pages.
Measure useful service output separately from work consumed and separately from monetary price.

## 4. Platform and feature matrix

FND-01 must discover actual workspace feature names, targets, toolchains and test entry points.
Use a matrix with rows for each selected native OS/architecture, actual browser/WASM execution,
constrained profile and optional carrier/gateway backend. Initial native Windows and Linux
coverage should be proposed and confirmed against the selected deployment scope; other targets
must be explicitly selected before they become claims.

Each row records build, unit, integration, native I/O, crypto/provider, allocation, durability,
memory enforcement and recovery evidence, plus reasons for inapplicable columns. WASM compile
success does not prove browser integration; a Linux-only bearer cannot certify Windows native
delivery. Hardware-specific claims need corresponding hardware. Simulator results remain labeled
as such. CI constraints cannot silently narrow advertised support.

Command shapes for agents to resolve into exact commands during FND-01:

```text
cargo metadata --no-deps --format-version 1
cargo check -p <owning-package> --no-default-features --features <reviewed-features> --target <target>
cargo test -p <owning-package> --features <reviewed-features> <actual-test-filter>
rustfmt --check <owned-rust-files>
```

These are planning placeholders, not executed tests or guaranteed feature names. Discover
fuzz/benchmark/workspace scripts before invoking them. Prefer targeted checks and owned-file
formatting; do not reformat concurrent unrelated work. Full integration checks run on a reconciled
source state. Include a dependency-closure check on the actual replacement application, not only
a minimal library build.

## 5. Measurement and evidence manifest

Record workload size/distribution, warm/cold cache state, hardware, power/thermal governor,
compiler/options, concurrency, durations/repetitions, time source and failures. Report sample
counts and p50/p95/p99 latency where meaningful, throughput, CPU, peak memory, I/O, storage
amplification, budget rejections and recovery time. Report variability and measurement uncertainty;
set acceptance budgets before seeing results. Do not claim linear scaling from a single machine.

Energy must identify an actual meter or estimation model and its uncertainty. CPU duration,
instructions, accelerator work, accepted useful output and joules remain different quantities.
Compare against a named, reproducible baseline rather than a hypothetical libp2p implementation.
Do not claim “state of the art,” novelty or superiority solely from passing correctness tests.

Use a retained evidence manifest per accepted change. A registry evidence entry is a relative
path under this plan directory to that manifest; it may reference separately managed artifacts
with stable identifiers, hashes, custody, retention and access instructions. Never commit secrets,
private production traffic or unrestricted evidence exports merely to satisfy a checklist.

## 6. Plan-only validation

Run `pwsh -NoProfile -File ./validate-plan.ps1` from this directory. Use the configured permitted
PowerShell host; do not change execution policy to run the validator. Windows PowerShell script
execution is disabled on the reviewed workstation; PowerShell 7 successfully ran this check.
The script is read-only: it validates package graph, checklist IDs/counts, initial/completed state,
evidence-manifest paths and local Markdown links. It does not execute runtime tests, fetch remote
sources, resolve semantic validity or certify that a checked item has convincing evidence.
Independent review remains required.
