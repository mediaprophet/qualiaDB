# Workstream 06 — Provider roles, economics and electronic evidence

Status: implementation plan only. This workstream owns ECO-01, ECO-02, EVD-01 and EVD-02; every
check is future work. Parent-registry dependencies govern scheduling, and the integrator owns
shared exports/registries. No wallet, price model, legal retention period or universal person
identifier is imposed by this plan.

Follow [library layout](../library-layout.md), [swarm protocol](../swarm-protocol.md) and
[validation matrix](../validation-matrix.md). Proposed Rust paths are relative to
`crates/qualia-core-db/src/`; reconcile placement with the parent before coding. The peer facade
consumes these libraries without a reverse core dependency. Persistence/GC, host admission and
crypto changes belong to their dependency owners, accessed through reviewed library interfaces.

Use directory-backed libraries with routing/re-export-only `mod.rs`. Separate cold planning, hot
execution, backend adapters, receipts, artifact lifecycle and tests. Keep each new implementation
file below 500 lines and split earlier for mixed responsibilities; a generic `economics.rs` or
`evidence.rs` containing all lifecycles is not an acceptable deliverable. Hot success/error/cancel
paths and ABI buffers use caller-owned fixed storage without `Vec`, `String` or `Box`. Cold scratch
requires bounded ownership and the repository's actual allocation classification.

Semantics define quantity meanings, rights, obligations, custody and uncertainty before schema
freeze. Exact artifacts remain core-managed and container-neutral; candidate QNF adoption cannot
be required to express an agreement or preserve evidence. Ordinary cells remain at most 512 MiB;
Webizen's complete evaluation-pass budget is 42 MiB within the applicable cell/host accounts.
Scaling cells must preserve parent credits, durable operation identities, holds and closure pins.

## ECO-01 — Typed compute, joules, seconds and the resource ledger

Dependencies: SEM-01, RT-01, CORE-03.

Inputs: [compute accounting](../../compute-resource-accounting.md),
[commons economics](../../commons-and-resource-economics.md), [ontological contracts](../../ontological-contracts.md),
and [network cells](../../network-cell.md). Obtain pinned interpretation bundles, atomic parent
resource admission and recoverable operation/receipt publication interfaces.

Deliverable: `net/qdnf/economics/quantities/` owns `types.rs`, `profiles.rs`, `arithmetic.rs`, `validate.rs`;
`net/qdnf/economics/resources/` owns `plan.rs`, `reservation.rs`, `meter.rs`, `attribution.rs`, `reconcile.rs`,
`receipts.rs`, `core_adapter.rs` and separate `backends/` adapters. Tests and vectors live outside
production files. The durable resource ledger adapts CORE-03, not a new database or wallet;
`reservation.rs` consumes the RT-01 parent governor rather than maintaining competing host credit.

