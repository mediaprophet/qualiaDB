# QDNF on the QualiaDB Core and Q42 Storage

**Status:** Normative design 0.1; integration requirements, not a completed network cache
**Date:** 2026-09-06

## 1. Core reuse is an architectural requirement

QDNF MUST use the QualiaDB core as its semantic storage, indexing, and policy substrate. Network
libraries implement protocol state and scoped adapters onto that core. Persistent route evidence,
ontological contracts, semantic bundles, contribution records, and settlement history use `.q42`
volumes and the existing core's storage lifecycle, extended where the required guarantees are missing.
QDNF does not introduce an independent general-purpose database, ontology store, or payment ledger.

Persistent source records, derived semantic indexes, and live protocol state have different lifetimes:

| State | Owning representation | Lifetime and access |
|---|---|---|
| Signed RARs, withdrawals, delegations, constitutions | Exact signed source bytes plus NQuin projections in Q42-managed storage | Retain according to authority/evidence policy; expiry still gates use |
| CBOR-LD agreements, contexts, ontologies, shapes, rules, compression tables | Content-digested semantic bundles and records referenced by `.q42` manifests/indexes | Pin the versions accepted by each contract; acquire and validate on cold paths |
| Contributions, spend reservations/intents, receipts and adjustments | Core durable mutation/checkpoint path plus exact signed record objects | Survive restart; preserve outstanding replay and settlement evidence |
| Active route/resolution selections and compiled policy | Bounded, immutable derived generations of typed handles | Rebuild from verified core records; invalidate on expiry, withdrawal, or policy change |
| Session keys, packet replay windows, ACK ranges, in-flight buffers | Caller-owned fixed arenas and purpose-separated key storage | Short-lived, with explicit teardown; private keys never enter Q42 artifacts |
| Legacy DNS/HTTP cache | Separate LIG namespace/principal using the same core facilities where useful | Distinct lookup, trust, expiry, and disclosure semantics from native records |

This is physical infrastructure reuse with explicit logical separation. Sharing a storage library
does not permit one relationship, service, or gateway to read another's private records.

## 2. Cloudflare comparison

Cloudflare's [DNS-cache engineering article](https://blog.cloudflare.com/dns-cache-memory-optimization-1111/),
published 27 August 2026, describes removing unused container capacity, consolidating record lists
with offsets, avoiding repeated owner names, and replacing bulky record variants with contiguous
encoded data. Reusable scratch buffers reduce construction allocations. Its final layout retains
structured metadata beside encoded record bodies. It reports both cache benchmarks and separate
production resident-memory measurements.

The QDNF design inference is to build on QualiaDB's compact records, shared term storage, indexed
blocks, and caller-buffered reads. The article is engineering prior art; it neither establishes
QualiaDB equivalence nor supplies a QDNF benchmark. Its allocation-based implementation does not
relax QualiaDB's stricter zero-heap execution contract, and its DNS authority model is not imported
into native resolution.

## 3. Existing storage primitives and their limits

| Repository anchor | Verified primitive | Integration boundary |
|---|---|---|
| [Q42Volume](../../../../crates/qualia-core-db/src/q42/q42_volume.rs) | Native memory mapping, checked volume structure, lexicon/index views, caller-buffered SuperBlock reads | Compressed blocks require decoding; mmap is not proof of zero resident memory or signature validity |
| [Q42LexMmap](../../../../crates/qualia-core-db/src/q42_lex.rs) | Compact persisted term lookup | Preserve full terms and exact codec mappings; a term hash is not cryptographic equality |
| [BIDX access](../../../../crates/qualia-core-db/src/q42/volume/index.rs) and [field postings](../../../../crates/qualia-core-db/src/q42/volume/postings.rs) | Checked block ranges and compact S/P/C candidate pruning, with Bloom fallback | Candidate membership does not establish exact identity, current permission, or absence across all providers |
| [Block cursor](../../../../crates/qualia-core-db/src/q42/volume/cursor.rs), [range volumes](../../../../crates/qualia-core-db/src/q42/volume/range_volume.rs), [query mode](../../../../crates/qualia-core-db/src/q42/volume/query_mode.rs) | Caller-buffered iteration and range queries; a 4 MiB resident-query selection threshold | QDNF must choose and enforce bounded paths, rather than calling compatibility whole-volume loaders |
| [Streaming writer](../../../../crates/qualia-core-db/src/q42/volume/stream_writer.rs) and [root publication](../../../../crates/qualia-core-db/src/q42/volume/publish.rs) | Block/segment production and root-manifest generation replacement | Need protocol-specific transaction, durability, concurrency, and crash-recovery guarantees |
| [WriteAheadLog](../../../../crates/qualia-core-db/src/wal.rs) | Native `append_mutation` uses `sync_all`; checkpoints link to DAG history | Current `recover` reads the WAL into vectors; this is not yet bounded multi-record settlement recovery |

The core's [revision-cached graph index](../../../../crates/qualia-core-db/src/query/graph_index.rs)
is another reuse point, but its current snapshot-copy behavior is not a ready-made bounded network
cache. [MmapStore](../../../../crates/qualia-core-db/src/storage/mmap.rs) also resets its active count
on open; merely choosing a mapped store does not establish durable record recovery. These are
integration obligations, not reasons to build a second engine.

## 4. Exact records and compact projections

