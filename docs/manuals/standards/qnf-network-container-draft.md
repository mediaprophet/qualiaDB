# QNF Network Container

**Status:** Proposed format 0.1; no reader/writer or interoperable byte vectors implemented

**Date:** 2026-09-06

**Working extension:** `.qnf` — Qualia Network Format

## 1. Decision and purpose

Define a dedicated, immutable network artifact container for
[Qualia Peer Runtime](./qualia-decentralized-network-fabric/peer-runtime.md). QNF stores exact signed
evidence, indexed objects and optional compiled network views in independently addressable sections.
It follows the bounded, contiguous, relative-offset principles of P64 and 10D, with a layout chosen
for network objects and large post-quantum proofs.

Q42 remains the semantic graph representation and the home of the
[networking modality](./q42-network-modality-draft.md). QNF is a sibling format managed by the existing
core artifact/generation lifecycle. It is not a new general-purpose database, an alternative wallet
ledger, a packet wire format, or a dump of live session memory. The Q42 48-byte ABI is unchanged.

This revises the earlier requirement to fit all persistent network bytes into `.q42` itself. Exact
source evidence may live in QNF while Q42 stores its semantic projection and references. The two
representations share a core commit and authority boundary; the same source object need not be
duplicated in both formats. Standalone distribution is allowed, but receipt of a signed file never
installs its policy or compiled views automatically.

## 2. Why a separate format

| Option | Fit | Decision |
|---|---|---|
| All network objects as NQuin payloads | Efficient semantic indexing, awkward for kilobyte proofs and exact variable-length source objects | Preserve projections; do not fragment every signature into invented semantic fields |
| Extend only the generic Q42 container | Shared storage machinery; couples network object/index evolution to the semantic volume | Keep support for existing Q42 sources and use core adapters |
| Purpose-specific QNF | Exact object bytes, simple object/chunk tables, optional compiled views, independent versioning | Adopt as the target network artifact format; validate the additional code/cost |
| Independent networking database | Would duplicate transactions, authority and recovery | Use existing core ownership instead |

The [P64 layout](../../../crates/qualia-core-db/src/q42/p64_weight/layout.rs) currently declares its
own v4 format; its [reader](../../../crates/qualia-core-db/src/q42/p64_weight/reader.rs) borrows blobs
but allocates descriptor storage. The [10D section implementation](../../../crates/qualia-core-db/src/container_10d/section.rs)
has bounded tables and caller-buffered encoding; [10D integrity helpers](../../../crates/qualia-core-db/src/container_10d/integrity.rs)
provide shared CRC machinery. Reuse checked-range, layout and writer techniques after audit.
Their checksums and alignment names are not cryptographic authentication or portable page guarantees.

QNF must earn its complexity through bounded-reader and representative end-to-end measurements.
Smaller descriptors or fewer copies alone do not prove lower latency, energy, or resident memory.

## 3. Representation and publication roles

| Representation | Role |
|---|---|
| Exact object bytes in QNF | Original contract, authority record, checkpoint, receipt, ciphertext or other admitted artifact; independently verifiable |
| Q42 projections | Queryable meanings, provenance, units and policy inputs linked to full source digests |
| Optional QNF compiled views | Rebuildable route/service/index generations with pinned compiler, input, scope and ABI bindings |
| Live network cell | Keys, nonce/replay windows, congestion state, queues and leases; not persisted as an executable image |

QNF stores immutable generations. Operation admission, replay identity, application effects and
settlement intent still use core transaction/recovery semantics. A new `.qnf` snapshot is not a
write-ahead log or an independent source of mutable account balances. Custody ciphertext may use
QNF artifact storage; custody acceptance/deletion/retention state stays in the core commit lifecycle.

## 4. Candidate physical layout

All integer fields are little-endian. Offsets below describe a proposed byte contract to test/freeze,
not structs that may be cast blindly from an untrusted mapping. Header, directory and sections start
on explicit 64-byte boundaries. This is file alignment, not a promise about OS page/cache sizes.

### 4.1 Header: 256 bytes

| Offset | Bytes | Field |
|---:|---:|---|
| 0 | 8 | Magic: ASCII `QNFNET01` |
| 8 | 2 | Format major: 1 |
| 10 | 2 | Header length: 256 |
| 12 | 4 | Flags; zero in the initial profile |
| 16 | 8 | Exact file length |
| 24 | 8 | Nonwrapping generation sequence within the declared scope |
| 32 | 8 | Directory offset: 256 in the initial profile |
| 40 | 4 | Section count, at most 32 |
| 44 | 4 | Directory entry size: 128 |
| 48 | 48 | Scope descriptor digest, SHA-384 |
| 96 | 48 | Format/semantic/crypto profile bundle digest, SHA-384 |
| 144 | 48 | Directory digest, SHA-384 |
| 192 | 48 | Generation commitment, SHA-384; construction below |
| 240 | 16 | Reserved, zero |

