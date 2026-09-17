# Qualia Scoped Rendezvous (QSR)

**Status:** Proposed algorithm, design revision 0.1, 2026-09-07; not implemented or benchmarked.
**Decision:** QSR replaces the proposed Kademlia lookup overlay in the target QPR architecture.
Kademlia remains a comparison baseline and may exist in separately selected legacy adapters.
QRoute still forwards packets; QSession still supplies encrypted streams. QSR is QResolve's
distributed indexing and rendezvous algorithm.

## 1. Purpose and contribution

Find a currently eligible resource/provider within an explicitly selected semantic and authority
scope, using bounded work and preserving the evidence behind the answer. Use existing QualiaDB
indexes, exact artifacts, range cursors and Webizen rather than importing a peer lookup runtime.

The proposed contribution is the combination of scope-bound semantic query compilation,
authenticated partition coverage, core index execution, reservation-aware scheduling and recoverable
provider leases. None of hashing, prefix trees, distributed indexes, caching or semantic routing is
claimed as newly invented. The combined algorithm and its gains require implementation, adversarial
evaluation and comparison. See [evaluation and prior art](./qsr-evaluation.md).

The intended improvement over the previous Kademlia proposal is fewer irrelevant candidate fetches
and repeated proof evaluations for governed semantic requests, better locality for related work,
explicit resource/coverage outcomes and a natural placement unit for multiple cells. There is no
claim of unconditional superiority for arbitrary Internet workloads.

## 2. Objects and trust boundaries

| Object | Contents and authority |
|---|---|
| Scope descriptor | Full scope reference, pinned interpretation profile, permitted publication/query operations, epoch authority, bootstrap DNIs, resource/disclosure limits and expiry |
| Scope epoch | Monotonic version, predecessor digest, immutable directory root, activation/cutover rules and endorsements required by the scope constitution |
| Facet | Versioned indexed relation or service property; full predicate/type semantics, supported entailment rules, value canonicalization and indexing completeness policy |
| Publication | Original signed RAR/provider record, source sequence/digest, declared facets, issuer grants, expiry, exact artifact references and immutable operation ID |
| Cover node | Authenticated immutable radix node identifying child intervals and serving cohorts; leaves delegate bounded partition-snapshot publication |
| Partition descriptor | Scope/epoch/facet/prefix, minimum checkpoint, snapshot-publisher grant, provider DNIs, role grants, expiry, successor/handover reference and capacity terms |
| Partition checkpoint | Signed writer-generation/version, predecessor, declared input snapshot, index root, inclusion watermark, compiler/completeness model and expiry |
| Query plan | Exact target or bounded formula, pinned scope/epoch/profile, permitted disclosures, selected index lanes, required snapshot consistency, resource reservation and return policy |
| Result envelope | Exact source references, observed versions, interpretation and coverage evidence, accepted candidates, continuation and explicit incomplete/unknown reasons |

A directory assigns **index service responsibility**, not ownership of a person, resource or route.
Only the applicable controller/instrument proof authorizes its source record. Only current QPolicy
authorizes use or disclosure. A signed directory response can be false or incomplete; a Merkle proof
binds it to a snapshot, not to reality.

Epoch authority is supplied by the existing relationship/community/realm constitution. A deployment
may select one authorized epoch publisher or an independently specified multi-party control protocol.
QSR does not pretend that threshold signatures by themselves implement Byzantine consensus. The
first profile uses a single authorized epoch writer per scope, with replicated immutable directory
objects and explicit authority-transfer records. That writer can censor or become unavailable;
cached epochs preserve only their remaining authorized lifetime. This availability/governance tradeoff
is declared. There is no network-wide root, compulsory person identifier or compulsory scope operator.

Independent scopes can federate through explicit scoped delegation. Federation is a finite set of
authorized search spaces, not an automatic global crawl. An entity can publish in multiple scopes
without making their identifiers publicly linkable.

## 3. Keys and semantic lanes

The target PQ profile uses full SHA-384 commitments and HMAC-SHA-384 for private tokens. Domains
below identify distinct operations; canonical encoding and profile version are included in every
input. Raw concatenation of variable strings is prohibited.

```text
exact = H("qsr.exact", profile, scope, canonical_target)
facet = H("qsr.facet", profile, ontology_bundle, indexed_relation, canonical_value)
item  = H("qsr.item", profile, scope, stable_source_record_identifier)

public address = (scope, epoch, lane_selector, partition_key_prefix)
private token = HMAC(scope_index_key, domain, profile, purpose_class, field, key_generation,
                     epoch, canonical_index_term)
```

