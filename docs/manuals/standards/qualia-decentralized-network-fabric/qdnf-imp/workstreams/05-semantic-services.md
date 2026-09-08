# Workstream 05 — Semantic contracts and peer services

Status: implementation plan only. All checks describe future work; no implementation or profile
freeze is claimed. This workstream owns SEM-01, SVC-01, SVC-02 and SVC-03. Dependencies identify
parent-registry tasks, not permission to edit their files. The integrator owns shared registration.

Follow [library layout](../library-layout.md), [swarm protocol](../swarm-protocol.md) and
[validation matrix](../validation-matrix.md). Proposed Rust paths below are ownership proposals
relative to `crates/qualia-core-db/src/`; confirm existing owners before coding. The `qualia-peer`
facade consumes these libraries; core never depends back on it. Transactions, cryptography and
runtime leases remain adapters to their existing owners.

Every capability is a directory-backed library. Its `mod.rs` only declares modules and re-exports
the public API. Split cold planning, hot execution, backend integration, receipts and tests into
separate files; keep new implementation files below 500 lines and split earlier for mixed duties.
Use caller-owned fixed buffers and bounded iteration on Tier-1 paths, including failure/cancel.
Cold allocation requires the actual Tier-2 classification and an explicit budget, never a blanket
builder exemption. Include Webizen's complete 42 MiB evaluation-pass budget within applicable
ordinary-cell accounting of at most 512 MiB; additional cells preserve parent resource reservations.

Semantics lead interface and encoding decisions. An unresolved ontology mapping, authority recipe
or codec version is a recorded profile decision, not an invitation for a worker to invent a global
identity key or packet assignment. Keep signed artifacts independent of the candidate QNF layout;
Q42 projections and exact source objects share the core lifecycle. No new database is introduced.

## SEM-01 — Identifier Fabric and CBOR-LD contracts

Dependencies: FND-02, CORE-01, CORE-03, CRY-01.

Inputs: [Identifier Fabric integration](../../identifier-fabric-integration.md),
[ontological contracts](../../ontological-contracts.md), [identifier resolution](../../identifier-resolution.md),
and [PQ profile](../../post-quantum-security.md). Obtain versioned identifier/digest types, bounded
artifact access, atomic publication contracts and exact-proof verifier interfaces from dependencies.

Deliverable: `net/qdnf/contracts/fabric/` owns `types.rs`, `relations.rs`, `authority.rs`, `coattestation.rs`
and `recovery.rs`; `net/qdnf/contracts/` owns `bundle.rs`, `codec.rs`, `validate.rs`, `compile.rs`,
`decision.rs`, `receipts.rs` and `core_adapter.rs`. Put conformance fixtures in `tests/` and immutable
codec/profile vectors in `test_vectors/`; each library has its own routing-only `mod.rs`.