- [ ] ECO-01.01 Define semantic quantity kinds for energy, distinct time categories and typed compute; separate provisioned capacity, consumed work, accepted useful output and economic credit before freezing records.
- [ ] ECO-01.02 Pin compute counting profiles with evaluator/cost schedule or algorithm, device, precision and counting convention; preserve vectors of incompatible work instead of inventing a universal compute scalar.
- [ ] ECO-01.03 Implement checked integer coefficient/scale arithmetic, dimensional validation, explicit conversion and rounding; reject overflow, incompatible quantities and tariffs that count an included resource twice.
- [ ] ECO-01.04 Represent measured, estimated, unknown and justified not-applicable states explicitly; absent telemetry has no numeric value and cannot silently become zero or a verified measurement.
- [ ] ECO-01.05 Bind meter observations to operation/provider scope, interval, profile, provenance and uncertainty; retain raw counter meaning, enabled/running times, scaling and wrap/reset behavior where applicable.
- [ ] ECO-01.06 Reserve conservatively across host, agent, role, cell, agreement and operation using one atomic parent admission interface; resource, money and unsettled-exposure limits remain independent dimensions.
- [ ] ECO-01.07 Include admitted work, measurement lag, parser/crypto/compilation costs, shared buffers and completion/recovery headroom in reservations; reject guarantees whose enforcement bound cannot be established.
- [ ] ECO-01.08 Charge pinned fuel before a bounded quantum and define safe overshoot for supervised backends; observational CPU counters alone must not be advertised as instruction-exact preemption.
- [ ] ECO-01.09 Preserve reservation lineage and unique operation identity through child jobs, rerouting, replacement providers and cell restart; prevent simultaneous children or new aliases from minting fresh parent credits.
- [ ] ECO-01.10 Attribute attempted, failed, retried, cached and completed work without double billing; record cache construction stewardship separately from actual serving work and counterfactual avoided consumption.
- [ ] ECO-01.11 Persist reservation transitions and usage/receipt bindings through CORE-03 with monotonic states; recover ambiguous work under its existing identity instead of releasing potentially consumed credit prematurely.
- [ ] ECO-01.12 Release unused allowances on verified completion/cancellation while retaining uncertain obligations and closure reserves; stale cell completions cannot debit or refund another generation's work.
- [ ] ECO-01.13 Integrate real telemetry only through scoped backend adapters; protect job traces and identifier relations, and keep meter authenticity separate from measurement truth and useful-output acceptance.
- [ ] ECO-01.14 Exercise profile substitutions, incompatible arithmetic, absent/scaled counters, overflow, retries and cache attribution using independent expected quantities and receipt outcomes.
- [ ] ECO-01.15 Stress concurrent reserve/commit/cancel/crash and cell-transfer races; assert parent conservation, no double release/debit and bounded durable recovery under every claimed backend.
- [ ] ECO-01.16 Measure zero-allocation hot metering/admission and aggregate memory/work bounds; publish enforceable versus estimated limits and freeze interoperable quantity vectors with SEM-01.
- [ ] ECO-01.17 Record accepted contributor labor/resources, useful-output valuation and agreed remuneration separately; voluntary donation cannot be inferred from source publication or missing payment credentials.
- [ ] ECO-01.18 Separate event operating cost, fees and creation-recovery amounts under dimensional/rounding rules; reject duplicated components, artificial cost inflation and undisclosed diversion of recovery margin.
- [ ] ECO-01.19 Bind the finite target to accepted new cost/return less prior funding exactly once, with separately tracked beneficiary entitlement and no universal exchange rate.
- [ ] ECO-01.20 Measure and report estimated/unknown event cost with explicit variance caps; resource overruns cannot silently consume the recovery allocation while claiming it paid down the obligation.

Acceptance: joules and seconds retain their physical meanings; compute values compare only under
compatible counting profiles or explicit accepted conversions. Capacity and consumption cannot
stand in for useful work. Tests must demonstrate parent-credit conservation across multiple cells
and providers, including unresolved execution and restart. No hard energy claim may rely solely
on absent telemetry or an unbounded estimate.

Swarm handoff: one worker owns quantity semantics/arithmetic, one owns reservation/reconciliation
after RT-01's interface is agreed, and backend workers own disjoint measurement adapters. An
independent reviewer tests concurrency and accounting conservation. Deliver ECO-02 and service
owners typed quantities, reservation tokens, receipt bindings, uncertainty semantics and failure
traces; request shared core/host changes through their owners rather than duplicating admission.

## ECO-02 — Provider roles, commons funding and optional settlement

Dependencies: ECO-01, RT-03.

Inputs: [semantic network roles](../../semantic-network-roles.md),
[commons economics](../../commons-and-resource-economics.md), [Identifier Fabric](../../identifier-fabric-integration.md),
and runtime service activation/withdrawal interfaces. Inherit SEM-01 contracts through ECO-01.

Deliverable: `net/qdnf/economics/provider_roles/` owns `mandate.rs`, `offer.rs`, `plan.rs`, `activation.rs`, `commitment.rs`,
`drain.rs`, `assignment.rs`; `net/qdnf/economics/funding/` owns `allocation.rs`, `entitlement.rs`, `threshold.rs`,
`receipts.rs`; `net/qdnf/economics/settlement/` owns `quote.rs`, `state.rs`, `reconcile.rs`, `receipts.rs`,
`backends/`. Keep payment-rail code and role execution in distinct libraries and tests.

