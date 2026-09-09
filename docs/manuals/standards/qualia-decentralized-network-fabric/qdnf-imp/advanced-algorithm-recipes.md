# Advanced algorithm implementation recipes

**Date:** 2026-09-09

**Status:** Detailed future implementation assignments, not implemented capabilities.

**Parent:** [Enhancement plan](0.0.37-enhancement-plan.md). Also read the [sensitive-operations blueprint](sensitive-operations-blueprint.md).

This document turns the most complex work into bounded, reviewable steps. Reuse the existing protocol specifications as the semantic authority, particularly [QSR](../qualia-scoped-rendezvous.md), [post-quantum security](../post-quantum-security.md), [QSession](../qsession-and-services.md) and [finite project compensation](../finite-project-compensation.md). If an implementation recipe conflicts with those specifications, record an explicit owner-reviewed version change; do not silently choose whichever behaviour is easier.

## 1. QSR: full-key authenticated lookup

### 1.1 Data and file ownership

Create `resolve/qsr/` with:

| File | One responsibility |
|---|---|
| mod.rs | Public re-exports only |
| key.rs | Full-key and four-bit digit operations |
| cover.rs | Parsed cover structure and coverage checks |
| authority.rs | Verified cover/publisher/source grants |
| node.rs | Bounded node codec and canonical commitment preimages |
| proof.rs | Membership/coverage verification against a pinned root |
| cursor.rs | Version-bound continuation encoding and validation |
| traversal.rs | Iterative state machine and work accounting |
| postings.rs | Sorted stable-record-ID intersection/continuation |
| snapshot.rs | Snapshot generation, rollback and fork handling |
| publication.rs | Durable verified source/index publication |
| handover.rs | Writer transition and fencing |
| tests/ | Independent closed-world oracle, corruption and boundary cases |

Use full StrongDigest values at security boundaries. `digit(key, depth)` rejects depth >= 96 and returns values 0–15:

```text
byte = key[depth / 2]
digit = if depth is even then byte >> 4 else byte & 0x0f
```

This helper is only indexing. It cannot certify that the target record exists or that a cover is authentic.

### 1.2 Verified inputs and outputs

A lookup job contains a verified scope invitation, exact query or compiled semantic plan, authorisation permit, pinned cover epoch and snapshot checkpoint, deadline and total remaining work/byte/verification budget. The query's hash, sensitivity label and recipient context are part of its identity.

Results are a tagged union:

- `Found`: authenticated candidate references with their exact target binding and permitted route adverts.
- `EmptyInSnapshot`: absence or exhausted postings within the named snapshot and declared completeness assurance.
- `NeedContinuation`: bounded progress with an owner/generation-bound cursor.
- `Incomplete`: exhausted operation resources or unavailable evidence; not a negative answer.
- `Stale`: required generation/freshness not met.
- `Conflict`: roots/authorities disagree or the requested policy has no admissible result.

Never return bool for an authenticated distributed lookup.

### 1.3 Traversal loop

```text
initialise(job):
    verify invitation, query permit, cover authority and checkpoint
    pin immutable input generations and exact query digest
    initialise bounded frontier at the authorised lane root

poll(job, incoming_verified_nodes, caller_output):
    reject expired/revoked/mismatched owner generations
    debit every received byte, proof verification and node expansion
    while invocation budget remains and frontier is nonempty:
        take next deterministic frontier item
        verify node root binding, prefix, depth and required child coverage
        if exact terminal:
            compare FULL key and source-controller authority
            validate target-bound adverts and stage authorised results
        else if posting terminal:
            consume sorted stable record IDs with bounded intersection
            evaluate supported residual predicates under the query permit
        else:
            push only matching children with STRICT prefix/depth progress
        if output is full: return validated continuation
    if unfinished:
        return continuation if total operation budget remains
        otherwise Incomplete
    return Found or EmptyInSnapshot with explicit completeness evidence
```

Do not update trusted caches from an unauthenticated node. Do not treat a signed publisher statement as independently established completeness.

Follow the existing reference bounds: three outstanding requests, 32 frontier entries, 12 node expansions per invocation, two epoch redirects, 64 candidate records and 16 shared verification jobs, unless a reviewed profile version changes them. Charge the **same operation budget across continuations**, retries and redirected epochs; resetting a per-call counter must not create unlimited total work. A pathological full-depth query may legitimately be incomplete under a small budget.

The lane selector, primary key and optional secondary posting trie are separate ordered stages. Ninety-six levels applies to one 384-bit trie, not a total of 96 arbitrary-valued buckets. Node compression/multiproofs can reduce traffic only after independent equivalence tests.

