# QSR Evaluation, Worked Traces and Implementation Acceptance

**Status:** Experiment design, 2026-09-07. No performance result has been measured.
Read the [QSR algorithm](./qualia-scoped-rendezvous.md) for normative proposed behavior.

## 1. What “better” means

QSR targets governed semantic lookup with repeated queries, large core-backed indexes, contextual
privacy, costly PQ verification and multiple bounded cells. Evaluate useful authorized results per
unit of admitted work, as well as latency, availability and freshness. Lower latency caused by
omitting policy checks, returning fewer relevant records or accepting stale data is not a win.

| Workload | Proposed advantage to test | Plausible disadvantage |
|---|---|---|
| Repeated scoped semantic queries | Reuse compiled plans and immutable cover/source proofs; fetch fewer irrelevant records | Kademlia can also cache answers; invalidation and policy changes may erase the benefit |
| Multi-condition provider discovery | Selective posting anchor and core residual evaluation reduce network candidate transfer | Building/maintaining several facet indexes consumes storage and update work |
| Large ontologies / datasets | Core range cursors and partition leases avoid loading the entire index | Broad queries still need scans, continuations and many partitions |
| Related work in one community | Scope locality and stable provider cohorts reduce discovery overhead | Scope epoch authority is an availability/censorship dependency |
| Enterprise routing/index nodes | Explicit partitions scale across admitted cells and serving replicas | Hot exact keys require replication; cell/host overhead may outweigh parallelism |
| Cold uniformly random exact keys | Direct cover lookup may help when a valid directory is already nearby | Bootstrap, cover validation and PQ proofs may cost more than Kademlia |
| Frequent churn / partition | Pinned snapshots expose stale/incomplete outcomes explicitly | Safe publication and handover can reduce write availability |

The algorithm is selected as the purpose-designed target. A production claim of superiority requires
evidence for named workloads and budgets; a failed comparison triggers tuning or a reviewed design
decision, not concealed fallback to Kademlia.

## 2. Prior art and attribution

Kademlia already combines XOR routing with parallel asynchronous lookup, replication and caching.
QSR cannot claim novelty for parallel requests or caching, and its baseline must retain these
capabilities. [Original Kademlia paper](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf).