- [ ] ECO-02.01 Define mandate, service offer, agreement, commitment, resource envelope and execution assignment as distinct linked records; role, cell, wallet, key and human principal must not collapse into one identity.
- [ ] ECO-02.02 Pin eligibility, purpose, scope, amendment/withdrawal and obligation semantics for each role; use current authority before service matching and cost preference, with unsupported meaning producing a non-allow outcome.
- [ ] ECO-02.03 Implement enable/advertise/admit/drain/disable transitions for routing, relay, resolution and custody roles; validate mandate, actual capability and funding before advertising a usable offer.
- [ ] ECO-02.04 Reserve capacity and accepted funding atomically at commitment, not advertisement; install bounded standing allowances so packet forwarding performs no contract compilation or settlement negotiation.
- [ ] ECO-02.05 Support gift, community pool, reciprocity, cost-sharing and paid modes under one obligation model; gifts create no recipient debt and paid participation cannot become mandatory native access policy.
- [ ] ECO-02.06 Define scoped pool eligibility, subsidy allocation and exhaustion behavior without universal person identifiers; prevent extra DIDs/cells or duplicated receipts from multiplying an entitlement or discharging the same obligation twice.
- [ ] ECO-02.07 Provision bounded discovery, authentication, error, revocation and closure allowances; avoid a paid-route dependency on the only service able to authorize that same route.
- [ ] ECO-02.08 Bind quotes to provider, service, outcome boundary, quantities, rates, fees, caps, validity and accepted profile; payment or economic preference must never widen consent, capability or sensitivity rights.
- [ ] ECO-02.09 Implement optional settlement adapters with explicit authorization, asset/issuer, finality and idempotency; do not switch rails, repeat uncertain charges or treat a submitted payment as final.
- [ ] ECO-02.10 Keep forwarding, recipient delivery, application acceptance and settlement receipts distinct; admit pool-funded rewards only under agreed useful-output evidence and bounded exposure against fabricated/collusive traffic.
- [ ] ECO-02.11 Reserve multi-hop replacement exposure before rerouting; preserve bilateral obligations, parent limits and disclosure restrictions without claiming atomic settlement across changing providers.
- [ ] ECO-02.12 Implement threshold-release predicates using accepted deduplicated contributions and explicit rights authority; preserve independent privacy duties, agreed finality and promised irreversible release conditions.
- [ ] ECO-02.13 Preserve role budgets and commitments during exclusive cell ownership transfer; thermal pressure or disable drains admitted work, while revocation halts forbidden delivery and records unresolved obligations.
- [ ] ECO-02.14 Handle disputes, refunds, cancellation, exhausted funding and unavailable rails through bounded reconciliation; a failed invoice must not silently cancel an evidence hold or substitute deletion for authorized custody transfer.
- [ ] ECO-02.15 Test community-router activation without a wallet, paid transit, intermittent custody, concurrent cell admission, stale grants, repeated receipts and ambiguous settlement with independent expected outcomes.
- [ ] ECO-02.16 Benchmark semantic feasible-provider selection against equivalent workloads including compilation/invalidation costs; freeze role/funding profiles only after tests demonstrate useful outcomes without weakening routing safety.
- [ ] ECO-02.17 Implement the finite-project-compensation profile with one stable obligation ledger across assets, providers, aliases, cells and settlement adapters.
- [ ] ECO-02.18 Enforce atomically that finalized recovery plus authorized non-cash discharges plus outstanding recovery reservations does not exceed the accepted target; reject unconstrained merge-after-collection accounting.
- [ ] ECO-02.19 Reserve the recovery component before executable debit authorization; clamp the last payment to the unreserved remainder and bind component allocation to the quote.
- [ ] ECO-02.20 Require a positive designated recovery contribution for applicable open-obligation usage above its separately accepted operating costs/fees; support explicit exempt, sponsored and fully-reserved admission outcomes.
- [ ] ECO-02.21 Distinguish received, pending, finalized, disputed and beneficiary-paid states; only accepted final credits discharge the project target, and collection alone does not prove contributor payout.
- [ ] ECO-02.22 Implement terminal fulfilment, zero future creation surcharge and stale-quote invalidation; ongoing service charges retain separate authority and cannot reset retired creation costs.
- [ ] ECO-02.23 Reconcile duplicate/late transfers, fees, cancellations, chargebacks and surplus through the original operation; refund excess or require fresh donor consent for redirection.
- [ ] ECO-02.24 Support offline recovery only through nonoverlapping pre-reserved allotments; uncertain settlement exposure survives expiry and cannot be silently reissued.
- [ ] ECO-02.25 Apply humanitarian-worker exceptions before general incorporated-principal duties for the verified operation; ordinary personal employment does not imply corporate billing.
- [ ] ECO-02.26 Provide free personal/humanitarian entitlements without wallets, forced labor or behavioral-data payment; name sponsor/fair-use/exhaustion policy and never create hidden recipient debt.
- [ ] ECO-02.27 Keep asset licences, service contracts, compensation terms and voluntary donations separate; execute only the authorized post-fulfilment permission transition without altering prior grants.
- [ ] ECO-02.28 Preserve obligation lineage across versions, forks, mirrors and provider changes; new maintenance/training compensation needs a distinct accepted scope without double recovery.
- [ ] ECO-02.29 Test concurrent last-unit charges/donations, zero targets, residual waivers, interrupted fulfilment, stale providers, offline allotments, unpaid beneficiaries and post-fulfilment chargebacks against independent ledger invariants.
- [ ] ECO-02.30 Expose contributor acceptance/valuation disputes, remaining finalized/pending target, beneficiary allocations and completed recovery through privacy-preserving operator/user views.

