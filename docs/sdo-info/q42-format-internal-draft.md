# q42 Format Internal Draft

**Status:** Internal draft  
**Date:** 2026-06-08  
**Purpose:** Freeze the current implementation reality of the `q42` container
family before any external standardization work begins.

## Draft Headings

1. Block Architecture Mandate
2. Capability Envelope Migration
3. Scope
4. Current Conclusion
5. Current Implementation Reality
6. Terminology
7. Canonical Physical Layout
8. Canonical Artifact Set
9. Contradictions To Remove From The Codebase
10. Proposed Canonical Rules
11. Open Questions
12. Immediate Cleanup Mandate

## 1. Block Architecture Mandate

The `q42` family is hereby defined as a hierarchically indexed,
block-oriented format, not as a continuous stream-compressed archive.

This is the architectural direction the repo should converge on.

### 1.1 What is deprecated

The following model is deprecated:

- treating `.q42` as if it were a single continuous compressed stream
- treating block compression and file identity as the same concern
- allowing multiple incompatible readers to all claim they read canonical
  `.q42`

### 1.2 What is mandated

The target storage model is:

1. A `q42` dataset is partitioned into independently addressable 40,960-byte
   SuperBlocks.
2. Each SuperBlock is the fundamental physical I/O unit.
3. Index lookup determines which block to read before any block payload is
   decompressed.
4. Compression, when used, is block-local rather than whole-file stream-based.

### 1.3 Current vs target index placement

The current implementation stores the block index in a sidecar:

- `.q42.bidx`

The long-term architecture may inline a header and index map at the front of
the file, but that is not the current on-disk reality.

Therefore this internal draft distinguishes:

- current canonicalization target for cleanup: raw `.q42` plus `.q42.bidx`
  sidecar
- possible future revision: single-file `q42` with embedded header and index

The codebase cleanup should first converge on the current sidecar-based model
before attempting a file-level v2 redesign.

## 2. Capability Envelope Migration

The capability envelope namespace must be disentangled from generic `.chk`
usage.

### 2.1 Migration decision

This draft recommends:

- rename QCHK artifacts from `.chk` to `.qchk`
- treat `.qchk` as the canonical extension for Qualia capability envelopes
- keep `QCHK` magic bytes as the in-file discriminator until a later revision
  changes the envelope format

### 2.2 Why this migration is required

The current `.chk` collision creates avoidable ambiguity with other checkpoint
and chunk formats, including the CogAI text path already documented in the
repo.

This draft therefore treats `.chk` support for QCHK as legacy compatibility,
not the long-term public name.

### 2.3 MIME and media-type direction

For future external standardization, the envelope should receive its own media
type rather than borrowing generic checkpoint naming.

Initial candidates to evaluate later:

- `application/vnd.qualia.qchk`
- `application/vnd.qualia.chk+cbor`

This draft does not yet choose between them, but it mandates that the public
identifier be Qualia-specific.

## 3. Scope

This draft covers the Layer 0 data artifacts currently exposed by QualiaDB:

- raw `.q42` SuperBlock files
- compressed `.c.q42` transport files
- `.q42.lex` reverse-lexicon sidecars
- `.q42.bidx` block-range sidecars
- companion `.qualia` vault-manifest semantics as they relate to the artifact
  family entry point
- related capability-envelope migration decisions required to avoid namespace
  collisions during standards work

This draft does not define:

- `did:q42`
- Webizen protocol behavior
- sync handshakes
- the full QCHK payload semantics

## 4. Current Conclusion

This draft adopts the following target model:

1. Raw `.q42` is the canonical on-disk container.
2. `.c.q42` is a separate compressed transport profile derived from `.q42`.
3. `.q42.lex` and `.q42.bidx` are sidecars attached to the raw `.q42`
   artifact.
4. The framed compressed `.q42` reader path currently implemented in
   `q42_reader.rs` is treated as a legacy implementation profile to deprecate
   or rename.
5. QCHK capability envelopes should migrate from `.chk` to `.qchk`.

## 5. Current Implementation Reality

Before cleanup, the repo currently exhibits all of the following at once:

- raw fixed-stride `.q42` SuperBlock writers
- compressed framed `.q42` readers
- `.c.q42` transport artifacts that strip SuperBlock headers and compress quin
  payload chunks
- `.q42.bidx` sidecars instead of embedded block-index headers
- QCHK capability files still documented under `.chk`

This section is descriptive, not normative. The mandate is to converge away
from this mixed state.

## 6. Terminology

### QualiaQuin

The atomic record unit is `QualiaQuin`.

- Size: 48 bytes
- Layout: six little-endian `u64` fields
- Fields:
  - `subject`
  - `predicate`
  - `object`
  - `context`
  - `metadata`
  - `parity`