Store an accepted source record once per authorized storage scope, retaining its signed bytes and
full strong digest. Project its queryable facts into canonical 48-byte NQuins. A Quin may index
record kind, target, issuer, context, validity, policy version, or an external record handle; it
does not hold an entire signature, ontology, invoice, or network packet.

The storage adapter MUST bind each handle to object kind, full digest, byte length, and immutable
volume/segment generation. Validate range arithmetic, alignment, bounds, and generation before use.
Never persist native pointers. Reusing an offset after compaction must not make an old handle refer
to a different record. Preserve full identifiers beside compact indexes at security boundaries.

Exact CBOR-LD/COSE bytes require a byte-preserving artifact representation. Reuse a suitable existing
core artifact facility only after proving exact round-trip behavior. If unified Q42 lacks the needed
opaque-record representation, add a versioned payload/manifest profile in the owning Q42 library,
with compatibility and bounds tests. Do not discard the signed source by keeping only hashed Quins,
or reinterpret an unrelated tensor/KV page type as a contract container.

Native mmap, constrained range reads, and browser storage can implement the same bounded record
access contract. Decode or copy only the required block into caller-owned scratch. Carry validated
serialized record bodies directly where the selected wire profile permits; still apply current
authorization, freshness, and new session encryption. Never reuse cached session ciphertext/nonces.

## 5. Cache keys, freshness, and publication

QResolve caches MUST include the target's full digest and the relevant requester/disclosure scope,
operation, sensitivity, method/profile, and policy generation. A source-specific negative cache
additionally identifies the queried source. Never key an authorized result solely by a DID hash,
human alias, or content digest. Two requesters can lawfully receive different answers for one target.

Object storage and cached authorization are separate. Identical immutable bytes may share storage
within an authorized scope, but visibility, expiry, and policy decisions remain independent. Avoid
cross-private-scope deduplication where existence or timing would disclose protected relationships.

Lookup proceeds through:

```text
scoped query -> core index candidate ranges -> bounded record access
  -> exact identifier/digest and verification-evidence checks
  -> expiry/withdrawal/block/policy-generation checks -> current authorized result
```

Keep immutable signed expiry separate from local eviction deadlines. A cache hit never extends a
signature, capability, route, or quote lifetime. Negative results obey the existing maximum 60-second
default and remain source-scoped. Disk persistence, popularity, and stale-while-revalidate behavior
MUST NOT revive withdrawn or expired authority. Historical records may remain for permitted audit
without becoming live routes or new spending rights.

Cold processing stages and validates a complete index/forwarding generation before publishing it
atomically. Readers hold a stable generation until completion; reclaim old segments only after no
reader or required evidence reference needs them. Revocation/withdrawal invalidation takes effect
before another generation is selected. A failed rebuild preserves the prior valid generation, with
independent expiry enforcement; it cannot leave partially updated live authority.

## 6. Durability, reclamation, and memory

Cache entries may be evicted; committed agreements and unresolved payment intents are not disposable
cache entries. Expiry, withdrawal, evidence retention, and garbage collection have separate policies.
Storage pressure pauses new admitted work before losing evidence needed to reconcile prior spending.

Reuse and strengthen the core WAL/segment lifecycle to make signed bytes, their projections, the
reservation, and the settlement instruction recoverable together. Define the commit point, writer
coordination, flush/sync ordering, atomic root replacement, torn-write handling, and restart replay
before external submission is enabled. Single-Quin sync and temporary-file rename alone do not prove
that multi-object economic state is crash-consistent. Browser durability requires its own evidence.

Compaction operates on marker-owned, budgeted artifacts, preserves pinned contract bundles and live
receipt references, and publishes a new generation before reclaiming the old one. Persisted
record mappings need protection against concurrent file mutation/truncation. Private source bytes
and indexes use scoped access/encryption; a commons flag never overrides sensitive contents.

Budget records, indexes, lexicons, decoding buffers, live generations, replay state, and queues
together. Large disk volumes do not need to be fully resident. Mapped file size, resident working
set, heap/arena use, and peak construction memory are different measurements; mmap alone does not
prove the 42 MiB execution-pass ceiling. Every hot path remains allocation-free, including rejection
paths; existing allocating error construction needs repair before those helpers are placed there.

## 7. Evidence and economics

The proposed [Q42 networking modality](../q42-network-modality-draft.md) formalizes source evidence,
semantic projections and compiled admission views on these storage facilities. Its byte-layout
clarification preserves the 48-byte ABI and five-field parity. Large PQ proofs/full digests use
versioned exact-evidence records; no 60-bit index or container checksum can stand in for them.

Required integration tests cover restart with pending payment, torn writes, concurrent publication,
stale-generation handles, expiry/withdrawal during lookup, scope leakage, hash/index false positives,
corrupt compressed blocks, and bounded cold/warm reads on native and browser targets.

Benchmark representative RARs, semantic bundles, contracts, receipts, and private/public scopes.
Report bytes per stored record including indexes and full evidence, allocations, working-set peaks,
disk bytes/page faults, lookup/insert latency distributions, and eviction/rebuild cost at stated
occupancy. Verify signatures and policy in the measured service path. No cache performance or memory
reduction is claimed by this documentation review.

Record elapsed time and attributable joules where measured, otherwise estimates or unknowns under
the [economics profile](./commons-and-resource-economics.md). Lower allocation or RAM consumption
can motivate energy measurements but is not itself a joule saving or an automatic extra charge.
Sharing a cached artifact recognizes creation and stewardship through its agreed contribution rules
while accounting for the resources actually spent serving it.
