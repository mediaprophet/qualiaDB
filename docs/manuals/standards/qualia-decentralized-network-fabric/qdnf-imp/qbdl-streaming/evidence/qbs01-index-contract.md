# QBS-01 provisional shared QBDL index contract

2026-11-12. Parser interface review only; provisional until QBS-00 and integrator acceptance. No implementation, builds, tests, source edits, registry edits or changes to unrelated Q42 authoring. The root reports its current core build is blocked by missing q42/emf_container/extension/tests.rs from concurrent unrelated work; that is not attributed to these parser interfaces and was not repaired here.

Baseline: evidence/baseline-network.md, QualiaDB HEAD afbe431195167fde0fadc233ed79fd2fe85f73f0. This review reread bundle/{format.rs,reader_limits.rs,reader_limits_tests.rs,streaming_loader.rs,writer.rs}. Line references below are to that inspected source; signatures below are proposed, not exports claimed to exist.

## Decision

Use one borrowed, nonallocating structural index admission owner for bounded whole-file and progressive header/index paths. The output means structurally valid directory bytes only. It is not publisher authority, SHA-384 proof acceptance, whole-file CRC verification, payload SHA-256 verification or permission to consume an inner format. Keep those states separate.

Keep existing public types and signatures unchanged: BundleReadLimits, BundleReader::{parse,parse_with_limits}, HmcHeaderProbe::parse_header_range, HmcProgressiveRangePlanner::from_header_and_index, HmcRangeRequest, verify_arrived_range and existing bundle re-exports. Add explicit bounded remote construction APIs rather than adding fields to public struct literals or changing argument lists. Root owns bundle/mod.rs, shared exports, Cargo and registries.

## Proposed exact shared signatures

The core implementation belongs in bundle/stream/index. Start visibility at pub(crate); root can expose the explicitly selected remote facade after review. The proposed IndexReadLimits has the existing resident fields without file_bytes: index_bytes, entries, key_bytes, kind_bytes, meta_bytes, decoded_index_bytes, each usize. Keep its defaults consistent with the current bounded index profile. decoded_index_bytes bounds the conservative aggregate of retained owned index storage plus simultaneous scratch, not remote content or actual RSS. A separate ExtentLimit carries only max_logical_bytes:u64 (initial QBS negotiated ceiling at most 1TiB); it never sizes an allocation.

```rust
// Existing public BundleReadLimits remains unchanged; conversion copies
// resident index limits and does not copy file_bytes into a remote size cap.
impl From<BundleReadLimits> for IndexReadLimits;

// Exact 64-byte network header, independent expected remote extent and cap.
pub(crate) fn admit_header(
    header_bytes: &[u8],
    expected_extent: u64,
    extent_limit: ExtentLimit,
    limits: IndexReadLimits,
) -> Result<CheckedHeader, IndexError>;

// Scans every encoded field, measures bounded retained/scratch demand,
// and never allocates. Does not yet certify unique keys/disjoint ranges.
pub(crate) fn measure_index(
    header: &CheckedHeader,
    index_bytes: &[u8],
    limits: IndexReadLimits,
) -> Result<IndexSizing, IndexError>;

// Rescans/validates the same immutable bytes, fills caller-owned rows,
// sorts that scratch in place by key then range, and checks global invariants.
// The returned value borrows index_bytes, not mutable scratch.
pub(crate) fn admit_index<'index>(
    header: &CheckedHeader,
    index_bytes: &'index [u8],
    limits: IndexReadLimits,
    scratch: &mut [EntryScratch<'index>],
) -> Result<AdmittedIndex<'index>, IndexError>;

// Cold, fallible conversion to existing BundleEntry output. It does not
// copy/re-encode an input bundle or change its authenticated encoded bytes.
pub(crate) fn materialize_entries(
    index: &AdmittedIndex<'_>,
    limits: IndexReadLimits,
) -> Result<Vec<BundleEntry>, IndexError>;
```

CheckedHeader, IndexSizing, EntryScratch and AdmittedIndex have private constructor/state fields. CheckedHeader stores fixed integer fields using their native widths, including u64 extent/index range; its alignment accessor returns 64 or 4096 from checked flags. AdmittedIndex stores the checked header and immutable original index slice and exposes entry count plus bounded iteration over borrowed entry views. It must not be named VerifiedIndex. IndexSizing reports entry_count, retained_bytes, scratch_bytes and aggregate bytes; it is sizing information, never an admission token. EntryScratch stores the borrowed key, checked u64 offset/end, length and original row ordinal; it contains no owned strings or payload data. Both empty scratch initialization and invalid handles are explicit constants; success returns only after all rows/global checks finish.