Important clarification:

- The current implementation is not a "42+6 byte split".
- It is a full 48-byte structure with a dedicated 64-bit `parity` field.

### QualiaSuperBlock

The canonical physical storage page is `QualiaSuperBlock`.

- Size: 40,960 bytes
- Alignment: 4,096 bytes
- Structure:
  - 160-byte header
  - 40,800-byte quin ledger

### Sidecars

- `.q42.lex`: reverse hash-to-string dictionary
- `.q42.bidx`: block-range index

### Transport profile

- `.c.q42`: compressed browser or network delivery artifact

### Vault manifest

- `.qualia`: high-level vault or collection descriptor
- not a raw quin container
- not a capability envelope
- expected to point at one or more `.q42` artifacts and related metadata

## 7. Canonical Physical Layout

### `QualiaQuin`

The canonical record layout is:

```text
Offset  Size  Field
0       8     subject
8       8     predicate
16      8     object
24      8     context
32      8     metadata
40      8     parity
```

All fields are encoded little-endian.

The codebase currently uses `parity` in two related but not identical ways:

- many producers set `parity = subject ^ predicate ^ object ^ context`
- `QualiaQuin::verify_ecc_parity()` is currently a mock corruption check that
  only rejects `u64::MAX`

This draft treats the XOR fold as the intended structural parity convention for
artifact production, while noting that validation behavior is not fully unified
yet.

### `QualiaSuperBlock`

The canonical block layout is:

```text
Offset  Size   Field
0       8      block_sequence_id
8       8      storage_owner_did
16      8      active_quin_count
24      4      validation_checksum
28      4      hardware_profile_flags
32      8      fea_mesh_index_id
40      120    layout_padding
160     40800  quin_ledger[850]
```

Properties:

- `std::mem::size_of::<QualiaSuperBlock>() == 40960`
- `std::mem::align_of::<QualiaSuperBlock>() == 4096`
- `QUINS_PER_BLOCK == 850`
- each quin slot is 48 bytes

The raw `.q42` profile is therefore:

```text
.q42 file = N x QualiaSuperBlock
```

No whole-file compression is implied by this profile.

### Active quin semantics

- `active_quin_count` indicates how many ledger entries are semantically active
  in the current block
- remaining ledger slots are zero-filled padding

## 8. Canonical Artifact Set

### Raw `.q42`

Recommended canonical meaning:

- concatenated raw `QualiaSuperBlock` records
- exact block stride: 40,960 bytes
- suitable for memory-mapped reads and block-range fetches

This matches:

- `crates/qualia-core-db/src/lib.rs`
- `crates/qualia-core-db/src/storage.rs`
- `crates/qualia-cli/src/ingest.rs`
- browser VFS assumptions in `docs/playground/vfs.js`

### `.c.q42`

Recommended canonical meaning:

- derived transport artifact for browser or network delivery
- created by stripping the 160-byte SuperBlock headers from raw `.q42`
- compressing concatenated quin payload chunks with LZ4
- framing each compressed chunk with a 16-byte block header

Current chunk frame:

```text
Per chunk:
  [0..8]   block_id      u64 LE
  [8..12]  comp_len      u32 LE
  [12..16] uncomp_len    u32 LE
  [16..]   payload       lz4_flex::compress_prepend_size output
```

This matches:

- `crates/qualia-cli/src/compress.rs`
- `docs/playground/vfs.js`

Important semantic point:

- `.c.q42` is not the same artifact as raw `.q42`
- `.c.q42` is a transport profile, not the canonical storage container

### `.q42.lex`

Current implemented binary layout:

```text
Header (32 bytes)
  [0..8]   magic          "Q42LEX\0\0"
  [8..16]  entry_count    u64 LE
  [16..24] strings_offset u64 LE
  [24..32] version        u64 LE

Index (entry_count x 16 bytes)
  [0..8]   hash           u64 LE
  [8..16]  str_off        u64 LE

String blob
  repeated:
    [0..2]  length        u16 LE
    [2..]   UTF-8 bytes
```

### `.q42.bidx`

Current implemented binary layout:

```text
Header (16 bytes)
  [0..4]   magic          "BIDX"
  [4..8]   version        u32 LE
  [8..12]  block_count    u32 LE
  [12..16] reserved       u32 LE

Index (block_count x 16 bytes)
  [0..8]   min_hash       u64 LE
  [8..16]  max_hash       u64 LE
```

### `.qualia`

Recommended canonical meaning:

- human-facing vault or collection manifest
- entry point that binds together lower-level artifacts
- expected to reference:
  - one or more `.q42` data files
  - optional `.q42.lex` and `.q42.bidx` sidecars
  - optional `.qchk` capability envelopes
  - optional qapp or shell entry metadata