- [ ] SEM-01.01 Record the consultation revision, unresolved diagram/schema intake and supported semantic fragment; agree decisions with the integrator before freezing executable bindings.
- [ ] SEM-01.02 Define typed entity, claim, spatiotemporal-handle and instrument references; keep NaturalAgent, AI agent, guardian, organization, device, role and wallet separately enumerable.
- [ ] SEM-01.03 Model directed relation locators, aliases, role occupancy and delegation with audience, purpose, validity and revision; prohibit universal person joins and authority inferred from `owl:sameAs` or lexical similarity.
- [ ] SEM-01.04 Bind operation authority to full identifiers, proof purpose, target, current role/membership, policy epoch and revocation; treat `q_hash` as a collision-checked index, never identity or proof.
- [ ] SEM-01.05 Implement an action-authorized co-attestation recipe with predicate bindings, deduplication and common-control/delegation checks; report missing independence evidence instead of counting keys or roles as independent authorities.
- [ ] SEM-01.06 Require actual proof artifacts and verifier results for supported proof systems; reject boolean proof-success flags, confidence scores and threshold key recovery as substitutes for capability authority.
- [ ] SEM-01.07 Preserve temporal recovery, retraction and reliance records without merging persons or retrospectively upgrading trust; compromised or missing instrument evidence must retain explicit uncertainty.
- [ ] SEM-01.08 Define a pinned bundle manifest for ontology import closure, contexts, shapes, rules, evaluator, codec and compression tables; validate digest, depth, expanded graph, instruction and workspace limits before activation.
- [ ] SEM-01.09 Build bounded CBOR-LD decoding/encoding under the selected exact revision; preserve graph scope, datatypes, language and ordered lists, and reject silent plain-CBOR or unknown-critical-term fallback.
- [ ] SEM-01.10 Bind signatures to exact received payload bytes and interpretation bundle; keep semantic equivalence separate from byte identity and use distinct record versions/domains for classical and PQ profiles.
- [ ] SEM-01.11 Validate quantity dimensions, ratification and supported SHACL/N3 constructs before compiling reusable deontic, temporal and other existing modality handles; unsupported, ambiguous or exhausted evaluation must not allow.
- [ ] SEM-01.12 Keep dependency acquisition and compilation cold and budgeted; prevent runtime HTTP context fetching, recursive import expansion and string allocation in hot decisions, including error paths.
- [ ] SEM-01.13 Persist source bytes, bundle and acceptance/decision receipts through CORE-03; bind compiled views to immutable source, compiler and policy generations and revalidate at delivery/commit.
- [ ] SEM-01.14 Define typed pending, incompatibility, revoked, resource-limit and invalid-proof outcomes; never turn missing facts into consent, a guessed person identity or a new funding obligation.
- [ ] SEM-01.15 Produce positive and adversarial vectors for table substitutions, hash collisions, unsupported rules, correlated co-signers, forged proof flags, alias merges, expired roles and disputed key recovery.
- [ ] SEM-01.16 Verify deterministic bounded compilation and zero-allocation hot success/error/cancel paths; submit schema decisions, vectors, ABI usage and measured workspace evidence for profile freeze.
- [ ] SEM-01.17 Define personal, represented-incorporated and humanitarian-mandate usage classes per operation, with explicit exception precedence and no classification from employment, wallet or network address alone.
- [ ] SEM-01.18 Compile bounded Exempt/SponsorFunded/ContributionRequired/Fulfilled/Unresolved outcomes from pinned terms; keep usage classification, service permission and spend authority separate.
- [ ] SEM-01.19 Bind creation-cost target, agreed capped return, beneficiary allocations, accounting unit and obligation lineage; prevent inferred perpetual compounding or resetting a recovered cost.
- [ ] SEM-01.20 Implement ProtectionPolicy semantics for contact, discovery, introduction, location, recording, recovery and disclosure as separate nontransitive grants.
- [ ] SEM-01.21 Keep child/PEP/protection and humanitarian eligibility attestations minimally disclosed and separate from public identifiers, pricing receipts and QSR facets.
- [ ] SEM-01.22 Handle guardian/official/organizational mandates as scoped duties, including disputed authority, evolving capacity, role succession and independent help/recovery routes.
- [ ] SEM-01.23 Test personal use by an employee, an incorporated humanitarian worker, corporate proxying through a human/AI identifier and unknown/mixed-purpose classification without invented personal debt.
- [ ] SEM-01.24 Test policy precedence: protective blocks and disclosure limits survive payment, funding exemptions, guardian keys, group membership and conflicting signed claims.
- [ ] SEM-01.25 Define authenticated known-peer patient/clinician relationships with bilateral standing grants, selected records/purpose, actual decrypting recipients and policy review; public discovery is unnecessary.
- [ ] SEM-01.26 Keep care-team membership, guardian participation, referral, consultation recording and AI processing as separate authorized scopes; personal recognition or a clinician label cannot replace endpoint authentication.

Acceptance: two independently exercised codec/verifier paths must agree on frozen exact-byte
vectors and rejection behavior. A signature-valid graph with an unsupported duty, stale role or
unproven independence cannot authorize delivery. Preserve current NQuin layout and canonical tags;
do not encode proof bytes or global person identity in short hashes. Profile freeze requires the
dependency owners' review, not merely a passing unit test over this library's own serializer.

Swarm handoff: one worker owns Fabric relationships and authority; another owns contract bundle,
codec and compilation after their common types are reviewed. An independent reviewer owns hostile
fixtures and checks non-entailments. Pass SVC/ECO/EVD consumers the supported semantic fragment,
immutable bundle handles, validation/decision outcomes and fixture manifest. Give the integrator
exports and registry proposals without editing shared `lib.rs` or registries independently.

## SVC-01 — QSync durable operations and content swarms

Dependencies: RT-03, CORE-03, SEM-01.

Inputs: [semantic peer services](../../semantic-peer-services.md),
[core storage](../../core-storage-and-cache.md), [runtime API](../../peer-runtime-api.md), and
[core memory review](../../core-memory-and-parallel-networking.md). Obtain session credits,
generation-bound semantic authority and crash-recoverable multi-artifact publication interfaces.