Acceptance: enabling a supported router role provisions its resources and economics together;
unsupported services cannot be advertised merely because a semantic descriptor exists. Donated
connectivity works without a payment adapter. A funded role preserves authority and resource caps
under reroute/restart, and failure does not invent delivery, finality or renewed credit. Different
funding models remain selectable without one universal valuation or compulsory wallet.

Swarm handoff: assign role lifecycle, funding allocation and each settlement backend disjoint
ownership. Freeze receipt-stage and reservation contracts before joining them. Independent review
checks economic replay, coercive access regressions and cell transfers. Pass EVD-02 custody funding,
renewal/transfer and reconciliation hooks; require EVD conformance before advertising preservation
services. Route runtime activation and registry changes through the parent integrator.

## EVD-01 — Retention tiers, holds and selected evidence

Dependencies: CORE-03, SEM-01.

Inputs: [electronic evidence lifecycle](../../electronic-evidence-and-retention.md),
[Identifier Fabric integration](../../identifier-fabric-integration.md), and
[core storage](../../core-storage-and-cache.md). Obtain serialized publication/GC decisions,
durable pins and bounded artifact traversal from CORE-03; obtain historical semantic bindings from SEM-01.

Deliverable: `evidence/retention/` owns `policy.rs`, `classify.rs`, `selection.rs`, `hold.rs`,
`closure.rs`, `promotion.rs`, `disposition.rs`, `receipts.rs`, `core_adapter.rs` and dedicated `tests/`.
Split traversal planning from hot continuation and backend publication when either acquires a
separate lifecycle. No packet logger may become the transaction, policy and archive implementation.

