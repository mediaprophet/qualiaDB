# Core Reuse, Webizen and Storage

Implement in existing core owners. The [source audit](../../core-memory-and-parallel-networking.md) is a starting snapshot, not a permanent assertion that code has not changed.

All packages follow [library boundaries](../library-layout.md), [swarm handoffs](../swarm-protocol.md)
and the [shared validation standard](../validation-matrix.md). Package dependencies are authoritative
in the register; early interface-based work cannot bypass their completion evidence.

## CORE-01 — Caller-owned Webizen and scoped caches

**Owner and inputs:** Core/Webizen owner; FND baseline and semantic contracts.

**Construction boundary:** Extract arena backing, scoped cache, rule activation and VM adapter into focused files under existing governance/webizen; preserve public compatibility through routing exports.

- [ ] CORE-01.01 Inventory arena constructors and consumers; replace uncontrolled per-call creation with explicitly admitted reusable caller storage.
- [ ] CORE-01.02 Preserve the 48-byte Quin ABI and distinguish 42 MiB slot backing, complete pass working set and encompassing cell budget.
- [ ] CORE-01.03 Define arena reset, reuse, lease lifetime and concurrent ownership; prevent stale data leaking between scopes or jobs.
- [ ] CORE-01.04 Replace Tier-1 vector-backed construction and mutable growth with caller-owned buffers, bounded rule slices and explicit capacity errors.
- [ ] CORE-01.05 Bound rule staging/activation and compiler scratch; reserve before changing live rule state.
- [ ] CORE-01.06 Audit bounded recursion and evaluator loops; use iterative bounded state where the project requires nonrecursive execution.
- [ ] CORE-01.07 Replace context-free authorization cache use with full scope/policy/source-generation bindings or proven isolation and current-authority checks.
- [ ] CORE-01.08 Specify hash-slot collision/eviction behavior; preserve recomputation/continuation instead of inferring absence or complete retained facts.
- [ ] CORE-01.09 Distinguish the recent-slot ring from durable storage and packet queues; no evidence/replay/obligation state may depend on lossy retention.
- [ ] CORE-01.10 Classify four-field versus five-field parity callers/fixtures, agree canonical behavior and implement tested compatibility/migration without dual-checksum guessing.
- [ ] CORE-01.11 Audit empty entries, output exhaustion, wildcard queries and cache replay against false allow decisions.
- [ ] CORE-01.12 Measure all hot construction/evaluation/error/cancel paths and include referenced pages, stacks and scratch in the admitted pass bound.
- [ ] CORE-01.13 Verify parallel leases, scope changes, revocation and generation reclamation with the independent harness.
- [ ] CORE-01.14 Publish a caller migration map, exact allocation evidence and remaining unsupported evaluator profiles.

Exit: Webizen callers have explicit bounded storage and scoped semantics. Handoff verified arena/VM interfaces to runtime and semantic compilers.

## CORE-02 — Bounded access to large logical graphs

**Owner and inputs:** Core query/storage owner; coordinate existing range/SPARQL maintainers.

**Construction boundary:** Separate query quanta, cursors, segment metadata admission, fallback policy and executor adapters in existing directories; reuse real joins and sort machinery.

- [ ] CORE-02.01 Record physical versus logical/decoded graph size and index/manifest overhead; do not admit full loads from a small root file's size.
- [ ] CORE-02.02 Route network reads through caller-buffered Q42 block/range/volume-set APIs with immutable source identities.
- [ ] CORE-02.03 Add scan-block/work/time quanta and resumable progress for sparse or absent matches; output page size alone is not a work limit.
- [ ] CORE-02.04 Bind continuations to exact query/source/profile generations; reject stale, forged and cross-scope cursors.
- [ ] CORE-02.05 Bound manifest decoding, segment handles, lexicon/index pages and constructor/error allocations independently of payload windows.
- [ ] CORE-02.06 Eliminate unbudgeted whole-volume fallback and read-before-capacity-check paths from selected network/query execution.
- [ ] CORE-02.07 Verify cross-segment scans and joins, hash-table exhaustion and nested-loop fallback costs; report unsupported or incomplete results honestly.
- [ ] CORE-02.08 Define partitioned intermediate/spill support only for selected algorithms; include disk bytes, merge fan-in, lifetime and crash cleanup.
- [ ] CORE-02.09 Bring external sort buffers and metadata under admitted byte/work budgets; existing million-Quin allocation exceeds 42 MiB before overhead.
- [ ] CORE-02.10 Handle global constraints and closure with explicit frontier/continuation; no recent-fact window may certify global completeness.
- [ ] CORE-02.11 Validate integer/range overflow, truncated blocks, corrupt indexes, decompression expansion and mutation/truncation during borrowed reads.
- [ ] CORE-02.12 Test a persisted logical dataset larger than RAM against an oracle, including empty matches and cross-partition dependencies.
- [ ] CORE-02.13 Measure startup metadata, peak working memory, scan work, page faults, useful results and continuation cost separately.
- [ ] CORE-02.14 Publish supported operators/fallbacks and their completeness/latency limits for SEM, SVC and enterprise-cell owners.