Distributed ordered indexes have long supported richer searches than an exact-key hash table.
Skip graphs are relevant prior art for distributed key ordering and fault handling. QSR instead
proposes authority-scoped semantic lanes and leased authenticated prefix coverage; this is not a
claim to have invented distributed range indexing.
[Skip graphs, Aspnes and Shah](https://www.cs.yale.edu/homes/aspnes/papers/skip-graphs-abstract.html).

Authenticated denial has privacy costs: hashing names is not a blanket protection against
enumeration. QSR's private lookup tokens and absence evidence therefore require an explicit
observer/leakage analysis, including guessing by authorized key holders.
[NSEC3 guidance, RFC 9276](https://www.rfc-editor.org/rfc/rfc9276.html).

These references inform the design; they do not establish QSR's performance, prove originality of
the whole combination or certify its security. A broader prior-art review accompanies any formal
novelty claim.

## 3. Matched baselines

Use two complementary experiments:

1. **Algorithm isolation:** independent QSR and Kademlia lookup implementations over the same
   QRoute/QSession transport, PQ verifier, source records, hardware, resource governor, replication
   budget and scope/authority checks. Keep Kademlia's asynchronous concurrency and cache behavior.
   This separates lookup/index decisions from improvements due to replacing the surrounding stack.
2. **Application comparison:** QSR versus a documented optimized Kademlia-backed service performing
   the same semantic provider query, with equivalent indexing, freshness, authorization and result
   requirements. Compare the actual current libp2p application separately and disclose every
   transport/crypto/feature difference; that comparison alone cannot isolate the lookup algorithm.

A one-RPC directory lookup is not one physical hop. Charge bootstrap, manifest fetching, QRoute
discovery, session establishment, proof chains, index maintenance, replication, invalidation and
handover. Test equal cache byte budgets and both warm and cold states. Include a Kademlia baseline
with the same semantic materialization opportunities, not one forced to scan every record.

## 4. Reproducible experiments

Use deterministic generators plus appropriately authorized representative graph snapshots.
Record schema, ontology bundle, source distribution, facet selectivity/correlation, update rate,
scope membership, graph size and the exact expected query result at each named snapshot.

Vary:

- Exact-target versus conjunction/disjunction/residual queries; absent and ambiguous targets.
- Uniform and skewed popularity, including a single hot key and a very broad facet.
- Freshness/withdrawal frequency, membership/key rotation, directory cold starts and unavailable roots.
- Independent versus correlated provider failure; dishonest omissions; signed root equivocation.
- Dataset sizes smaller than a cell, larger than host RAM and multi-volume datasets.
- 1, 2, 4 and more cells on admitting hardware; equal aggregate budgets for every comparison.
- Loss, delay, reordering, outages, cell crashes, partial storage failure and cancelled queries.
- Tight and generous work/byte deadlines, including query plans that cannot finish within the cap.

Measure distributions of useful query latency, lookup/physical-hop bytes, completed coverage,
freshness, relevant-result recall against the closed oracle, CPU/work, proof jobs, peak memory,
storage amplification, update visibility and recovery delay. Measure energy with a real meter
or report an identified estimate/unknown. Do not infer joules from arbitrary FLOP ratios.

Choose sample sizes, acceptance thresholds and permitted regression budgets before running.
Retain raw measurements, seeds, configuration and confidence/uncertainty analysis. Report timeouts
and failed/incomplete queries in availability results, not merely omit them from latency charts.
Proposed release decision: no authority/completeness/budget regression, plus reproducible useful-work
gains in the selected target workloads and disclosed cold/random/churn tradeoffs. The deployment
owner chooses numerical performance budgets using its hardware and service requirements.

## 5. Worked state traces

These are specified cases for independent tests, not executed simulator results.

| Trace | Initial state and events | Required outcome |
|---|---|---|
| Cold bootstrap | Invitation binds scope S, authorized writer A, epoch 7/root R7 and directly usable DNIs. Client has no QSR cache; first replica times out, second supplies a root-bound cover page. | Debit both attempts under one reservation; validate invitation/current authority and R7 before following its prefix. If bootstrap cannot be authenticated/reached, return ScopeUnavailable; no recursive QSR or DNS bootstrap. |
| Conflicting roots | Client pins S/7/R7. An authorized A-signed S/7/R7b arrives with a different digest. | Preserve bounded equivocation evidence, mark conflict and stop automatic root selection. A larger unvalidated epoch or another alias for A cannot cure the fork. Resume only through the scope's accepted conflict/authority-transition policy. |
| Deletion during pagination | Query reads posting snapshot P10/page1. Publisher's withdrawal becomes visible in P11 while page2 of P10 is in flight. | Cursor continues only P10; result states P10 coverage. A now-known withdrawal rejects affected live use even if P10 contains the row. No page mixing or unqualified latest-state/absence claim. |
| Omitted posting | A valid source belongs in facet F, but a provider supplies a Merkle-consistent F index that excludes it. | Merkle validity alone cannot establish semantic completeness. Require the selected independently checked source-to-index commitment or label the result as an authorized publisher assertion, not verified exhaustive coverage. |
| Hot exact key | All requests target X. Prefix split would leave X in one child; immutable posting list also exceeds one page. | Replicate serving pages and distribute admitted reads. Partition the posting list by stable full record identity under a secondary authenticated cover; broad lookup keeps explicit unsearched partitions. Do not pretend prefix splitting X distributes those requests. |
| Migration failure | New cohort has copied P20; old admission is fenced and drained at durable boundary B21, but final delta/ready receipt is missing. Epoch writer is unavailable. | Do not activate a successor or release old storage. Retain all accepted writes through B21; no old writer acknowledges beyond it. Serve allowed pinned old snapshots, reject new writes or return unavailable/incomplete. Recover under the same operation; do not silently unfreeze the old generation. |
| Partial OR | Query requires F1 OR F2; all F1 intervals searched, F2 provider unavailable. | Return known verified matches with incomplete F2 coverage; never EmptyInSnapshot or complete top-k. |
| Private epoch rotation | Removed member knows old token key. New epoch/index key is activated while old queries remain. | No new disclosures under old authority. Old-token copies cannot be made secret retroactively; explicitly drain/reject continuations, migrate authorized postings and account for old/new overlap. |
| Exhausted frontier | A broad facet requires 80 partitions; 32 descriptor slots are available and checkpoint storage is exhausted. | Return incomplete with a safe restart/coverage description within the remaining reply budget; never drop 48 intervals and declare completeness. |
| Unknown energy | Provider quotes estimated work but has no attributable energy meter. | Preserve unknown joules, apply the accepted uncertainty/funding policy and enforce known resource caps; a zero energy charge is not evidence of zero consumption. |

## 6. Properties and proof obligations

| Property | Required argument or test |
|---|---|
| Authority safety | Every delivered live candidate passes full source/controller and current purpose/operation checks; placement, payment and cached handles cannot bypass them |
| Cover integrity | Children form a complete nonoverlapping parent interval partition under a pinned root; compressed-prefix and malformed-child cases rejected |
| Index completeness | Source snapshot, indexing policy/compiler and output root are bound, with the declared assertion or independent-reconstruction trust level |
| Termination/yield | Prefix progress within fixed root; monotonic cursors; finite retries/redirects/scopes; every iteration charged, every cap explicit |
| Resource conservation | All branches/providers/cells inherit one reservation lineage; concurrency, retries and cold proof work cannot mint credit |
| Recovery | No source/posting/receipt falsely complete after injected crashes; old/new authority and retained evidence survive handover consistently |
| Privacy | Explicit observations for router, directory, provider, scope member and revoked member; no unauthorized proof/absence enumeration |
| Performance | Measured gains under matched semantics and cache/transport budgets; no claim from operation-count examples alone |

For a fixed key within one trie, complete binary/nibble descent has a finite key-width bound; it is not
automatically O(log N) in provider count. Under balanced occupancy, path compression and selective
lanes, observed depth may be much smaller. Semantic queries can require work proportional to all
relevant partitions and matching/filtered postings. State these worst cases in operator limits.
Lane selection, primary partition and optional secondary posting traversal have separate depth
bounds and one shared operation budget; treating 96 digits as a bound for all stages is invalid.

A warm valid cover can select an eligible cohort without searching hash-neighbor chains, but a
warm Kademlia client can also know the target. Compare actual workload distributions and proof
cache lifetimes. No theorem here demonstrates lower latency, energy or Byzantine failure rate.

## 7. Integration and completion

NET-04 owns algorithm construction and the [additional numbered checks](./qdnf-imp/workstreams/03-native-network.md).
FND-02/03 and SEM-01 freeze meaning/interfaces, CORE owns index/proof/storage primitives,
RT owns admitted execution, and QA-02 owns independent system comparisons. Domains review their
own authority, resource, crypto and evidence boundaries.

The initial profile does not import Kademlia/Yamux or require a new Q42/QNF ABI. An unavailable QSR
scope produces an explicit result. Any later algorithm substitution requires a versioned design
decision, consumer review and fresh comparison; it cannot happen as an undocumented fallback.