This draft does not freeze the manifest schema yet, but it does freeze the
artifact role:

- `.qualia` is the orchestration layer
- `.q42` remains the raw data layer
- `.qchk` remains the capability layer

## 9. Contradictions To Remove From The Codebase

### 9.1 Raw `.q42` vs framed compressed `.q42`

The repo currently uses `.q42` for at least two incompatible artifact forms:

1. raw fixed-stride SuperBlocks
2. framed compressed block-stream data read by `q42_reader.rs`

This draft recommends:

- keep raw SuperBlocks as `.q42`
- keep compressed transport as `.c.q42`
- deprecate or rename the framed compressed `.q42` reader profile

### 9.2 BIDX dimension mismatch

There is a documented mismatch between implementation and some prose docs:

- `crates/qualia-cli/src/ingest.rs` sorts by object hash and writes min/max
  object hash ranges
- browser VFS lookup logic also uses object-hash semantics
- some architecture docs describe BIDX in terms of subject hash

This draft follows the current implementation:

- `BIDX` currently indexes object-hash ranges

If the project wants subject-hash indexing instead, that must be a deliberate
format revision rather than an undocumented prose correction.

### 9.3 Naming drift: `.q42` vs `.qla`

Older storage comments still refer to `.qla`. The artifact family should be
normalized to `.q42` terminology in future cleanups.

### 9.4 Compression wording drift

Some docs say `.q42` itself is LZ4-compressed. The codebase instead supports a
cleaner model:

- raw `.q42` for aligned storage
- `.c.q42` for compressed transport

This draft recommends aligning all docs and tooling to that distinction.

### 9.5 QCHK extension collision

QCHK capability envelopes are still named as `.chk` in multiple parts of the
repo even though the extension space is already overloaded.

This draft recommends:

- `.qchk` becomes the canonical extension
- `.chk` remains legacy compatibility only during migration
- parser, docs, fixtures, and UI labels must converge on `.qchk`

## 10. Proposed Canonical Rules

1. `.q42` means raw aligned SuperBlock container.
2. `.c.q42` means LZ4-framed transport artifact derived from raw quin payloads.
3. `.q42.lex` is the canonical reverse lexicon sidecar.
4. `.q42.bidx` is the canonical block-range sidecar.
5. `.qchk` is the canonical capability-envelope extension.
6. `.qualia` is the canonical vault-manifest or collection-descriptor
   extension.
7. Raw `.q42` readers and writers must agree on:
   - little-endian integer encoding
   - 40,960-byte block size
   - 160-byte header size
   - 850 quin slots per block
8. Legacy framed compressed `.q42` handling must be renamed, version-gated, or
   deprecated.
9. Legacy `.chk` handling for QCHK must be renamed, version-gated, or
   deprecated.

## 11. Open Questions

1. Should `validation_checksum` remain a real field if `parity` already exists?
2. Should `BIDX` continue indexing object-hash ranges, or move to subject-hash
   ranges in a versioned revision?
3. Do we need a file-level magic for raw `.q42`, or is fixed-stride structure
   sufficient?
4. Should the compressed transport profile keep the `.c.q42` extension or move
   to content negotiation only?
5. Should raw `.q42` receive an explicit version header in a future v2 block
   format?
6. Should a future v2 embed the block index into the file header instead of
   using `.q42.bidx` as a sidecar?
7. Which public media type should be chosen for `.qchk`?
8. Should `.qualia` be JSON-LD, N3, or another manifest syntax?
9. Should file association be wired first in the Flutter shell, with Tauri
   treated as secondary or legacy?

## 12. Immediate Cleanup Mandate

1. Align docs to raw `.q42` vs transport `.c.q42`.
2. Scan the codebase for stream-style `.q42` assumptions and mark them legacy.
3. Decide whether `q42_reader.rs` is renamed or replaced.
4. Decide whether `BIDX` is officially object-indexed.
5. Rename QCHK public references from `.chk` to `.qchk`.
6. Update parser, fixtures, and UI labels to treat `.qchk` as canonical.
7. Define a minimal `.qualia` manifest schema:
   - `qualia:vaultName`
   - `qualia:includesDataFile`
   - `qualia:entryPointQapp`
8. Document Flutter-first file association strategy for `.qualia`, with Tauri
   noted as in-tree but not the primary shipped shell.
9. Add one canonical media-type proposal section for future IETF drafting.
10. Add test vectors:
   - one single-block raw `.q42`
   - one matching `.c.q42`
   - one `.q42.lex`
   - one `.q42.bidx`
   - one `.qchk`