IndexError is a nonallocating enum or fixed record with a finite code, optional row ordinal and u64 observed/limit values. Codes distinguish header length/magic/version, unsupported flags, reserved bytes, extent mismatch/cap, arithmetic overflow, footer placement/alignment, encoded/resident/count limits, malformed/unsupported CBOR, duplicate/missing field, duplicate key, invalid digest, payload overlap/bounds and scratch capacity. It never copies an attacker key or parser-created String. Existing public wrappers translate it to current BundleError variants where representable; otherwise use a fixed BundleError::Cbor message on the cold boundary. Do not truncate a u64 remote extent into BundleError's usize total/actual fields (format.rs:120–128). Root need not add variants to the public BundleError enum for QBS-02.

Implementation may combine measure/fill behind one internal parser routine. The public contract must preserve the two properties: all encoded lengths are bounded before owned decoding, and no admitted directory can escape before global duplicate/range checks. measure_index output cannot bypass a rescan if bytes or limits differ. An immutable borrow or source digest is necessary if a cached sizing result is reused later.

## Wrapper behavior and compatibility

1. BundleReader::parse_with_limits(bytes, old_limits): first require resident bytes.len() <= old_limits.file_bytes; convert bytes.len() to u64 with try_from; take exact first 64 bytes; admit_header with expected_extent equal to actual full length and an extent cap derived from the local file limit. Extract footer only after checked u64-to-usize conversions and checked slice bounds. Measure index, charge aggregate retained/scratch demand, allocate exact bounded cold scratch with try_reserve_exact, then call admit_index. Verify whole-file CRC and all complete-entry SHA-256 before returning the reader. Disjointness must be proven before complete-entry hashing, so duplicate references cannot multiply hashing work. Existing payloads still borrow the original slice.
2. BundleReader::parse(bytes): preserve its documented legacy whole-file CRC and payload-bound semantics, including the existing regression that a CRC-resealed changed payload can parse while parse_with_limits rejects it (reader_limits_tests.rs:71–78). It may delegate arithmetic/header helpers and compatible structural routines, but must not silently promise bounded admission or source authority. Do not force historical encodings through the strict definite-field profile without an explicit compatibility decision. The common strict contract is shared by bounded full admission and progressive admission; legacy parse remains explicitly named/documented compatibility.
3. HmcHeaderProbe::parse_header_range(header_bytes): retain the existing >=64 prefix acceptance by passing only [0..64) to shared header checks after length validation. The existing method cannot validate a transport/descriptor extent it was never given; check the header's declared extent against itself and default logical cap, and document that external extent pinning happens at remote construction. It must now reject unknown flags/reserved bytes and invalid footer geometry. Add a bounded method/facade with an externally pinned expected_extent for the remote path; do not treat the first header's own total_length as authenticated truth.
4. HmcProgressiveRangePlanner::from_header_and_index(header, index): preserve signature and return type. HmcHeaderProbe has public fields (streaming_loader.rs:57–64), so revalidate every numeric field, magic/version assumptions and flags via an internal checked_from_probe routine; do not trust the word Probe as a capability. This method cannot prove original reserved header bytes because those bytes are not represented in the public struct. It uses the same resident index limits, measure/admit/materialize owner and checks header entry count/exact footer length. Its structural result remains unauthenticated.
5. Add HmcProgressiveRangePlanner::from_header_and_index_with_limits(header, index, expected_extent:u64, extent_limit:ExtentLimit, limits:IndexReadLimits), plus a raw-header remote facade on bundle::stream::index that calls admit_header directly. The raw facade is the preferred QBS boundary because it can check exact original header bytes, reserved fields and externally pinned extent. Existing from_header_and_index delegates using header.total_length and default limits; it never claims an independent extent check. Root selects final public placement/naming; adding a new constructor is source compatible.
6. Public wrappers may allocate cold within explicit limits; the shared header/index scan/admission and its errors must allocate zero. Materialization should copy from admitted borrowed views using fallible explicit reservations, avoiding an unchecked second serde allocation graph. If retaining ciborium decoding temporarily, it must remain after preflight, within the aggregate reservation, with exact decoded/borrowed row equivalence checks; do not call it directly on unadmitted network bytes.
7. plan_entry_ranges currently converts a malformed digest to None (streaming_loader.rs:198). Shared admission guarantees every retained digest is exactly 32 bytes, so a planner built through either bounded constructor always supplies Some digest for whole entries. verify_arrived_range must additionally reject a request having neither expected SHA-256 nor expected CRC32C; it can still accept independently checked legacy CRC/SHA requests without pretending they are QBS proof handles. No digest means no verified arrival, including a manually constructed public HmcRangeRequest.