### 1.4 Cover and cursor invariants

- Verify prefix length, child intervals, ordering, overlap and both range endpoints against the expected parent domain.
- Reject duplicate child claims, missing intervals represented as complete, out-of-domain paths and non-progressing edges.
- Cursor binds operation, query, scope/epoch, snapshot root, compiler/profile, last emitted full key/record ID, stage, policy generations and remaining budget.
- Protect exported cursor integrity using an established authenticated mechanism; a server must not trust a client's claimed remaining budget.
- No record is skipped/duplicated across continuation. A changed snapshot starts a new declared query or an explicit reconciliation transition.
- Private cursor/token data receives the query's markings and is never a public pagination identifier.

### 1.5 Concrete child assignments

- [ ] QSR-A: digit/key comparison and 0/95/96-depth tests.
- [ ] QSR-B: complete cover validator with interior and edge-gap negative vectors.
- [ ] QSR-C: verified root/grant inputs and forged/scope-swapped signatures.
- [ ] QSR-D: exact immutable trie traversal against an independently generated small map.
- [ ] QSR-E: bounded cursors with interrupted/resumed traversal equivalence.
- [ ] QSR-F: sorted semantic posting intersection against a slow relational oracle.
- [ ] QSR-G: live storage source and authenticated publication.
- [ ] QSR-H: writer handover and malicious/partitioned provider tests.

Accept QSR-D before adding distributed providers; accept QSR-F before claiming semantic lookup; accept QSR-H before live repartitioning.

## 2. QSR publication, hot keys and handover

A directory writer is not automatically authorised to invent a source controller's record. Verify the source signature and authority first, then compile/publish the scoped index. Commit source snapshot, compiler/profile and output root together.

Hot exact keys need replica placement, request coalescing and cache admission. Splitting a key prefix cannot spread the work for one identical key. Large postings can partition on stable full record IDs under a secondary cover. Use acyclic commitments: the secondary structure binds stable parent context; the primary leaf commits its secondary root; the checkpoint commits the primary root.

### Writer state machine

| State | Permitted operation | Durable condition before transition |
|---|---|---|
| ActiveOld | Admit mutations with monotonic sequence and operation identity | Every acknowledged mutation is durable |
| Fencing | Stop new write admission; finish already admitted work | Fence epoch and final accepted sequence committed |
| Draining | Finish/resolve in-flight transactions | No acknowledged mutation lies beyond the final durable boundary |
| Transferring | Copy immutable state and required tail to successor | Each transferred range verifies against the final root |
| ReadyNew | Successor proves exact root/boundary and required dependencies | Readiness receipt and reserved resources committed |
| ActiveNew | Publish/activate new authorised epoch | Single-writer authority and new endpoint binding committed |
| RetiredOld | Serve allowed immutable history/redirects | Cannot accept new writes under retired epoch |

If the old owner disappears before a trustworthy final boundary is established, do not declare a complete transfer. Remain unavailable for writes or use a separately specified recovery/consensus protocol. Replicas and two signatures do not automatically solve that problem.

- [ ] HANDOVER-A: model the transitions and prohibit two active writers.
- [ ] HANDOVER-B: implement core transaction fences and restart at every boundary.
- [ ] HANDOVER-C: transfer real ranges while maintaining old/new memory reservations.
- [ ] HANDOVER-D: kill either process before/after every acknowledgement; prove no accepted mutation is lost and no retired owner can write.
- [ ] HANDOVER-E: measure temporary storage, network traffic and read/write unavailability.

## 3. Constraint-first multipath routing

The algorithm is deliberately two-stage: **feasibility first, optimisation second**. Privacy/authority is never exchanged for a cheaper path.

### Inputs

- Authenticated topology snapshot with adjacency, origin, scope, sequence, validity and withdrawal.
- Request target plus an execution permit specifying allowed realms/providers, handling profile, required deadline/reliability and any hard resource limit.
- Measured or explicitly estimated path metrics with uncertainty and provenance.
- Caller-owned bounded candidate/path storage, work budget and stable tie-break order.

Do not compare unlike compute types as one scalar. Unknown energy is not zero energy; if a hard energy constraint requires a bound, an unknown path is infeasible until a permitted conservative bound is available.

### Algorithm