An exact lane partitions canonical-target digests and works without a semantic classification.
Semantic lanes first select an authorized facet token, then partition item digests within that
facet. This groups relevant postings without placing all traffic for a popular facet on one server.
Records may have several bounded index postings referencing one retained source artifact.
Lane selection and partition-key selection are explicit:

| Mode | Lane selector | Primary partition key | Secondary posting key |
|---|---|---|---|
| Public exact | H(profile, scope, exact-lane domain) | exact target digest | stable item digest |
| Public facet | H(profile, scope, facet-lane domain, facet digest) | stable item digest | stable item digest, only if a separate posting collection is needed |
| Private exact | HMAC(scope key, bound context, exact-lane domain) | HMAC(scope key, bound context, exact-target domain, canonical target) | HMAC(scope key, bound context, item domain, stable record ID) |
| Private facet | HMAC(scope key, bound context, facet-lane domain, canonical facet term) | HMAC(scope key, bound context, item domain, stable record ID) | the corresponding private item key |

Bound context includes profile, scope, purpose class, field, key generation and epoch. These
private tokens replace the corresponding public lane and partition keys, not the retained full
source identifiers used inside authorized verification. Every domain input is length-delimited.
The exact lane uses the target digest as its primary split key; facet postings use the stable
item digest. Publication operation IDs and source sequence numbers are separate version fields,
so an update does not arbitrarily move its source to another item partition. Several provider
records under one target use a secondary posting cover keyed by stable full record identity.

The selected public facets reveal their meaning. Private tokens hide cleartext from parties without
the key, but reveal equality and access patterns; authorized members can still make dictionary
guesses. They do not provide searchable-encryption or traffic-analysis guarantees. Where even
equality leakage is unacceptable, use local/private execution or an explicitly trusted proxy with
separate disclosure consent; do not quietly expose tokens in a public directory.

Facet expansion uses pinned, supported ontology rules in bounded core compilation. Neither embedding
distance nor a classifier can supply an authority decision or silently remove candidates. A
sound indexing profile states which source classes are indexed and which supported entailments
must produce postings. If expansion cannot complete within budget, publish/query returns incomplete
or uses an admitted unfiltered source lane. It never treats missing inferred postings as absence.

## 4. Directory and bootstrap

Each scope epoch commits to a radix cover over its admitted lanes and key intervals. In the reference
profile radix digits are four bits; an uncompressed key path has at most 96 digits for a 384-bit key.
Path compression records the skipped prefix explicitly. A verified internal node lists at most
16 nonoverlapping child intervals whose union is the parent interval, including explicit empty
intervals. Leaves bind partition descriptors and their snapshot-publication grants. A partition
writer publishes separately signed immutable checkpoints under that grant; ordinary source updates
therefore do not require a new scope cover. Changing writer authority, partition bounds or the
minimum accepted writer generation does require an authorized successor cover.

Select a checkpoint only after verifying its cover-granted writer, partition/epoch, version,
expiry, declared input and index root. Pin it for pagination and persist the client's known minimum.
Conflicting roots at the same writer-generation/version return Conflict. A larger checkpoint
number from an unauthorized writer is invalid; a valid observed number still does not prove that
no newer checkpoint exists. Reader freshness requirements may force an explicit stale/unavailable
result. Serving replicas return the writer's committed checkpoint and cannot mint their own.

The full directory stays in core-managed storage. A node reads only bounded pages and proofs;
verification checks the complete node's partition of its interval before any pruning. It cannot
infer coverage from a partial child list. A node may list unavailable intervals, which preserves
coverage knowledge without claiming they can currently be searched.

Bootstrap inputs are authenticated DNIs in the scope descriptor, provided locally, through an
existing relationship or through authorized discovery. They establish QRoute/QSession access to
directory replicas without first requiring QSR to resolve its own bootstrap. The client verifies
the selected scope root against its authority input. Learning several bootstrap keys from one
untrusted introducer does not establish independent control.

Directory pages can be replicated, cached and fetched through any permitted carrier. Their integrity
is checked against the selected epoch root; their availability and freshness remain separate.
Public join does not mean permission to mutate someone else's directory. A newcomer lacking any
authorized scope/bootstrap input returns ScopeUnavailable, not global not-found or DNS fallback.