Deliverable: `net/peer/replication/` separates `operation.rs`, `admission.rs`, `checkpoint.rs`, `proof.rs`,
`reconcile.rs`, `merge.rs`, `tombstone.rs`, `core_adapter.rs` and `receipts.rs`.
`net/peer/replication/content/` separately owns `manifest.rs`, `plan.rs`, `transfer.rs`, `verify.rs`,
`resume.rs` and `core_adapter.rs`; keep backend and crash tests outside production modules.

- [ ] SVC-01.01 Freeze scoped descriptor and operation meanings with SEM-01, then selected wire profiles, caps and vectors; preserve explicit SHA-256/classical versus SHA-384/PQ identities without truncation or reinterpretation.
- [ ] SVC-01.02 Bind each operation to author authority, epoch/sequence, scope, contract, parents, exact payload, purpose and audience; reject wraparound and unauthorized epoch replacement.
- [ ] SVC-01.03 Verify competing signed bodies and author authority before persisting equivocation or penalizing an author; unauthenticated conflicts may consume only bounded transient resources.
- [ ] SVC-01.04 Commit accepted operation identity, graph effect and result receipt atomically through CORE-03; reserve commit capacity first and acknowledge only the durability class actually achieved.
- [ ] SVC-01.05 Return an existing committed outcome for an identical authorized retry; preserve replay identity through crash, rekey, cell transfer and compaction, and recheck permission to disclose the old result.
- [ ] SVC-01.06 Implement bounded dependency queues, iterative causal traversal and explicit continuation/limit outcomes; full buffers must never truncate validation or report incomplete history as complete.
- [ ] SVC-01.07 Construct canonical authorized-view checkpoints with full operation/envelope digests, scope, count, frontier and merge profile; specify empty, odd-node and duplicate-ID cases in proof vectors.
- [ ] SVC-01.08 Verify membership separately from range coverage and set completeness; bound page/proof/work sizes and reject forged counts, omitted ranges and private neighboring metadata in proofs.
- [ ] SVC-01.09 Support complete authorized source envelopes or independently signed projector view operations; require explicit derive-projection authority and never reuse a source signature after redaction.
- [ ] SVC-01.10 Apply each dataset's accepted merge/conflict semantics; retain relevant concurrent alternatives and enforce membership/revocation authority independently of LWW or wall-clock ordering.
- [ ] SVC-01.11 Retain tombstones and replay state until an authorized acknowledgment frontier or explicit epoch exclusion permits compaction; stale returning replicas must reauthorize/resnapshot before contributing.
- [ ] SVC-01.12 Define immutable content manifests with exact ranges, strong digests, codec and decoded-size bounds; permit raw Q42/QNF artifacts only when every byte and metadata disclosure is authorized.
- [ ] SVC-01.13 Schedule bounded blocks across admitted providers/paths under shared receive, storage and verification credit; charge partial blocks, scratch, retries and old/new generations to the same parent reservation.
- [ ] SVC-01.14 Resume only verified blocks of a pinned manifest; verify block and complete artifact commitments before atomic promotion, without equating a container generation root to an authorized QSync root.
- [ ] SVC-01.15 Inject crashes and concurrent retries at identity/effect/receipt boundaries; verify scoped convergence, causal deletion, dishonest checkpoints, conflicting IDs and failures of every advertised durability backend.
- [ ] SVC-01.16 Exercise data larger than RAM with bounded scan/work quanta and output pages; measure allocation, memory and cancellation across cells and publish declared at-least-once delivery/at-most-once local-application limits.

Acceptance: repeated delivery and recovery produce one committed local effect within the declared
replay epoch, or an explicit unresolved outcome; never promise general exactly-once external work.
Prove authorized-view convergence against an independent oracle under stated merge assumptions.
Fault injection must cover every durable boundary and sparse scans, not just successful transfers.
Candidate QNF support must be optional behind the core artifact adapter and cannot block Q42 reuse.

Swarm handoff: separate operation/admission, checkpoint/proof and content-transfer owners behind
reviewed immutable interfaces. The core owner implements transaction changes; this stream supplies
required atomicity and failure traces. Deliver durable cursors, scoped checkpoint APIs, replay
contracts, promotion semantics and failure fixtures to SVC-02/SVC-03; register exports through the
integrator. Independent verification checks privacy of proofs and recovery acknowledgments.

## SVC-02 — Authorized semantic publish/subscribe

Dependencies: SVC-01.