Exit: selected large-graph operations have bounded memory and resumable work, with verified results. Arbitrary complete ontology reasoning is not inferred from file streaming.

## CORE-03 — Atomic core artifacts and durable effects

**Owner and inputs:** Storage owner; reviewed semantic records and bounded access.

**Construction boundary:** Focused intent, publication, recovery, reference and GC components under existing artifact/WAL owners; explicit backend adapters instead of an independent database.

- [ ] CORE-03.01 Define exact-object storage and typed digests for contracts, proofs, operations, receipts and evidence; preserve original signed bytes.
- [ ] CORE-03.02 Specify transaction IDs, writer serialization, commit points and ordering with authority withdrawal.
- [ ] CORE-03.03 Atomically bind source/projection publication, replay acceptance, application mutation, reservations and durable outcome where required.
- [ ] CORE-03.04 Write bounded intents/segments, sync required data, publish roots/markers and specify platform-specific durability/failure outcomes.
- [ ] CORE-03.05 Keep content dependencies acyclic; join sealed Q42 and selected exact artifacts through external commit manifests.
- [ ] CORE-03.06 Implement bounded WAL/checkpoint recovery, pending intent reconciliation and deterministic replay without unbounded vectors.
- [ ] CORE-03.07 Keep external effects separate: durable submission intent before dispatch, stable identity after ambiguous outcome, no local-commit claim of remote atomicity.
- [ ] CORE-03.08 Protect old/new generation leases and pending effects until definitive release; bound retention and backpressure on stalled consumers.
- [ ] CORE-03.09 Serialize hold/promotion/reference acquisition with GC commit, including protected dependency discovery and future matching arrivals.
- [ ] CORE-03.10 Require preserved remote dependency acknowledgment or local copies; a local root pin cannot guarantee remote custody.
- [ ] CORE-03.11 Scope deduplication, encryption, index disclosure and access; identical bytes do not unify unrelated private authorities.
- [ ] CORE-03.12 Define replica/backup root and hold propagation, corruption handling and restoration independent of the live network.
- [ ] CORE-03.13 Inject crashes/torn writes/concurrent revoke/GC at every durable boundary; verify no duplicate effect or lost protected record.
- [ ] CORE-03.14 Publish backend durability classes, exact transaction traces, errors, recovery limits and storage/key retention ownership.

Exit: durable guarantees are demonstrated on each claimed backend; handoff transaction/hold interfaces to services, economics, evidence and optional QNF.

## CORE-04 — QNF comparison and selected artifact representation

**Owner and inputs:** Storage format owner with crypto/semantic reviewers.

**Construction boundary:** A reviewed representation decision and the corresponding implementation/fixtures. QNF lives in focused container_qnf modules only if selected.

- [ ] CORE-04.01 Compare existing exact core artifacts/Q42 extensions with the QNF candidate on actual route, contract, PQ-proof and evidence workloads.
- [ ] CORE-04.02 Measure full index/proof overhead, range-read work, copies, allocations, resident memory, cold/warm latency and disk cost.
- [ ] CORE-04.03 Select representation by semantics and demonstrated benefit; record why large graph size alone does not decide the format.
- [ ] CORE-04.04 If retaining existing artifacts, implement missing byte-preserving bounded access and publication guarantees and record QNF as unselected.
- [ ] CORE-04.05 If selecting QNF, freeze explicit header/directory/chunk/object/profile bytes only after review; no inherited numeric layout is automatically final.
- [ ] CORE-04.06 Validate repeated scope/profile bindings, signature-length/hash-cycle avoidance, exact-file versus generation digests and source-signature independence.
- [ ] CORE-04.07 Implement authenticated bounded range access, partial/full validation states, reusable verification under pinned immutable identity and explicit index-scan work.
- [ ] CORE-04.08 Test overflow, reserved/unknown values, duplicate/overlapping ranges, endian/alignment, malformed counts, whole coverage and file mutation.
- [ ] CORE-04.09 Preserve acyclic cross-artifact references, authority invalidation and hold-aware reclamation through CORE-03, whichever representation is selected.
- [ ] CORE-04.10 Provide independent round-trip/corruption vectors and migration/recovery results; record conditional checks as satisfied by explicit decision evidence, never silently skipped.

Exit: a selected, justified representation meets the shared contract. Downstream services remain representation-neutral; no separate ledger or engine is introduced.