The first profile fixes SHA-384 for these commitments and the descriptor digests. Another algorithm
requires an explicit version/profile with new vectors; no digest truncation or reinterpretation of
Q42's SHA-256 header root is allowed. A sequence is meaningful only with its authorized scope and
predecessor/current-head policy; a high sequence number alone does not win authority.

### 4.2 Directory entry: 128 bytes

| Offset within entry | Bytes | Field |
|---:|---:|---|
| 0 | 4 | Section kind: four ASCII bytes |
| 4 | 4 | Section flags; zero in the initial profile |
| 8 | 8 | Absolute file offset |
| 16 | 8 | Stored byte length |
| 24 | 8 | Decoded byte length; equal to stored length in this profile |
| 32 | 4 | Fixed record width, or zero for an explicitly variable section |
| 36 | 4 | Record count; zero for variable sections |
| 40 | 8 | Auxiliary directory index, or `u64::MAX` for none |
| 48 | 48 | SHA-384 of exact section bytes; AUTH exception below |
| 96 | 32 | Reserved, zero |

Entries are unique by kind and ordered by raw ASCII kind bytes. All referenced intervals are
checked for overflow, file bounds and overlap, including header/directory and padding. Padding is
zero and cannot carry extra records. For fixed tables, `record_width × count` must equal section
length using checked arithmetic. Unknown flags/kinds fail the initial profile; extensibility needs
a separately defined critical/optional rule rather than silently ignoring unknown executable data.

### 4.3 Initial sections

| Kind | Contents | Access |
|---|---|---|
| `AUTH` | Bounded dual COSE proof bundle over the generation commitment | Verify under externally accepted publishing authority; does not replace individual object proofs |
| `BIND` | Bounded deterministic-CBOR scope/profile, source-generation and optional view/compiler bindings | Exact bytes; URI/digest/role interpretation follows the pinned bundle |
| `CHNK` | 64-byte chunk descriptors for DATA | Verify this bounded table before accepting partial DATA reads |
| `DATA` | Contiguous exact object bytes, uncompressed in the initial profile | Byte-preserving read ranges, no session-key storage |
| `OBJS` | 64-byte object descriptors | Object kind, exact interval and full digest |
| `VIEW` | Optional immutable compiled networking view with a separately selected schema | Validate/rebuild under local authority, compiler and ABI before use |

All except VIEW are required. Section-count headroom is reserved for future profiles; it does not
authorize an initial reader to accept unnamed sections. DATA may contain already encrypted object
bodies. The file layout/index itself is visible: private object counts, digests or view metadata
require a private distribution scope or a separately specified encrypted outer container.

An object descriptor is exactly 64 bytes: DATA-relative offset (`u64`), length (`u32`), object kind
(`u16`), flags (`u16`, initially zero), then 48-byte SHA-384 of the exact object. Kinds refer to the
profile's fixed allowlist, not a NQuin inline datatype. Canonical order is offset then full digest;
intervals may not overlap. Equal source bytes can share one descriptor via higher-level references.

A chunk descriptor is exactly 64 bytes: DATA-relative offset (`u64`), stored length (`u32`), decoded
length (`u32`, identical initially), then 48-byte SHA-384 of that chunk. Chunks cover DATA exactly,
in order, with no gaps/overlap; all except the last are 64 KiB. Object boundaries may cross chunks.
Validate every covering chunk and the complete object digest before claiming object validity.

Large objects remain subject to their service-specific limits. A QNF file does not authorize a
larger contract, route record, expanded graph, or handshake than QDNF's selected profile permits.

### 4.4 Integrity without a signature cycle

The AUTH entry's digest is all zero by definition; no other nonempty section has that exception.
Compute all other section digests, then SHA-384 over the complete directory. Compute the generation
commitment as SHA-384 of `"QNF-GENERATION-V1"`, the exact header with bytes 192–239 zeroed, and the
exact directory, in that order. The header includes the directory digest. AUTH offset and exact
length are already fixed by the chosen proof encoding before commitment/signing.

AUTH contains the profile's required ML-DSA-65 and Ed25519 COSE proofs over the same binding object
containing that commitment, scope and profile. Verify both using the
[PQ proof policy](./qualia-decentralized-network-fabric/post-quantum-security.md); an included key is
not automatically an authorized publisher. Encode/decode vectors must prove sizing and the absence
of a header/signature self-reference. No ignored AUTH tail or alternative unsigned header is allowed.