Inputs: [semantic peer services](../../semantic-peer-services.md),
[semantic roles](../../semantic-network-roles.md), and SVC-01's verified view, journal and recovery
interfaces. Inherit SEM-01 profile decisions and runtime/core contracts through SVC-01.

Deliverable: `net/peer/subscriptions/` owns focused `descriptor.rs`, `plan.rs`, `snapshot.rs`, `journal.rs`,
`delivery.rs`, `mesh.rs`, `membership.rs`, `recovery.rs`, `receipts.rs` and `core_adapter.rs`.
Keep graph backend bindings and failure/property tests in separate subdirectories with narrow APIs.

- [ ] SVC-02.01 Define event-feed and graph-projection profiles with distinct gap, retention and completeness claims; bind the selected descriptor and semantic contract at service opening.
- [ ] SVC-02.02 Admit source inspection and projection delivery separately, binding subscriber, purpose, scope, sensitivity, filter and policy generation; reject unsupported joins or predicates explicitly.
- [ ] SVC-02.03 Compile supported filters in bounded cold workspaces; run hot matching over caller buffers with no ontology acquisition, unbounded recursion or allocation.
- [ ] SVC-02.04 Establish an atomic snapshot/live journal cut for a pinned source generation; concurrent writes must fall in the snapshot or subsequent deltas without an invisible gap.
- [ ] SVC-02.05 Emit authenticated additions/removals with stable event identities and declared ordering; preserve meaningful projection changes instead of treating opaque string broadcasts as graph deltas.
- [ ] SVC-02.06 Apply bounded journal retention and slow-consumer policy; overflow creates an explicit gap and resnapshot requirement rather than a falsely current projection.
- [ ] SVC-02.07 Commit the applied projection and resume cursor atomically; transport consumption/ACK is not application durability, and cursor loss requires recovery from committed state or a new snapshot.
- [ ] SVC-02.08 Recheck current authority before queued delivery and invalidate affected filters, tokens and pending disclosures on membership, policy or role change; define what already disclosed bytes cannot be revoked.
- [ ] SVC-02.09 Derive private scoped topic tokens under an epoch-bound profile; avoid public query text, global stable topic hashes and hidden member counts while documenting residual traffic correlation.
- [ ] SVC-02.10 Enforce distinct publish, relay, subscribe and derive-projection grants; relay ciphertext without granting read authority or permitting republication across scopes.
- [ ] SVC-02.11 Bound mesh degree, churn, fanout, digest advertisements, duplicate state and retries under aggregate session credits; reused sessions must not multiply connection reservations per subscription.
- [ ] SVC-02.12 Keep behavior scoring scoped and separate from rights or independent identity; validate origin scope, expiry and payload before forwarding and resist duplicate/invalid advertisements within fixed quotas.
- [ ] SVC-02.13 Reconcile authorized checkpoints through SVC-01 after disconnection or suspected loss; Bloom hints and successful gossip cannot establish complete delivery or a current graph view.
- [ ] SVC-02.14 Bind source versus projector provenance to SVC-01's accepted mode; reject disclosure-unsafe causal proofs instead of exposing hidden data to complete a subscription.
- [ ] SVC-02.15 Test snapshot/live races, restart after apply-before-ACK, lost cursors, membership churn, gaps and adversarial slow consumers against a reference authorized projection.
- [ ] SVC-02.16 Measure bounded work, zero-allocation hot paths and shared-credit conservation across subscriptions/cells; compare useful authorized deltas and recovery cost before claiming semantic network uplift.

Acceptance: a durable graph subscription resumes from precisely its committed projection state
or explicitly resnapshots. A transient event feed may report lost history and cannot claim this
stronger guarantee. Negative fixtures must demonstrate that neither recovery proofs nor private
topic negotiation reveal fields, counts or relationship context outside the subscriber's grant.

Swarm handoff: one owner handles planning and snapshot/journal semantics; another handles delivery,
mesh and membership after state transitions are frozen. An independent worker builds race and
disclosure fixtures. Deliver subscription contracts, cursor durability evidence, declared delivery
limits and resource measurements to runtime/provider-role integration. Route shared core journal
changes to its owner; do not grow a monolithic service file to absorb backend responsibilities.

## SVC-03 — Encrypted custody and bounded compute RPC

Dependencies: SVC-01, SEM-01, RT-02.

Inputs: [semantic peer services](../../semantic-peer-services.md), [PQ security](../../post-quantum-security.md),
[compute accounting](../../compute-resource-accounting.md), and [evidence lifecycle](../../electronic-evidence-and-retention.md).
Use existing QDP/qapp RPC and admitted executors; obtain resource leases and cancellation from RT-02.