Persist accepted (scope, epoch, root digest, authority-transition digest) checkpoints. Two authorized
roots at the same epoch produce Conflict and bounded equivocation evidence; do not choose by hash,
arrival time or an unvalidated larger epoch. Key rotation/recovery follows the constitution's
explicit old/new authority binding and recovery policy. If the authorized publisher and recovery
route are lost, QSR cannot manufacture a successor. Checkpoint gossip can expose forks but cannot
guarantee their discovery during partition or decide their resolution.

## 5. Publication and withdrawal algorithm

1. Validate the publisher's indexing grant, source authority, profile, sizes and scope disclosure.
   Reserve publication, retained source, posting, proof and replication work before accepting.
2. Persist the exact signed source and a bounded publication intent through CORE-03. Compute the
   exact key and supported facet postings through caller-owned core workspaces.
3. Route postings using the pinned cover. Each receiving provider validates the partition/role
   epoch, exact publication identity, source binding and allotted resources.
4. Atomically publish each provider's postings, source references and receipt under a new immutable
   snapshot generation. Retried identical operations return the same committed outcome; conflicting
   bytes under the same operation ID are rejected.
5. The publisher retains a completion vector for every required lane/replica. Publication is
   IndexedComplete only for the declared lane set and required receipt policy. Partial success
   stays visible and recoverable. A signature does not prove that all possible publishers enrolled.
6. Withdrawal is a new authorized version/tombstone. Persist it with index changes and retain it
   through the selected replay/replica horizon. A reader enforces its known minimum source version;
   older snapshots cannot restore an already observed withdrawal.
7. Recheck current source authority and expiry at result delivery and subsequent use. An unobserved
   withdrawal cannot be magically known during partition: expiry, freshness requirements and
   controller checks define whether use is allowed, unknown or denied.

Replica receipt policies are explicit: one acknowledged provider proves one claimed store; several
matching replies do not prove independent control, universal publication or latest state. Strong
replicated mutation semantics require a separately accepted replication/control protocol. QSR's
reference lookup binds immutable snapshots and reports version uncertainty instead of inventing
linearizable “latest” reads.

Submission received, source durably accepted and source included in a committed index snapshot are
distinct receipt states. Each index snapshot binds its declared input snapshot, indexing/admission
policy, compiler/profile version and output root. Its completeness level is either an authorized
publisher assertion or independently reconstructed and checked coverage of those declared inputs.
Without the latter evidence, a matching index proof cannot be advertised as independently verified
omission freedom. Neither level proves that all eligible external providers published into the scope.

## 6. Query compiler

Inputs include scope, purpose, exact target or supported formula, authority evidence, permitted
disclosures, desired result count and independent time/energy/typed-work/byte budgets.

The initial formula language supports exact-target lookup, conjunctions of indexed equality/class
facets, bounded disjunction and residual local predicates. It does not claim arbitrary distributed
SPARQL, global negation, unlimited transitive closure or globally optimal top-k discovery.

For a conjunction, choose a sound complete posting lane as an anchor and evaluate the remaining
conditions over its authorized source candidates. Estimates choose the anchor, never determine
correctness. Intersect independent facet postings only when their publication/version semantics
establish the same source snapshot and completeness; otherwise use source evaluation. For OR,
search every required branch or report the uncovered branches. Negation requires an explicit closed
snapshot universe and otherwise returns unsupported/unknown.

Cold compilation uses core statistics and supported semantic rules to choose among a bounded set
of plans. The cost vector includes expected page reads, candidate bytes, proof jobs, typed work,
latency and any accepted monetary terms. Estimates carry provenance/uncertainty. The caller supplies
lexicographic priorities or dimensionally valid tariffs; joules, seconds, work and money are never
added as bare numbers. A cheap plan cannot widen disclosure or authority.

## 7. Lookup algorithm

The iterative client state machine is:

```text
Admit -> Compile -> Locate -> Fetch -> Verify -> Select -> Return
                      |        |        |
                      +--------+--------+-> Pause / Incomplete / Denied
```

Algorithm, with all collections caller-owned and bounded:

```text
step(query, state, buffers, events):
  debit_work_quantum_or_pause()
  recheck_scope_epoch_authority_and_deadline()
  if first_step:
    reserve_parent_budget()
    compile_supported_plan_or_return_explicit_failure()
    try_current_verified_local_handles()
    enqueue_required_lane_intervals_in_stable_order()

  consume_at_most_event_quantum(events):
    validate_response_binding_and_debit_actual_cost()
    accept_only_verified_cover_edges_or_source_candidates()
    retain_uncovered_intervals_and_typed_failure_reasons()

  while work_remains_in_this_quantum:
    choose_next_admitted_task_by_caller_priority_and_stable_tie_break()
    if cover_task:
      verify_root_binding_and_full_node_coverage()
      follow_only_strictly_longer_matching_prefixes()
      enqueue_uncovered_required_children_or_checkpoint_frontier()
    if partition_task:
      choose_eligible_provider_with_known_diversity_constraints()
      reserve_reply_and_verification_capacity_before_request()
      request_bounded_posting_page_bound_to_plan_and_snapshot()
    if candidate_task:
      verify_exact_source_and_supported_semantics()
      apply_current_Webizen_authority_and_scope_checks()
      merge_by_exact_record_identity_without_person_identity_joins()
  return_ready_results_with_coverage_or_pause_with_continuation()
```

Each response binds operation, scope, epoch, plan digest, lane/prefix, snapshot, cursor and peer
service role. A reply for another query or generation cannot install state. Records rejected by
an authenticated source check do not invalidate unrelated valid candidates.

A progress token is the selected epoch plus traversal stage, lane, prefix depth and continuation position.
Descent strictly increases prefix depth; scans advance their authenticated page cursor. A failed
replica retry consumes attempts and budget but does not count as logical progress. The client
records bounded visited identities and does not follow lateral peer referrals as authority.
There are at most three ordered trie stages per branch: lane selection, primary partition and an
optional secondary posting cover. Each has depth at most 96 digits. Moving to the next stage resets
that stage's depth only; it consumes the same node/work/proof budgets. Returning to an earlier stage
requires a separately counted restart. Epoch redirects are bounded transitions, never a way to
reset the original reservation. The 288-digit bound for one three-stage branch is not a bound on
total work across several branches or retries.

For the reference scheduler, required control/expiry handling precedes data work. Ready query tasks
are ordered by the caller's declared cost-vector priority, then oldest waiting task, then full
lane/prefix digest. Provider choice first filters current grants, scope and hard caps, then applies
the same priority with a stable digest tie break. Missing estimates carry unknown values under the
caller's declared uncertainty policy; they cannot make a required branch disappear. Reserve one
of the three request slots for the oldest admitted required branch when other branches keep
producing work. Estimates affect scheduling only, so bounded starvation prevention preserves the
coverage obligation even when an estimator or provider is misleading.

Local compiled plans may cache verified cover/source handles. Cache keys include scope, profile,
policy, authority epoch, source/snapshot generation and disclosure class. Reuse still checks
expiry/revocation/current minimums. Webizen's lossy slot table is only an accelerator.

## 8. Completeness, continuations and negative answers

Return typed outcomes: Matches with coverage status; EmptyInSnapshot; Incomplete with uncovered
intervals; Stale; ScopeUnavailable; Ambiguous; Conflict; Denied; Unsupported; BudgetExhausted.
Wire representations belong to the shared profile freeze; these meanings are fixed here.

EmptyInSnapshot requires an authenticated complete cover of the selected interval(s), complete
page/range absence evidence or an exhausted authenticated scan, and a sound indexing profile
for that committed dataset. It says nothing about unregistered resources or an unavailable newer
epoch. A Bloom filter, estimator, directory signature, partial page or provider silence cannot
support it. If the core cannot produce the needed proof, scan within budget or report incomplete.

Every coverage/absence result carries the declared input snapshot, posting checkpoint, compiler/
admission policy and completeness trust level. With publisher-asserted indexing, EmptyInSnapshot
proves emptiness only in that committed index, with asserted source coverage. Only independently
checked source-to-index reconstruction supports the stronger declared-source-snapshot claim.
The caller can require that stronger level and otherwise receive incomplete/unsupported.

A query spanning shards records a snapshot vector. It cannot claim one global point-in-time view
unless an accepted source generation explicitly binds that vector. “Give up to eight matching
providers” can stop early with partial coverage; “all providers” cannot. Global top-k is not claimed
without complete search or proven admissible bounds over every remaining partition.