## Header, CBOR and range invariants

- Header magic QBDL, version 1, exact 64 bytes at the raw network boundary; reserved bytes 40..64 all zero. Accept flags exactly 0 or 1; reject all other bits. Preserve flag-0 64-byte and flag-1 4096-byte alignment; do not create new alignment flags or change inner bytes. format.rs:48–59 defines alignment for entries and index footer, and writer.rs:133–183 emits both accordingly.
- All extents/offsets/sums remain u64 until a local resident slice is taken. Require total_length == expected_extent, 64 <= total_length <= max_logical_bytes, index_offset >= 64, checked index_offset+index_length == total_length, and footer alignment from flags. Reject a non-footer index or trailing logical bytes. No footer download/allocation is authorized merely because the header names it; index_length must fit negotiated resident index_bytes before requesting it. Require supplied index_bytes.len() converted with try_from to equal index_length.
- Definite-length top array equals header entry_count and <= limits.entries. Every row is a definite map with exactly one of each required key/kind/offset/length/sha256 and optional meta. Reject duplicate field names, unknown fields, missing required fields, nested substitutes, negative/floating offsets, trailing encoded values and every incomplete decode. No unbounded recursion or live metadata/schema dereference.
- Preserve the existing inner-CBOR profile rather than imposing QASD canonical-CBOR restrictions: field map order need not be canonical; known definite unsigned integer encodings retain their prior meaning. reader_limits.rs:39–52 already recognizes byte strings or definite u8 arrays for sha256/meta; retain those supported forms, with explicit compatibility fixtures proving both materialize correctly. Do not normalize/re-encode original footer bytes for a digest. Nonminimal integer acceptance is not silently tightened as a by-product of outer descriptor work.
- key/kind are valid nonempty UTF-8 and independently byte-capped; metadata is opaque optional bytes (omitted, null or admitted byte form), not parsed as semantic CBOR. Count all retained strings/digests/meta, per-row struct storage and simultaneously live scratch using checked arithmetic before allocation. Repeated small rows/text must hit the cumulative decoded_index_bytes bound. sha256 is required and exactly 32 decoded bytes: absent, null, short, long and non-u8 arrays fail.
- Entry offset >=64, proper alignment, checked offset+length <=index_offset. Preserve original index order for public entries/plans; only scratch is sorted. Compare key bytes directly for uniqueness (no truncated hash identity). Sort ranges in scratch and track occupied end; reject all nonempty overlap including nested/identical ranges. Gaps are allowed and are not inferred to be verified zeros.
- Empty entries are valid writer outputs and require SHA-256 of empty bytes when complete-entry verification is performed; no data leaf is implied. Proposed precise rule: aligned zero-length offsets within [64,index_offset] are accepted and occupy no bytes; they cannot reset the occupied-end check or hide overlap between neighboring nonempty entries. This is the mathematical half-open-range rule and preserves writer outputs. It intentionally clarifies the existing adjacent-pair implementation; freeze the edge rule before implementation and test it explicitly.
- Header/index admission cannot check payload SHA-256, complete-file CRC or descriptor proof from absent payloads. Whole-file full admission adds those checks; remote admission records their status as incomplete. No structural success is exposed as VerifiedLeaf, AuthenticatedPlaintext or ConsumerCommitted.

## Exact file ownership proposal

Paths relative to crates/qualia-core-db/src/. Root approval should claim these files together for QBS-02; no other agent should edit them concurrently.