- [ ] EVD-01.01 Define transient, diagnostic, operational-audit and selected-case tiers with versioned purpose, authority, scope and expiry/review rules; require deployment policy rather than inventing universal legal durations.
- [ ] EVD-01.02 Record collection extent, relevant negative/contrary material, selection criteria, software/query versions and known gaps; preserve reviewability without retaining excluded private content unnecessarily.
- [ ] EVD-01.03 Preserve acquisition, producer, observation and witness time separately with clock provenance/uncertainty; restart or uncertain wall time must not silently accelerate deletion.
- [ ] EVD-01.04 Validate hold issuer, selectors, purpose, effective scope and release authority; track overlapping holds independently and never interpret a review date as automatic release.
- [ ] EVD-01.05 Serialize hold installation and deletion commit through CORE-03; GC must recheck the committed hold generation, and a deletion that already won must produce a visible preservation gap.
- [ ] EVD-01.06 Protect the unresolved dependency closure through bounded segment/snapshot pins or hold-aware reachability before traversal; root-only pins and truncated traversal cannot establish preservation.
- [ ] EVD-01.07 Traverse closure iteratively with bounded visited/frontier state and durable continuations; include exact source bytes, semantic bundles, dictionaries, historical authority, proofs, time and custody context.
- [ ] EVD-01.08 Apply holds to matching future arrivals and quarantine bounded unresolved selectors from GC; missing or over-budget dependencies remain explicit pending/gap states, not empty selections.
- [ ] EVD-01.09 Obtain verified remote preservation acknowledgments or protected local copies for remote dependencies; record scope, duration and failure assumptions instead of treating a local reference as remote custody.
- [ ] EVD-01.10 Reserve destination storage, closure indexes, proofs, independent-copy requirements and recovery headroom before promotion; preserve admitted holds under pressure and stop work whose required evidence cannot be retained.
- [ ] EVD-01.11 Build immutable candidates and verify exact reconstruction and the promised durability class; atomically publish bundle, closure ownership, receipt and retention state before releasing temporary pins.
- [ ] EVD-01.12 Recover crashes by retaining pending pins and resuming or quarantining partial promotion; temporary-artifact cleanup on error/unwind must never remove durably held input or an unresolved protected frontier.
- [ ] EVD-01.13 Represent redactions, normalization, assertions, corrections and reliance analysis as separately attributed derivations; preserve originals and distinguish actor instruments from natural-person identity or culpability.
- [ ] EVD-01.14 Serialize disposition with all holds, retention obligations, key retention and closure references; scope deduplication and replica deletion so one owner's expiry cannot destroy another's protected evidence.
- [ ] EVD-01.15 Inject hold/expiry/GC/key-destruction races, cyclic/large closures, late arrivals, missing remote acknowledgments and promotion crashes; verify no false complete bundle or unauthorized reclamation occurs.
- [ ] EVD-01.16 Demonstrate bounded work and exact restoration for artifacts larger than RAM or a candidate container cap; report actual storage savings, closure overhead and irrecoverable selection gaps without implying universal completeness.
- [ ] EVD-01.17 Separate protective block state, temporary contact diagnostics, voluntary reports and selected held evidence; preserve minimum required context without blanket message surveillance.
- [ ] EVD-01.18 Protect report scope and retention from a potentially abusive guardian, employer or operator; allegations trigger reviewed preservation, not automatic guilt or broad disclosure.

Acceptance: accepted holds protect the required local closure while discovery proceeds; remote
coverage remains separately evidenced. A published complete bundle has verified required context,
durability and transfer of pin ownership. Otherwise expose pending, incomplete or lost status.
Selection can reduce storage but cannot reconstruct expired unselected logs, prove substantive
truth or guarantee admissibility. Required jurisdictional decisions remain deployment-policy inputs.

Swarm handoff: assign policy/selection, hold/closure and promotion/disposition separate files and
reviewed state transitions. CORE-03's owner owns GC/publication primitives; this worker supplies
race traces and required serialization semantics. An independent reviewer checks balanced selection
and preservation under failure. Pass EVD-02 immutable inventories, closure manifests, hold states,
historical authority references and reconstruction evidence, including unresolved remote gaps.

## EVD-02 — Custody, export, long-term renewal and offline verification

Dependencies: EVD-01, ECO-02, CRY-02.

Inputs: [electronic evidence lifecycle](../../electronic-evidence-and-retention.md),
[PQ security](../../post-quantum-security.md), [semantic services](../../semantic-peer-services.md),
EVD-01's preservation states and ECO-02's custody funding/continuity hooks. CRY-02 supplies reviewed
proof-suite, key lifecycle and verification contracts; do not implement new cryptographic primitives here.

Deliverable: `evidence/custody/` owns `transfer.rs`, `acknowledgment.rs`, `witness.rs`, `repair.rs`,
`receipts.rs`; `evidence/export/` owns `plan.rs`, `inventory.rs`, `redaction.rs`, `writer.rs`;
`evidence/renewal/` owns `schedule.rs`, `migration.rs`, `attestation.rs`; `evidence/verify/` owns
`reader.rs`, `closure.rs`, `proofs.rs`, `timeline.rs`, `report.rs`. Backend adapters, offline-tool
entry points, vectors and tests stay separate from these lifecycle libraries.