1. Validate/expire topology changes and update a caller-backed adjacency index.
2. Remove edges/realms that violate hard authority, reachability or profile constraints.
3. Produce a bounded set of simple candidate paths with the selected qualified shortest-path/alternative-path algorithm. Do not enumerate all paths.
4. Accumulate checked metrics under their declared semantics. Additive upper bounds may be summed; p95 latencies cannot simply be summed and called an end-to-end p95.
5. Discard paths violating hard deadlines/budgets.
6. Pareto-filter comparable candidates: remove B only when A is no worse on every selected comparable metric and strictly better on at least one.
7. Select at most three candidates with configured failure-domain diversity and deterministic tie breaks.
8. Keep the current path unless it becomes infeasible or a replacement meets a reviewed hysteresis threshold.
9. Let QSession schedule traffic using validated per-path congestion state; routing does not create unlimited transmission credit.

A route advertisement proves only the authorised statement it contains. Live reachability still requires topology/path validation, and a “cheap” resource advertisement does not prove measured cost.

### Update and failure behaviour

Bound recomputation through dirty-region queues and continuation. While rebuilding, retain only still-valid safe routes; mark unresolved destinations explicitly unavailable. Prevent stale worker results from overwriting a newer topology generation.

- [ ] ROUTE-A: dynamic admitted adjacency index and exact 16/17-node boundary.
- [ ] ROUTE-B: deterministic single-path oracle equivalence, including zero/equal weights and disconnected nodes.
- [ ] ROUTE-C: hard-constraint filter tests where cheapest paths are forbidden.
- [ ] ROUTE-D: bounded alternative/Pareto selection with independent small-graph exhaustive oracle used only in tests.
- [ ] ROUTE-E: hysteresis and failover under alternating metrics and adversarial advertisements.
- [ ] ROUTE-F: protected multi-hop QPR traffic through the selected paths with no bypass and measured convergence.

For large graphs, the test oracle can use slower heap-backed structures outside Tier-1. Production traversal remains caller-buffered and bounded.

## 4. Multipath transport without unsafe congestion shortcuts