The generation commitment authenticates the directory's content commitments and layout. It is not
the SHA-384 of every byte in the file: AUTH bytes are outside that hash. A core artifact reference
uses the separately computed whole-file digest when exact-file identity matters. Substituting a
different AUTH bundle must still satisfy the full selected publishing authority and proof policy.

Partial reads verify AUTH, BIND, directory, the complete bounded CHNK/OBJS tables and the requested
chunks/object. They establish validity of those ranges only. Full-file verification additionally
checks every section digest, whole DATA coverage and any expected exact-file digest. A valid chunk
or manifest signature does not establish all source signatures, semantic truth, or completeness of
an authorized dataset. Keep these validation states distinct in the API.

## 5. Bounds and corruption behavior

Initial hard caps: 1 GiB file, 32 directory entries, 65,536 objects, 16,384 DATA chunks, 64 KiB BIND,
and 64 KiB AUTH. Thus OBJS is at most 4 MiB and CHNK at most 1 MiB. The 1 GiB file cap includes all
overhead, so all maxima are not simultaneously achievable. Larger collections use independently
bounded QNF generations referenced by a core manifest, without forcing a whole-collection read.

A reader accepts a stricter local byte/work/object limit before allocating or mapping working pages.
Validation is iterative and caller-buffered. Huge counts, checked-range failure, unrecognized
profiles, digest mismatch, duplicate kinds, nonzero reserved bytes and unsupported view ABI return
typed errors. A malformed file cannot schedule an unbounded graph compile or proof-verification job.

The initial uncompressed profile makes exact range access and bounds simple. Add block-local
compression only when benchmarks justify it, with new codec parameters, expansion limits and
per-chunk vectors. Do not copy Q42's 40,960-byte SuperBlock size or P64's weight-page size merely
because the formats share a core; QNF's initial DATA chunk is explicitly 65,536 bytes.

Mmap can avoid a source copy for suitable immutable public bytes on a native host. It does not avoid
page residency, hashing, encryption, endian conversion or validation. Browser/constrained readers
use bounded range APIs with the same guarantees. Pin file identity/generation against replacement
or truncation while any range is borrowed. CRC may diagnose accidental corruption in an optional
future profile, but never replaces the SHA-384 and signature checks here.

## 6. Views, invalidation and core commits

BIND names the full source evidence/projection generation, semantic/policy/crypto profiles and,
when VIEW is present, compiler version, view schema, sensitivity and target execution ABI. VIEW
contains relative references or validated table indices, never process pointers or live lease IDs.
No raw machine code or remote serialized capability becomes executable by being in VIEW.

The receiving core verifies source evidence and current authority, validates or rebuilds the view,
and issues fresh local generation-checked handles to the networking cell. Remote claimed policy
generation numbers cannot select local authority. Stale compatible views can be rebuilt from exact
evidence; unsupported or unverifiable views remain unavailable. Byte-level corruption is rejected,
not repaired by treating damaged bytes as another representation.

Publish QNF evidence, Q42 projections, operation/replay records and generation references through one
recoverable core commit. Write a bounded temporary artifact, finalize/verify it, synchronize the
required files, then publish the root/commit marker with tested platform durability. On restart,
unpublished generations are collectible and committed references resolve to complete verified data.
QNF rename alone does not prove a multi-artifact transaction; existing WAL/root helpers need work.

Retention is reference- and policy-based. An immutable generation may remain as historical evidence
after its routes, capabilities or plans expire. Compaction changes offsets and exact-file identity;
it cannot make an old handle current, revive authority, or discard unresolved replay/payment records.
Scope-sensitive deduplication must not reveal that another private context holds the same object.

## 7. Implementation and validation

Add focused `container_qnf/` layout, read, write, integrity, validation and test modules under the core,
sharing audited bounded-container primitives with `container_10d` where appropriate. Keep QPR view
compilation in the networking library and transaction publication in core storage owners. No P64,
10D, Q42 or live network state is silently reinterpreted as QNF.

P16 in [Implementation and Conformance](./qualia-decentralized-network-fabric/implementation-conformance.md#qdnf-p16--qnf-container-and-core-publication)
must produce exact empty/minimal/multi-chunk/PQ vectors, signature/root tampering cases, range and
integer overflow tests, borrowed-lifetime/mutation tests, unknown-version rejection, and crash
injection across QNF/Q42/root publication. Measure reader working memory, verification work, useful
bytes, cold/warm latency and energy for representative small/large/private/PQ evidence workloads.
The format remains proposed until a separately implemented parser reproduces its vectors.