Deliverable: **separate libraries** `net/peer/custody/` with `envelope.rs`, `keys.rs`, `admission.rs`,
`retrieval.rs`, `expiry.rs`, `core_adapter.rs`, `receipts.rs`; and `net/peer/compute/` with
`descriptor.rs`, `plan.rs`, `execution.rs`, `cancel.rs`, `result.rs`, `receipts.rs`, `backends/`.
Neither library may become the other's scheduler, crypto implementation or economic ledger.

- [ ] SVC-03.01 Define custody and compute semantic descriptors separately, with operation grants, output/side-effect limits and receipt meanings; negotiate only supported, reviewed profiles.
- [ ] SVC-03.02 Bind custody IDs, mailbox scope, ciphertext digest/length, expiry and recipient/custodian grants; verify inner operation and recipient bindings after authorized decryption.
- [ ] SVC-03.03 Require authenticated expiring recipient encryption-key advertisements and an approved offline envelope/rotation profile; block PQ-custody claims until its separate crypto vectors and review exist.
- [ ] SVC-03.04 Reserve bytes, objects, retention and replication exposure before accepting custody; atomically persist exact ciphertext and the stored receipt through the core's declared durability interface.
- [ ] SVC-03.05 Keep offered, stored, retrieved, applied and released receipts distinct; authenticate retrieval, deduplicate identical IDs/digests and reject conflicts without inventing application success.
- [ ] SVC-03.06 Bound replica sets and retrieve/retry work under the original operation reservation; enforce recipient deduplication and distinguish verified persistence from a provider advertisement.
- [ ] SVC-03.07 Revalidate delayed execution authority and deadlines; expose key loss/rotation and compromise limitations without claiming session forward secrecy for static stored ciphertext.
- [ ] SVC-03.08 Route custody expiry through core retention/hold checks and recover interrupted transitions; generic custody must not advertise EVD preservation or disclosure authority before that integration passes.
- [ ] SVC-03.09 Bind QDP/qapp requests to admitted executor/version, capability manifest, exact inputs, allowed outputs/effects, deadline and contract; semantic descriptions must never authorize downloaded arbitrary code.
- [ ] SVC-03.10 Plan work near authorized data only after privacy/purpose checks; receive bounded resource grants through shared interfaces and require current authority at dispatch and result delivery.
- [ ] SVC-03.11 Enforce caller-buffered execution, bounded quanta and cancellation/drain with reserved completion capacity; ordinary networking cells remain at most 512 MiB and Webizen passes retain their complete 42 MiB budget.
- [ ] SVC-03.12 Isolate explicitly admitted exceptional LLM/device work through its own execution profile; it cannot enlarge routing cells, reset parent credits or consume essential control reserves.
- [ ] SVC-03.13 Preserve operation identity across retries and ambiguous external effects; declare idempotence/reconciliation boundaries and never equate a core result commit with transactional rollback of an external service.
- [ ] SVC-03.14 Bind result receipts to inputs, executor, outputs, verification method and typed resource evidence; separate an issuer's claim from correctness, useful-output acceptance and settlement finality.
- [ ] SVC-03.15 Test custody crashes, expiry/hold races, duplicate retrieval, wrong recipients, lost keys and unavailable durable backends; test RPC cancellation, stale completions and duplicate irreversible-effect attempts.
- [ ] SVC-03.16 Publish independent custody and RPC conformance results, allocation/budget measurements and disabled-profile reasons; hand off resource and receipt hooks without implementing a second economics ledger.

Acceptance: a stored receipt requires demonstrated durable ciphertext custody; an applied receipt
requires its independently authorized application outcome. RPC cancellation must stop new effects,
drain owned buffers and reconcile uncertain effects. Resource telemetry may be unknown; this cannot
be represented as zero work or satisfy an agreement requiring verified measurement. Stronger offline
crypto and evidence claims remain gated on their owning workstreams' completed profiles.

Swarm handoff: assign custody and compute RPC to different workers with disjoint code directories.
Agree only descriptor, lease and receipt contracts at their boundary. Provide ECO-01/ECO-02 hooks
for resource attribution and obligation outcomes, and EVD-01/EVD-02 hooks for holds and custody
history. Those integrations are release gates for the corresponding advertised feature, not new
hidden dependencies for a minimal non-economic RPC or ordinary encrypted-mailbox implementation.