Implement a correct single-path controller first. The [transport reference](https://www.rfc-editor.org/rfc/rfc9002.html) supplies a useful reviewed baseline; any alternate controller requires its own specification and comparison.

Each path owns sent-packet history, RTT estimates, validated ECN state, congestion window, pacing deadline and loss state. The connection owns stream ordering, connection flow control, authorised peer identity and aggregate resource reservations.

The packet-number/nonce design must remain globally unique for each key or use separately derived path keys with a reviewed domain-separated schedule. Do not initialise every path's packet counter to zero under the same traffic key.

Scheduling procedure:

1. Find paths with current authenticated validation and the required profile.
2. Compute send eligibility from pacing, congestion and both stream/connection flow credit.
3. Select eligible frames according to bounded control/interactive/bulk fairness and deadline policy.
4. Select an eligible path by the qualified scheduler; charge aggregate bottleneck policy so extra paths do not multiply entitlement.
5. Allocate a fresh protected packet number, seal frames and commit ownership to the bearer.
6. On send failure, release/requeue according to the driver contract without reusing a nonce or advancing application delivery.
7. Process authenticated ACK/loss events; retire sent ownership exactly once.
8. During migration, validate new path and generation before using it; retain bounded old-path/key state for accepted overlap.

- [ ] TRANSPORT-A: bytes-in-flight conservation on ACK/loss/cancel/retry.
- [ ] TRANSPORT-B: pacing and normal reordering without pathological timeout.
- [ ] TRANSPORT-C: independent stream progress with a stalled consumer.
- [ ] TRANSPORT-D: stale activation, forged ACKs, path restart and key-overlap negatives.
- [ ] TRANSPORT-E: shared-bottleneck fairness and multipath advantage on genuinely independent paths.

## 5. Finite obligation transactions

Reuse the core transaction owner. Model **one obligation per agreed target**, with stable identity across provider/cell/version changes.

### Durable records

- Obligation: ID, contributors/accepted costs, target T, agreed return policy, prior funding accounting, state and revision.
- Usage quote: operation ID, acting-capacity classification, exemption evidence, O/F/A split, validity and recipient/provider.
- Hold: unique hold ID, obligation/operation, amount, deadline, owner and finality policy.
- Settlement: rail-specific event identity, verified finality, amount, hold/operation and effect digest.
- Discharge: authorised non-cash amount and provenance.
- Payout claim: entitlement to collected funds; separate from usage collection.

### Reserve transaction

```text
begin transaction at obligation revision
verify operation authority and current classification
if exempt: recovery = 0
else if fulfilled: recovery = 0
else:
    available = checked(T - S - W - H)
    recovery = min(accepted_quote_recovery, available)
    require recovery > 0 if a recovery-charged event is accepted
check operating/service terms separately
insert unique operation/hold and increase H by recovery
commit hold + usage decision + receipt together
```

If available recovery is zero because of live holds but the obligation is not fulfilled, do not claim fulfilment. Follow the accepted policy: wait, proceed without a recovery charge if allowed, or decline that paid event. Never make personal/humanitarian access contingent on a corporate payment race.

### Finalise transaction

1. Verify the payment event through the selected settlement adapter and match operation/hold/currency/unit.
2. Reject or idempotently return the prior receipt for an already applied event.
3. Require the hold still owns the allocated recovery; subtract its amount from H and add only finalised recovery to S.
4. Mark fulfilled only when S + W reaches the agreed T under exact integer arithmetic and policy.
5. Commit settlement, obligation revision and receipt atomically.
6. Allocate payouts through separate recorded entitlements; the same settlement cannot create multiple collection credits.

A final payment arriving after hold expiry/reallocation must not simply increment S. Apply the declared late-payment policy, such as refund/unapplied credit or a new atomically reserved residual allocation. Record disputes/chargebacks explicitly; a finality reversal must follow the agreement and cannot silently resurrect unlimited creation debt.

For genuinely disconnected collection, preallocate finite signed allotments in H before disconnection. An offline node spends only its allotment; reconnect reconciles events idempotently. Autonomous unbounded offline collection and a globally exact cap cannot both be promised.

- [ ] ECON-A: state/amount types with checked integer arithmetic and explicit units.
- [ ] ECON-B: acting-capacity/exemption evaluation independent of payment status.
- [ ] ECON-C: reserve/finalise/cancel/expire transactions and duplicate events.
- [ ] ECON-D: two providers racing on the last recovery amount.
- [ ] ECON-E: crash, delayed finality, chargeback and disconnected allotment scenarios.
- [ ] ECON-F: fulfilled obligation plus restricted clinical data retains all privacy checks.

## 6. Selective evidence promotion

Use a transaction that binds the selected original bytes to a preservation intent and holds before volatile source deletion.

1. Select incident/obligation records under an authorised capture policy and byte budget.
2. Identify exact originals, source authority, context, countervailing records and interpretation dependencies.
3. Reserve durable storage; if unavailable, return an explicit preservation gap, not an empty success receipt.
4. Copy/retain the originals through the core artifact owner and verify the committed bytes.
5. Atomically commit manifest, labels, provenance, custody and applicable holds.
6. Only then permit volatile-source retirement according to its policy.
7. Examiners receive scoped exports with verification material and declared omissions.
8. Hold release and retention GC run through the same durable owner; concurrent holds survive individual release.

- [ ] EVIDENCE-A: promotion ordering and preservation gaps.
- [ ] EVIDENCE-B: overlapping holds and expiry races.
- [ ] EVIDENCE-C: restart/torn-write and exact-byte verification.
- [ ] EVIDENCE-D: sealed examiner export with incomplete-chain reporting.
- [ ] EVIDENCE-E: long-term key/algorithm renewal preserving original records.

## 7. Qualification boundary for advanced cryptography

The existing [post-quantum design](../post-quantum-security.md) defines the selected hybrid goals. Junior implementers must use its reviewed versioned composition plus independent vectors; correcting an unkeyed Finished must not become an invitation to invent a new handshake.

For asynchronous medical envelopes, explicitly specify forward-secrecy limits when long-lived recipient keys are used, sender authentication, prekey consumption/replay behaviour where selected, recipient-set changes and recovery. A forward-secret live session does not automatically make stored mail forward-secret.

For protected groups, specify membership/epoch state, concurrent updates, removed-member access, offline reconciliation and message-key deletion. A group protocol with classical components cannot be advertised as fully post-quantum because its enclosing transport is hybrid.

For private biometric computation, specify the exact function, arithmetic encoding, party/collusion model and permitted leakage before choosing a library. Implement a plaintext test oracle, then verify the selected private construction against it and adversarial inputs. No claim extends beyond the qualified function.

- [ ] CRYPTO-ADV-A: selected construction/profile decision with expert review and explicit unsupported cases.
- [ ] CRYPTO-ADV-B: versioned messages, commitments, key lifetimes and independent vectors.
- [ ] CRYPTO-ADV-C: complete error/replay/recovery state machine.
- [ ] CRYPTO-ADV-D: measured memory/work/energy and real caller integration.
- [ ] CRYPTO-ADV-E: independent security qualification before enabling the capability.

These review gates are implementation tasks with required outputs. They are not product mocks or completed guarantees. Unqualified advanced capabilities remain unselectable; the qualified profile continues to operate where its threat model is sufficient.
