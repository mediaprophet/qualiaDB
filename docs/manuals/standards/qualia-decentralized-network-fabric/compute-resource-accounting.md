# QDNF Compute, Energy and Time Accounting

**Status:** Semantic design proposal; quantity profiles and encodings remain open
**Date:** 2026-09-06

## 1. Three baseline dimensions

Describe computational service use through **energy, time and typed compute work**, including
donated and community-funded services. Energy in joules and duration in seconds have physical
meanings. Compute describes operations under a named counting model; there is no hardware-independent
SI base unit of compute. Its semantic definition is essential to comparing offers and contributions.

Keep provisioned capacity, executed work, accepted useful output and economic credit separate.
A reserved accelerator-hour does not establish executed operations or useful results. A delivered
range or accepted inference output is an outcome; a community credit is an agreement-backed claim.
Neither substitutes for a resource observation. Router agents consume compute through forwarding,
proof verification, policy evaluation and cache construction as well as application execution.

## 2. Meaning precedes the counter

| Compute quantity | Required interpretation | Comparison boundary |
|---|---|---|
| VM steps or weighted fuel | Evaluator/version, instruction/cost schedule and charge points | Same pinned schedule; fuel is a work allowance, not measured joules |
| Retired instructions or CPU cycles | Device/ISA, counter event, process/thread scope, interval and scaling | Different CPUs, instruction mixes and counter definitions are not interchangeable |
| Algorithmic operations | Algorithm/version, parameters, input size, precision, rounding, sparsity and counting convention | A multiply-add counted as one operation cannot silently compare with a two-operation convention |
| Cryptographic work | Primitive, parameter set, input length, attempt/completion/success distinction | A signature verification is not a hash or an arbitrary cryptographic operation |
| Model tokens/inference work | Model/tokenizer digests, input/output roles, batch/context and output acceptance criteria | Token counts alone do not normalize models, quality or execution cost |
| Reference workload credits | Workload, reference run/system, validation method and conversion agreement | Benchmark-relative valuation, not a physical conversion |

Preserve a vector when work uses several profiles. Do not add cycles, FLOPs, fuel and tokens without
an explicitly accepted conversion model and its uncertainty. Equal operation counts can involve
different memory traffic and hardware efficiency. Counter semantics and availability depend on the
processor; multiplexed measurements may need scaling and enabled/running time reporting, as described
in the primary [Linux perf tutorial](https://perfwiki.github.io/main/tutorial/). Benchmark speed and
throughput also differ; pin workload and validation conditions when using a reference such as
[SPEC CPU 2026](https://www.spec.org/cpu2026/Docs/overview.html).

## 3. Semantic observations and agreements

A compute observation identifies operation/provider scope, quantity kind, unit/counting profile,
profile digest, evidence state, interval and provenance. Reuse the checked integer/decimal
representation in [Commons and Resource Economics](./commons-and-resource-economics.md). Values
for `unknown` are absent, never zero. Computer-performed work under an accounting agreement declares
compute as `measured`, `estimated` or `unknown`; `not-applicable` requires an inapplicable quantity
and a reason. No sensor is required to participate.

The counting profile explains relevant device/backend, evaluator/compiler/kernel versions,
algorithm/precision and input/output commitments. Disclose only facts needed for the agreement.
Counters distinguish attempted, completed, failed, retried, cached and accepted work. Parent/child
attribution must prevent billing the same work twice. Public discovery does not expose private job
traces, model inputs or identifier relationships.

These are ontology concepts bound through [CBOR-LD bundles](./ontological-contracts.md), not allocated
packet fields or a frozen vocabulary. Incompatible meanings produce an explicit incompatibility
outcome. Price may use an agreed subset of resources or outcomes while physical and spending caps
remain independently enforceable. Machine work never determines a person's worth or time value.

## 4. Limits and useful work

Reserve work across host, cell, role and agreement scopes before admission. A bounded evaluator
charges its pinned fuel schedule before each operation or bounded quantum. Include parser expansion,
proof verification, compilation and already admitted work. Splitting or moving a job between cells
preserves its parent reservation and operation identity.

Hardware counters commonly provide observations rather than safe instruction-by-instruction
preemption. A claimed hard work bound needs a bounded algorithm, enforceable fuel or supervised
execution with stated overshoot. CPU time quotas and queue limits are complementary protections.
No cell may relabel a job under a cheaper profile to escape its accepted cap.

Existing [Vibe budgets](../../../../crates/vibe/src/budget.rs),
[VM charging](../../../../crates/vibe/src/bytecode/vm.rs) and
[resource contracts](../../../../crates/qualia-core-db/src/governance/coordination.rs) provide
instruction or caller-charged cycle/token mechanisms. They are reuse points, not universal CPU
meters, shared network reservations or evidence of actual energy consumption.

Useful output has an independent acceptance rule: delivery boundary, correctness/quality,
completeness and verifier. Consumption alone does not establish contribution. Failed work is
billable only under accepted terms. A cache hit accounts for actual lookup/serving work; construction
or stewardship can receive separately agreed compensation without claiming construction repeated.
The system must not reward manufactured traffic or wasted computation.

## 5. Rates and illustrative quantities

For a fixed compute profile `C`, `C/s` is work rate and `J/C` is energy per counted operation.
Power is `J/s`. There is no universal conversion from compute to energy or time independently of a
workload/execution model. Physical units follow [BIPM SI definitions](https://www.bipm.org/en/measurement-units/).

For an illustrative job, 400 completed verifications in two seconds give 200 verifications/s under
that profile. An attributed average of 20 W gives **estimated** energy of 40 J, or 0.1 J per completed
verification. A schedule charging 25 fuel units per attempt charges 10,000 for 400 attempts. Fuel
does not prove the energy estimate, and completed checks need not succeed. These are arithmetic
examples, not QualiaDB measurements.

## 6. Acceptance

[P18](./implementation-conformance.md#qdnf-p18--typed-compute-accounting) selects and freezes quantity
profiles after representative router, policy, crypto and compute workloads establish their usefulness.
Verify dimensional mismatch/overflow rejection, unknown handling, cost-schedule binding, concurrent
reservations, retries/cache attribution, unavailable/scaled counters and independent outcome acceptance.
A signed meter record authenticates an issuer's claim, not the correctness of work or measurement.