- [ ] EVD-02.01 Define custody, retrieval, export, preservation and disposition grants separately; authenticate historical actor/instrument/role bindings without inferring natural-person responsibility from a signature.
- [ ] EVD-02.02 Record sender transfer and recipient acknowledgment independently, binding exact object/closure, custody terms, authority and time evidence; missing or invalid acknowledgment cannot discharge source custody.
- [ ] EVD-02.03 Verify promised copy/failure-domain requirements and recovery capacity before handoff; preserve source holds until authorized verified destination custody satisfies EVD-01's ownership transition.
- [ ] EVD-02.04 Bind storage duration, retrieval, repair, migration and verification obligations to ECO-02 funding; exhaustion requires reserve, renegotiation or verified authorized transfer, never silent release of a hold.
- [ ] EVD-02.05 Implement authorized minimal independent checkpoints with explicit witness trust/control assumptions; detect suffix deletion against a retained expected head without requiring a universal public ledger.
- [ ] EVD-02.06 Separate integrity, instrument authentication, custody continuity, selected-set coverage and interpretation in status/receipts; a valid prefix or signed claim must not become a completeness or truth verdict.
- [ ] EVD-02.07 Plan exports against current disclosure authority and pinned preserved generations; include only authorized source/provenance bytes while retaining protected originals under their own policy.
- [ ] EVD-02.08 Emit human-readable and machine-verifiable inventories with exact sizes/digests, transformations, tools, selection criteria, clock uncertainty and gaps; redacted derivatives receive new identities and proofs.
- [ ] EVD-02.09 Supply bounded offline verification using explicit trust inputs, pinned schemas/bundles and preserved historical validation material; forbid mutable web lookups or dependence on a running QDNF node for interpretation.
- [ ] EVD-02.10 Validate large chunked artifacts and reference closure with caller buffers, work quanta and explicit continuation; reject missing, reordered, duplicated or altered ranges without silently truncating originals.
- [ ] EVD-02.11 Schedule fixity and restore checks with policy-owned intervals and reserved work; record actual corruption, inaccessible keys and failed repairs and recover from independently verified copies where available.
- [ ] EVD-02.12 Perform format/media migration as linked derivation with exact reconstruction or declared transformation evidence; retain originals, old manifests and required proofs until their own authorized disposition.
- [ ] EVD-02.13 Implement profile-reviewed cryptographic renewal preserving original proofs, historical trust and new witness/clock assumptions; fresh signatures attest their actual time and claim, not retroactive PQ security or cured compromise.
- [ ] EVD-02.14 Govern key copies, backups and exported replicas through separate retention/release records; a local deletion or crypto-erasure receipt must not claim every remote copy has disappeared.
- [ ] EVD-02.15 Exercise lost/forged acknowledgments, correlated witnesses, suffix deletion, clock ambiguity, compromised keys, expired funding, interrupted migration and partial exports with independently constructed adversarial bundles.
- [ ] EVD-02.16 Demonstrate offline reconstruction and verification on each advertised backend/profile; publish pass/fail/unknown dimensions, bounded resource use, custody limitations and independent-review evidence before enabling long-term service claims.
- [ ] EVD-02.17 Require distinct disclosure authority for protective reports and independent trusted-help custody; never automatically export to the alleged abuser or a shared activity feed.
- [ ] EVD-02.18 Test contested guardians, colluding recovery keys, sensitive eligibility records and malicious report/export requests; preserve alternative review and necessary contrary evidence.

Acceptance: an offline examiner can reconstruct authorized originals or identify exact gaps, trace
derivations and evaluate preserved proofs under explicit historical trust/time assumptions. A
verifier must distinguish invalid evidence from unavailable evidence and cannot resolve truth or
culpability by boolean signature success. Renewal and witness receipts expose their actual claims;
storage payment or local hash-chain validity cannot establish independent preservation by itself.

Swarm handoff: assign custody continuity, export, renewal and offline verifier distinct owners.
The verifier worker consumes frozen fixture specifications independently of the exporter and tests
tampered/partial bundles. Review common manifest/proof interfaces with EVD-01 and CRY-02 before
implementation; coordinate funding failure traces with ECO-02. Deliver deployment-ready capability
claims, reproducible verification instructions and known gaps to the parent validation/release owner.