When the frontier cannot fit in memory, store it as a bounded core artifact and return an
authenticated continuation referencing it, or return incomplete with a restart point. Never
silently discard required branches. Continuations bind input/profile/epoch/snapshot, authority,
budget lineage and expiry; resuming neither reauthorizes old grants nor replenishes credits.

## 9. Placement, hot keys and multiple cells

A partition has one currently authorized index-generation writer in the reference profile and
bounded serving replicas. Cohorts receive routing/index roles through scope policy, with declared
failure domains and accepted funding. Key count alone is not a diversity measure.

Split a busy prefix into its 16 child prefixes when admitted occupancy/work crosses the configured
high watermark for a sustained interval. Merge only after the low watermark and dwell time, with
capacity reserved for old/new overlap. Candidate thresholds are profile inputs, not performance
claims. Predictive or semantic materializations are optional bounded accelerators built only after
publication/profile checks; their absence cannot break exact lookup.

A single hot exact key cannot be split by hashing its identity again. Replicate its immutable
snapshot pages, coalesce identical scoped reads, bound negative/proof caching and rate-limit
requests under parent budgets. Facet partitions can additionally shard by item digest. Count
replication and invalidation traffic in every claimed improvement.

If one exact target's posting list exceeds a cohort's serving budget, a secondary cover partitions
that list by stable full record identity. Its nodes bind a stable parent identity/context excluding
the parent's final digest and secondary-root field. The primary posting leaf commits to the computed
secondary root; the partition checkpoint commits to the primary index root. This acyclic order
uses the same interval/proof/continuation rules. Splitting storage does not remove the need to
visit every required posting partition for an exhaustive query. Coalesced requests share only
authorized immutable work; each consumer retains independent disclosure checks and accounting.

Handover protocol:

1. Reserve destination cells/storage and accept their scoped service grants.
2. Transfer an initial pinned snapshot and bounded deltas while the old writer remains active.
3. Durably fence old-generation write admission under the authorized cutover intent, drain or
   reject in-flight writes, and commit the final replay boundary. No old-generation write may be
   acknowledged beyond it. Transfer and verify every accepted mutation through that boundary.
4. Obtain destination readiness receipts for that exact final snapshot/boundary, then activate
   the successor cover and new writer generation. The handover certificate binds old/new cover
   roots, the durable fence, source/posting roots, final boundary and readiness receipts.
   New writes use the new generation; old-epoch requests fail or redirect using the same operation.
5. Retain old pages for admitted readers/continuations; expired readers replan. Publish version
   uncertainty during incomplete cutover rather than asserting uninterrupted latest reads.
6. Release old ownership only after reader/retention/hold obligations permit it. Crash recovery
   resumes the same intent and never creates two authoritative generation writers.

If epoch authority cannot issue a safe fence, do not split into competing writers. A declared
partition may lose write availability; no availability/consistency guarantee is implied under an
unbounded partition. Malicious old writers can sign stale replies, so clients must enforce the
pinned epoch/freshness rules and report when a newer epoch cannot be established.

Ordinary worker cells remain at most 512 MiB. Webizen passes retain their complete 42 MiB budget
within admitted cell/host resources. Routing, index serving and semantic evaluation may be separate
cell roles. Several cells share immutable core pages and typed references through explicit leases;
neither shared mappings nor GPU work removes host accounting. No new database or file format is
required; Q42 supplies semantic projections and existing artifacts preserve exact proofs.

## 10. Bounded reference profile

These are concrete initial experiment limits, reviewed before interoperability freeze. Tighter
admitted profiles are allowed; larger values need versioned resource review. They are upper bounds,
not preallocated giant arrays or claims that a whole operation fits without accounting.

| Resource | Initial bound |
|---|---|
| Active plans per admitted operation | 1, with at most 8 candidate anchor plans considered during compilation |
| Supported formula branches | 8; broader queries require explicit continuation or rejection |
| Concurrent directory/provider requests | 3 total |
| Active cover/frontier descriptors | 32; excess checkpoints or returns incomplete |
| Cover descent per invocation | 12 nodes across all stages; at most 96 digits per trie and 3 ordered trie stages per branch |
| Epoch redirects per operation | 2; then Stale/Incomplete, with the same budget lineage |
| Collected candidate descriptors | 64, matching the existing QResolve bound |
| New authority/proof verification jobs | 16 per operation, shared across cover, provider and source validation |
| Returned routes | 8; at most 3 parallel connection attempts after resolution |
| Receive/parse page cap | 16 KiB per message; larger logical objects require authenticated chunks |
| Compiled descriptor cache example | 4,096 entries at 128 bytes = 524,288 bytes, excluding all backing proofs/state |