| Owner | Files / change |
|---|---|
| Root integrator | bundle/mod.rs export/routing only; any shared Cargo/feature/registry changes. No Cargo dependency changes appear necessary for this parser work; minicbor/ciborium/sha2 already exist. |
| QBS-02 parser owner | New bundle/stream/mod.rs (routing only); stream/index/{mod.rs,limits.rs,error.rs,header.rs,cbor.rs,admit.rs,owned.rs,tests.rs}. Keep each implementation focused and below 500 lines; split field decoder tests/resources if needed. APIs/errors/types route through index/mod.rs. |
| Same QBS-02 owner | bundle/reader_limits.rs: retain public BundleReadLimits layout/defaults and compatibility adapter; replace its standalone preflight logic with the shared owner. bundle/reader.rs: bounded full wrapper and error conversion, with legacy parse semantics explicitly retained. bundle/reader_limits_tests.rs: parity/regression coverage. |
| Same QBS-02 owner | bundle/streaming_loader.rs: delegate header/index validation and owned decoding, add bounded constructor, refuse digestless arrival. Move existing unit tests into bundle/streaming_loader/tests.rs using #[path] or directory module with unchanged namespace. Existing public types/functions stay available at bundle::streaming_loader and current bundle re-exports. |
| Deferred | Do not change writer.rs bytes, format.rs flag constants/layout, 10D/EMF/M64 codecs, Q42 extension tests, HTTP transports or QBS proof/authentication APIs in this package. Correct stale prose such as the 10D table-size comment only in a separately agreed scope or clearly incidental documentation-only edit. |

streaming_loader.rs currently has 490 lines; reader.rs has 405, reader_limits.rs 117 and reader_limits_tests.rs 96 (raw split-line count including final empty line). The loader is not yet >=500, but adding another constructor/parser lifecycle would exceed the repository signal. Factor header/index admission into the new directory and move its existing tests out during QBS-02 rather than growing the file. Keep section-range planning and priority behavior intact; a larger priority/consumer refactor is unnecessary for preflight.

## Required tests before QBS-02 acceptance

Run existing bounded reader/progressive/writer fixtures first when the root build blocker is resolved; no counts are inherited here. Pair each rejection with a valid positive fixture so a blanket error cannot pass.

1. Writer roundtrips with flags 0/1 through bounded full and both progressive constructors; exact entries, metadata and requested byte ranges agree and source bundle bytes do not change. Empty bundle and empty entry at shared start/end boundaries pass.
2. Raw header truncated/exact/prefix handling, bad magic/version, all unsupported flag bits, every reserved byte, wrong expected extent, remote logical cap, footer offset+length overflow, index not at logical end, footer before header and flag-specific footer alignment.
3. Fake huge u64 remote extent with a tiny valid header/index is handled without allocating remote-sized memory; extent >usize::MAX on 32-bit/WASM arithmetic fixture; conversion of actual resident length is checked. Logical limit and resident index/scratch limit fail independently.
4. Header/top-array count mismatch, count at/over configured cap, huge declared CBOR arrays/maps/text/bstr with tiny input, incomplete data, trailing values and duplicate map fields. Unknown/nested critical substitutes and indefinite forms refuse before owned materialization.
5. Required fields individually absent/wrong type; digest absent/null/0/31/32/33 bytes and array element >255/noninteger. Both admitted bstr/u8-array representations and all accepted field orders preserve values and original encoded bytes.
6. key/kind/meta exact per-field caps; empty/invalid UTF-8 key/kind; omitted/null/empty/nonempty opaque metadata. Aggregate decoded/scratch cap exhaustion from many valid small fields refuses with no output publication.
7. Duplicate keys with distinct ranges, identical/nested/partially overlapping nonempty ranges, checked end overflow, before-header/inside-index ranges, 64/4096 alignment violations, legal adjacent ranges/gaps and unsorted index order. Zero-length rows cannot hide overlap; public iteration retains encoded row order.
8. Insufficient scratch (0,count-1), exact scratch and oversized scratch; every rejection leaves no admitted object and does not publish partially materialized entries. New allocations occur only in documented cold wrappers; core scanner/admission/error paths have allocator positive-control zero-allocation coverage.
9. Full-file CRC mismatch; CRC-valid tampered payload rejected by bounded full SHA checking, while legacy parse behavior remains its existing regression. Progressive index admission succeeds structurally on absent payload and never claims whole-file or entry digest completion.
10. Manually forged public HmcHeaderProbe invalid flags/count/ranges, manually constructed digestless HmcRangeRequest, wrong arrival length/CRC/SHA and valid zero-length complete-entry digest. Unbounded/deprecated constructors cannot bypass shared structural rules.
11. Independent malformed footer byte fixtures rather than fixtures generated exclusively through the production reader/writer; baseline source hashes/feature/command/result evidence recorded after actual execution.

Acceptance requires the signatures and empty-range/legacy compatibility rules to be agreed by root, then tests on the current source snapshot. The unrelated missing Q42 extension test file remains the root's external build prerequisite. This document authorizes no source changes or status promotion.