A verification job must itself have bounded proof/chain size and primitive/work limits; one job
cannot hide unlimited dual signatures or ontology dependencies. Cached valid proof reuse counts
its checks and residency. Cold lookups may exhaust the 16-job bound before returning eight routes,
which is an explicit incomplete outcome, not a reason to omit verification.

The 128-byte descriptor is an internal sizing example, not a network ABI. Full SHA-384 references,
role proofs and source bytes live in admitted core artifacts; a 60-bit Q42 handle cannot replace
them. Peak operation memory is the sum of descriptors, receive/transmit buffers, proofs, crypto
scratch, compiler state, result storage, continuation ownership and concurrent old/new state.
Measure it; the table alone is not a memory proof.

## 11. Economics and evidence

Apply [finite compensation](./finite-project-compensation.md) and
[socially defined protection](./socially-defined-protection.md): use compiled operation-specific
contribution/exemption handles, retire fulfilled creation surcharges, and never expose eligibility
or child/PEP/protection membership through public facets or absence evidence. Economic ranking
cannot buy discovery, introduction, contact or disclosure rights.

Directory, replica, query-planner and index-serving roles are funded by gifts, community pools,
reciprocity or accepted paid terms. Epoch publication does not require a wallet. Providers quote
versioned terms for their measurable work; consumers reserve caps before choosing them.
Parent allowances follow the operation through retries, splits and provider changes.

Track energy, elapsed/CPU time and typed compute independently, with measured/estimated/unknown
states. A provider's claim of work or a signed response does not prove useful output or settlement.
Do not authorize a provider merely because it is cheaper, paid, highly connected or semantically
similar to the requested service. Purchased priority cannot consume reserved control capacity.

Routine query logs expire according to scope policy. Preserve selected disputed receipts, source
versions, cover/interpretation roots and custody context through the existing evidence lifecycle.
Directory replication is not an evidence hold. Bloom summaries and hashes cannot reconstruct
discarded source data. Protect against query-log correlation and preserve relevant contrary
evidence without a universal public query ledger.

## 12. Security properties and limits

- Invalid records cannot authorize a route if source/controller, purpose and current-policy checks
  are correctly implemented. Cover/placement membership alone never grants that authority.
- A valid committed cover provides structural interval coverage; it does not prove honest enrollment,
  actual facet truth, current reachability or completeness across all possible scopes.
- An attacker may censor an entire known bootstrap/replica set. QSR reports unavailable/incomplete;
  it offers no unconditional eclipse or permissionless Sybil-resistance theorem.
- Budget limits apply before parsing, crypto, inference, replies, logging and retry. Signed clients
  and economically funded roles still receive quotas.
- A compromised semantic bundle, scope root or allowed source can undermine that scope. Root
  diversity is an explicit trust decision, not a count of co-attestations.
- Learned cost estimates can degrade efficiency, but cannot prune required coverage or alter authority.
- Within a fixed finite epoch/snapshot and finite admitted plan, strict prefix/cursor progress plus
  bounded retries makes the state machine terminate or yield explicitly. It does not guarantee a
  matching answer exists or can be reached within the operation's budget.

## 13. Implementation boundaries

NET-04 owns the algorithm through directory-backed libraries under
`net/qdnf/resolve/qsr/`: `keys/`, `cover/`, `publication/`, `planner/`, `lookup/`,
`placement/`, `handover/` and separate `tests/`. Each separates types/validation, hot execution,
cold preparation and backend adapters; `mod.rs` only routes exports. Keep implementation files
below 500 lines and obey existing decomposition thresholds.

CORE-03 owns atomic artifacts and recovery; CORE-02 owns bounded scans/proofs; RT-01/02 own
reservations/cells; SEM-01 owns sound compilation and authority; CRY-01/02 own proof/key profiles;
ECO/EVD owners supply funding and preservation interfaces. QSR supplies no duplicate evaluator,
signature implementation, ledger or storage engine. Consult the
[implementation checklist](./qdnf-imp/workstreams/03-native-network.md) and
[evaluation protocol](./qsr-evaluation.md).
