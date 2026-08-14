# q42 Format Internal Draft

**Status:** Internal draft (implementation converged on v3 unified volume, 2026-06-11; indexes, publication gates, and five-field ECC recorded 2026-08-15)  
**Date:** 2026-06-09 (revised 2026-08-15 — v3 indexes + publication)  
**Purpose:** Freeze the current implementation reality of the `q42` container
family before any external standardization work begins.

This revision **does not relax** the 48-byte NQuin, 40,960-byte SuperBlock,
256-byte header, v3-only new writes, or 42 MB SlgArena ceiling. It records
indexes, ECC, and publication rules that the writer already enforces.

## Draft Headings

1. Block Architecture Mandate
2. Capability Envelope Migration
3. Scope
4. Current Conclusion
5. Current Implementation Reality
6. Terminology
7. Canonical Physical Layout
8. Canonical Artifact Set
9. Legacy Profiles (read-only compatibility)
10. Contradictions Resolved or Remaining
11. Proposed Canonical Rules
12. Open Questions
13. Remaining Cleanup Mandate

## 1. Block Architecture Mandate

The `q42` family is defined as a hierarchically indexed,
block-oriented format, not as a single continuous stream-compressed archive.

### 1.1 What is deprecated

The following models are deprecated for **new writes**:

- treating `.q42` as a raw concatenation of uncompressed SuperBlocks with
  mandatory `.q42.lex` and `.q42.bidx` sidecars
- treating `.q42` as if it were a single continuous framed LZ4 stream (the
  profile read by `q42_reader.rs`)
- emitting a separate `.c.q42` as a distribution artifact. LZ4 SuperBlocks
  live inside the unified `.q42`. The `.c.q42` extension is obsolete.

### 1.2 What is mandated

The storage model is:

1. A `q42` dataset is partitioned into independently addressable 40,960-byte
   SuperBlocks.
2. Each SuperBlock is the fundamental physical I/O unit.
3. Index lookup determines which block to read before any block payload is
   decompressed.
4. Compression, when used, is **block-local** (per SuperBlock), not
   whole-file stream-based.

### 1.3 Index placement (v3 — implemented 2026-06-11)

As of 2026-06-11, **new ingest writes a single unified v3 `.q42` volume**:

- 256-byte file header (`Q42\0`, version 3)
- embedded Q42LEX blob (uncompressed)
- embedded BIDX blob (uncompressed)
- optional FIDX (per-block subject/predicate/context ranges; flag `0x0008`)
- optional PIDX (compact S/P/C postings or measured Bloom; flag `0x0010`)
- block directory (16 bytes per block: relative offset, compressed length,
  uncompressed length)
- concatenated LZ4-compressed SuperBlock payloads
- optional temporal index section (offset/length in header; `0` if absent)
- optional Merkle-DAG history section (offset/length in header; `0` if absent)

Implementation: `crates/qualia-core-db/src/q42/q42_volume.rs` and
`crates/qualia-core-db/src/q42/volume/`.

New writes set `FLAG_PERMISSIVE_COMMONS` (`0x0020`) only for affirmatively
catalogued public datasets. `FLAG_SANCTUARY` (`0x0040`) is set for personal,
medical, legal, fiduciary, bilateral, or otherwise restricted volumes.
Public magnets / web-seeds / IPFS pins **fail closed** unless Commons is set
and Sanctuary is clear. See `q42/volume/publication.rs`.

v2 files are **hard-rejected** by v3 builds — `verify_version()` returns an error
unless version == 3. Use `migrate_v2_to_v3()` (one-pass, in-place rewrite) before
opening. v3 adds no byte-alignment changes to the SuperBlock or NQuin layouts.

Legacy v1 sidecars (`.q42.lex`, `.q42.bidx`) and raw SuperBlock streams remain
**readable** via `Q42Lexicon::load_for_q42()` and related fallbacks, but are no
longer emitted by `qualia-cli ingest`.

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

- unified v3 `.q42` volumes (canonical for new ingest)
- legacy v1 raw SuperBlock `.q42` streams (read compatibility)
- legacy `.q42.lex` and `.q42.bidx` sidecars (read compatibility)
- leftover `.c.q42` framed LZ4 profile (read-only; do not emit)
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

This draft adopts the following model as of 2026-06-11:

1. **Unified v3 `.q42`** is the canonical on-disk container for new datasets.
   v2 volumes must be migrated via `migrate_v2_to_v3()` before use.
2. **Embedded Q42LEX and BIDX** replace sidecars for new writes.
3. **Block-local LZ4** compresses each SuperBlock inside the volume; there is
   no separate mandatory compressed transport file.
4. **`.c.q42` is obsolete.** New ingest MUST NOT emit it. A leftover file may
   still be readable as legacy framed transport; it is not a distribution
   twin of a v3 volume.
5. **`q42_reader.rs`** reads the legacy framed LZ4 transport profile only; it
   does not read v3 unified volumes. Use `Q42Volume` / `docs/playground/vfs.js`.
6. **QCHK capability envelopes** should migrate from `.chk` to `.qchk`.
7. **v3 header extensions** add: temporal index section, SHA-256 Merkle root,
   assertion timestamp, and Merkle-DAG history section (see §7 for layout).

## 5. Current Implementation Reality

The repo now converges on one primary write path and several read fallbacks:

| Path | Role | Module |
|------|------|--------|
| v3 unified volume write | canonical ingest output | `q42/q42_volume.rs`, `q42/volume/stream_writer.rs`, `write_quins.rs` |
| v3 unified volume read | mmap open, lex view, BIDX/FIDX/PIDX, block decompress | `Q42Volume`, `docs/playground/vfs.js` |
| inspect / verify / magnet | CLI + Studio Volume Manager | `q42/volume/{inspect,verify,publication,magnet}.rs` |
| v2 → v3 migration | one-pass header upgrade | `q42_volume::migrate_v2_to_v3()` / `qualia q42 compact` |
| v1 sidecar lex read | legacy WordNet / Index trees | `Q42Lexicon::load`, `load_for_q42` |
| v1 raw SuperBlock read | legacy aligned streams | `storage.rs` |
| framed LZ4 transport read | leftover `.c.q42` only | `q42_reader.rs` |

Writers:

- `ingest_ntriples`, `ingest_rdf_xml` → `write_unified_volume()`
- external sort (`ingest_chk`, `ingest_cbor`, import merge) →
  `UnifiedVolumeBuilder`

Readers:

- `chat_ontology`, neuro-symbolic sieve → `Q42Lexicon::load_for_q42()` (v3
  embedded lex; v1 sidecar only as fallback)
- daemon / graph / chat compact / checkpoint → `write_sorted_quins_volume` /
  `Q42Volume`
- GitHub Pages VFS → `docs/playground/vfs.js` (`parseQ42Header`, FIDX/PIDX flags)

## 6. Terminology

### NQuin

The atomic record unit is `NQuin`.

- Size: 48 bytes
- Layout: six little-endian `u64` fields
- Fields: `subject`, `predicate`, `object`, `context`, `metadata`, `parity`

Important clarification:

- The current implementation is not a "42+6 byte split".
- It is a full 48-byte structure with a dedicated 64-bit `parity` field.

### QualiaSuperBlock

The canonical physical storage page is `QualiaSuperBlock`.

- Size: 40,960 bytes
- Alignment: 4,096 bytes (logical page; v2 stores LZ4-compressed payloads)
- Structure: 160-byte header + 40,800-byte quin ledger (850 × 48 bytes)

### Unified v3 volume

- Single `.q42` file with file magic `Q42\0`, version `3`
- Flags (u16):
  - `0x0001` `FLAG_BLOCKS_LZ4` — SuperBlocks are LZ4-compressed
  - `0x0002` `FLAG_OBJECT_SORTED` — ingest sorted by object hash
  - `0x0004` `FLAG_VOLUME_ROOT` — multi-file volume set root + manifest
  - `0x0008` `FLAG_FIELD_RANGES` — FIDX present
  - `0x0010` `FLAG_FIELD_POSTINGS` — PIDX present
  - `0x0020` `FLAG_PERMISSIVE_COMMONS` — public catalog (required for magnets)
  - `0x0040` `FLAG_SANCTUARY` — Sanctuary / restricted; public magnets denied
- Extended header adds: temporal index section, SHA-256 Merkle root
  (streaming chain over SuperBlock payloads), assertion timestamp,
  Merkle-DAG history section, optional natural-person / software-agent DID
  offsets, FIDX/PIDX pointers in `_reserved`

### Embedded sections (v3)

- **Q42LEX**: same binary layout as the obsolete `.q42.lex` sidecar
- **BIDX**: same binary layout as the obsolete `.q42.bidx` sidecar
- **FIDX**: `FIDX` magic, 16-byte header, 48-byte per-block S/P/C range entries
- **PIDX**: compact postings / Bloom for S/P/C; optional, flag-gated

### Legacy sidecars (v1, read-only for new ingest)

- `.q42.lex`: reverse hash-to-string dictionary (standalone file)
- `.q42.bidx`: block-range index (standalone file)

### Transport profile (obsolete)

- `.c.q42`: leftover framed LZ4 quin stream. Not a v3 twin. Do not emit.

### Vault manifest

- `.qualia`: high-level vault or collection descriptor
- expected to reference one or more unified v3 `.q42` artifacts

## 7. Canonical Physical Layout

### `NQuin`

```text
Offset  Size  Field
0       8     subject
8       8     predicate
16      8     object
24      8     context
32      8     metadata
40      8     parity
```

All fields are little-endian.

Producers **MUST** set
`parity = subject ^ predicate ^ object ^ context ^ metadata`
(`NQuin::calculate_parity`). `verify_ecc_parity()` returns true only when that
five-field fold matches. A four-field fold (metadata omitted) is a **verify
Fail** on current volumes. This is stricter than earlier draft prose that
omitted `metadata`.

### `QualiaSuperBlock`

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

- `size == 40960`, `QUINS_PER_BLOCK == 850`
- remaining ledger slots are zero-filled
- `fea_mesh_index_id` is a reserved attachment point, not proof that dense geometry is stored
  inside Q42. As of 2026-07-03, `q42_volume::encode_superblock()` leaves this field zero and the
  OBJ/STL/GLB ingest path writes semantic mesh facts only. The planned computational-asset
  compiler will either wire this field to a versioned analysis-mesh sidecar or supersede it with
  an explicit Q42 relationship that can address multiple meshes and fidelity tiers.

### Unified v3 volume layout

```text
[0..256)                  Q42VolumeHeader
[lex_offset ..)           Q42LEX blob (see §8)
[bidx_offset ..)          BIDX blob (see §8)
[block_dir_offset ..)     block_count × BlockDirectoryEntry (16 bytes each)
[data_offset ..)          concatenated LZ4 payloads (lz4_flex prepend_size)

BlockDirectoryEntry:
  [0..8]   rel_offset   u64 LE   — byte offset from data_offset
  [8..12]  comp_len     u32 LE
  [12..16] uncomp_len   u32 LE   — always 40960 for current ingest
```

Header fields (little-endian, v3 — 256 bytes total):

```text
[0..4]    magic                  "Q42\0"
[4..6]    version                u16 = 3   (v2 hard-rejected; use migrate_v2_to_v3())
[6..8]    flags                  u16
[8..16]   lex_offset             u64
[16..24]  lex_length             u64
[24..32]  bidx_offset            u64
[32..40]  bidx_length            u64
[40..48]  block_dir_offset       u64
[48..56]  block_dir_length       u64
[56..64]  data_offset            u64
[64..72]  data_length            u64
[72..80]  block_count            u64
[80..84]  block_size             u32 = 40960
[84..88]  quins_per_block        u32 = 850
— v3 extensions (carved from former reserved bytes) —
[88..96]  temporal_index_offset  u64   (0 if no temporal index)
[96..104] temporal_index_length  u64
[104..136] merkle_root           [u8; 32]  SHA-256 of DAG root node
[136..144] assertion_timestamp   u64   ms since Unix epoch (last write)
[144..152] dag_root_offset       u64   (0 if no DAG history)
[152..160] dag_root_length       u64
[160..168] natural_person_did_offset  u64
[168..176] software_agent_did_offset  u64
[176..256] _reserved             [u8; 80]
           [176..192] volume-set manifest offset/length when FLAG_VOLUME_ROOT
           [192..208] FIDX offset/length when FLAG_FIELD_RANGES
           [208..224] PIDX offset/length when FLAG_FIELD_POSTINGS
```

Detection: `is_v2_volume()` checks magic `Q42\0` at offset 0. `verify_version()` rejects
anything other than version 3; to open a v2 file call `migrate_v2_to_v3()` first.

Ingest sorts all quins by **object hash** before chunking so BIDX ranges are
ascending and binary-searchable.

## 8. Canonical Artifact Set

### Unified v3 `.q42` (canonical)

Recommended meaning for all new datasets:

- single self-contained file, version 3
- embedded lexicon and block index
- LZ4-compressed SuperBlocks
- optional temporal index section (bi-temporal PROV-O quins)
- optional Merkle-DAG history section (`DagNode` chain from `git_bridge.rs`)
- SHA-256 Merkle root over the current DAG tip
- memory-mapped via `Q42Volume::open()`

Matches:

- `crates/qualia-core-db/src/q42_volume.rs`
- `crates/qualia-core-db/src/git_bridge.rs` (`DagStore::serialize()`)
- `crates/qualia-cli/src/ingest.rs`
- `scripts/fetch_wordnet.sh` (outputs `wordnet.q42` only)

### `.p64` weight container (AOT LLM)

The byte-accurate normative layout, including all fixed-record offsets, flags,
tokenizer encoding, manifold records, checksums, and reader validation rules,
is specified separately in
[`p64-weight-container-standard.md`](p64-weight-container-standard.md).

A **separate** artifact from the semantic graph `.q42` above — it carries an **independent section
magic** (`b"p64\0"`, never collides with `Q42\0`). It is the Ahead-Of-Time-compiled home for an LLM's
weights, so the engine boots **zero-parse** instead of re-parsing a GGUF on every load. Produced by the
AOT compiler `q42::p64_weight::compile_gguf_to_p64` (also reachable from WASM), cached in OPFS, and
consumed by the resident-weight loader (`P64TensorIndex::from_p64` → GPU arenas). **Inference runs from
the `.p64`; the source GGUF is never re-touched.**

> **Producer / runtime boundary.** The **engine runs the `.p64`**. The WGSL *forge* transcodes
> GGUF → `.p64` and certifies the GPU kernels against a CPU oracle — it does **not** run inference, and
> throughput (tok/s) is the engine's, not the forge's.

Unlike the semantic `.q42`, the weight container carries **no `NQuin` scaffold**: the 48-byte declarative
`q42` quin manages truth/provenance, while the 64-byte-aligned `p64` manages pure mathematical inference
with zero-copy relative (WASM-native) pointers.

```text
P64WeightHeader (64 B, #[repr(C, align(64))], little-endian)
  magic                 "p64\0"  u8[4]       — distinguishes from the semantic .q42 (Q42\0)
  version               u16 = 3
  flags                 u16                   — endianness (P64_FLAG_LITTLE_ENDIAN)
  — 32-bit RELATIVE offsets (WASM-native; bytes from start of file) —
  role_table_offset     u32                   — tensor → semantic role map
  tensor_table_offset   u32                   — P64TensorEntry descriptor table
  tokenizer_offset      u32                   — embedded tokenizer (self-contained; no GGUF needed)
  hparams_offset        u32                   — model hyperparameters
  string_table_offset   u32                   — centralized string pool
  checksum_offset       u32                   — CRC-32C tamper-evidence
  manifold_table_offset u32                   — 10-D ManifoldCoordinate10D table (64 B/entry)
  tensor_count          u32
  page_size             u32                   — blob alignment (default 16 KB; page_log2 = 14)
  reserved              u8[20]                — pads to exactly 64 B

Manifest               tensor_count × P64TensorEntry (64 B each)
  role, dtype, rank, dims, relative blob offset/length. Roles (P64_ROLE_*):
  ATTN_K 0, ATTN_V 1, ATTN_Q 2, ATTN_OUTPUT 3, FFN_GATE 4, FFN_UP 5, FFN_DOWN 6,
  ATTN_NORM 7, FFN_NORM 8, TOKEN_EMBD 9, OUTPUT 10, OUTPUT_NORM 11;
  UNKNOWN 0xFFFE (preserved byte-for-byte, unconsumed), LAYER_GLOBAL 0xFFFF (non-layer tensor).

Tensor blobs           — opaque, contiguous, page-aligned (page_log2) quantized bytes for
                         single-fetch mmap / zero-copy WebGPU bind.
```

Matches `crates/qualia-core-db/src/q42/p64_weight.rs` (`P64WeightHeader`, `P64TensorEntry`,
`compile_gguf_to_p64`, `P64TensorIndex::from_p64`, `P64_ROLE_*`). Integrity is table-less CRC-32C; the
16 KB page alignment lets the resident-weight upload map blobs straight into the GPU arenas. The semantic
`.q42` and the weight `.p64` are siblings: both cache-aligned, distinct magics, distinct jobs.

> **Historical note.** A pre-P64 `Q42W` weight layout (144-byte `Q42WeightHeader`, NQuin-scaffolded,
> magic `"Q42W"`) preceded this container; it is **superseded** by `.p64` and retained only as migration
> test fixtures (`q42::p64_weight` tests).

### Embedded Q42LEX layout

Same as legacy `.q42.lex`:

```text
Header (32 bytes)
  [0..8]   magic          "Q42LEX\0\0"
  [8..16]  entry_count    u64 LE
  [16..24] strings_offset u64 LE
  [24..32] version        u64 LE

Index (entry_count × 16 bytes, sorted by hash)
  [0..8]   hash           u64 LE
  [8..16]  str_off        u64 LE

String blob
  repeated: u16 LE length + UTF-8 bytes
```

### Embedded BIDX layout

Same as legacy `.q42.bidx`:

```text
Header (16 bytes)
  [0..4]   magic          "BIDX"
  [4..8]   version        u32 LE = 1
  [8..12]  block_count    u32 LE
  [12..16] reserved       u32 LE = 0

Index (block_count × 16 bytes)
  [0..8]   min_hash       u64 LE   — min object hash in block
  [8..16]  max_hash       u64 LE   — max object hash in block
```

BIDX indexes **object-hash ranges** (not subject-hash). This matches ingest,
`Q42Volume::bidx_blocks_for_hash()`, and the intended literal lookup pattern
`?s ?p "literal"`.

### Legacy raw v1 `.q42`

Historical profile still readable:

```text
.q42 file = N × QualiaSuperBlock   (40,960-byte stride, uncompressed)
```

Requires separate `.q42.lex` and `.q42.bidx` sidecars for full retrieval.

### Legacy `.c.q42` transport (do not emit)

Historical framed profile. Browser Pages now demand-page **unified v3** volumes
(`vfs.js`). A leftover `.c.q42` may still decode through `q42_reader.rs`.

```text
Per chunk:
  [0..8]   block_id      u64 LE
  [8..12]  comp_len      u32 LE
  [12..16] uncomp_len    u32 LE
  [16..]   payload       lz4_flex::compress_prepend_size output
```

### `.qualia`

Vault or collection manifest — references `.q42` data files. For v3 volumes,
lexicon and block index are embedded; manifest sidecar pointers are optional
legacy hints only.

## 9. Legacy Profiles (read-only compatibility)

The engine retains read paths for:

1. **v1 raw SuperBlock stream** + `.q42.lex` + `.q42.bidx`
2. **Framed LZ4 transport** (`.c.q42` or mislabeled `.q42`) via `q42_reader.rs`
3. **Pre-v2 WordNet trees** under `Local_LIbraries/wordnet/` until re-ingested

New ingest MUST NOT emit v1 sidecars. Re-ingest with `qualia-cli ingest` or
`scripts/fetch_wordnet.sh` to upgrade.

## 10. Contradictions Resolved or Remaining

### 10.1 Resolved: single-file vs sidecar index

**Resolved (2026-06-09):** v2 embeds lex + BIDX. Sidecars are legacy read-only.

### 10.2 Resolved: file-level magic and version

**Resolved:** v3 uses magic `Q42\0` and version `3` at offset 0. v2 files are
hard-rejected by v3 builds; `migrate_v2_to_v3()` upgrades the 168-byte reserved
region to the v3 extended header (temporal, merkle_root, assertion_timestamp,
DAG section pointers).

### 10.3 Resolved: compression model

**Resolved:** block-local LZ4 inside the volume. Whole-file stream compression
is legacy transport only.

### 10.8 Resolved: v3 temporal and DAG header fields (2026-06-11)

**Resolved:** v3 carves six new fields from the former `_reserved` region
(bytes 88–159):
- `temporal_index_offset/length` — byte range of the optional temporal index
- `merkle_root [u8;32]` — SHA-256 of the current Merkle-DAG tip
- `assertion_timestamp u64` — last-write wall-clock ms
- `dag_root_offset/length` — byte range of the serialized `DagStore`

All fields are `0` until the corresponding section is written. v3 builds that
read a v3 file with `dag_root_length == 0` simply have no DAG history yet.

### 10.4 Resolved: browser VFS (2026-08-15)

`docs/playground/vfs.js` reads unified v3 (`parseQ42Header`, FIDX/PIDX flags,
block directory, LZ4 SuperBlocks). Pages boot Schema.org 30.0; WordNet is
optional (~127 MB Release asset), not a required sidecar pair.

### 10.5 Remaining: BIDX dimension prose drift

Some older architecture docs describe BIDX in terms of subject hash. The
implementation and this draft use **object-hash ranges** exclusively unless a
future versioned revision changes the index dimension.

### 10.6 Remaining: QCHK extension collision

QCHK capability envelopes are still named `.chk` in parts of the repo.
Migration to `.qchk` is not yet complete.

### 10.7 Remaining: naming drift `.q42` vs `.qla`

Older storage comments still refer to `.qla`. Normalize to `.q42` in future
doc sweeps.

## 11. Proposed Canonical Rules

1. **New writes** produce unified v3 `.q42` only (magic `Q42\0`, version 3). v2 files must be migrated with `migrate_v2_to_v3()` before opening.
2. Q42LEX and BIDX blobs inside a v2 volume use the same layouts as legacy
   sidecars.
3. SuperBlocks inside v2 are LZ4-compressed individually; uncompressed size is
   always 40,960 bytes per block entry.
4. Ingest sorts quins by object hash before block assignment.
5. BIDX indexes object-hash min/max ranges per block.
6. `.q42.lex` and `.q42.bidx` sidecars are legacy; readers must fall back to
   them when opening non-v2 files.
7. `.c.q42` MUST NOT be emitted. When a leftover file is present it is legacy
   framed transport only — never a required twin of a v3 volume.
8. `q42_reader.rs` is legacy transport only — not a canonical v3 reader.
8a. Public magnets fail closed unless `FLAG_PERMISSIVE_COMMONS` is set and
    `FLAG_SANCTUARY` is clear. Unmarked personal volumes do not become
    addressable by rewriting a flag.
8b. Quin parity is the five-field XOR. Verify Fail on four-field leftovers.
9. `.qchk` is the target canonical capability-envelope extension.
10. `.qualia` is the vault-manifest extension.
11. SlgArena / hot-path memory budget remains **42 MB**, not whole-file mmap
    of multi-gigabyte volumes; block fetch decompresses one SuperBlock at a
    time into caller-supplied buffers.

## 12. Open Questions

1. Should `validation_checksum` remain a real field if `parity` already exists?
2. Should BIDX move to subject-hash ranges in a future v4, or stay
   object-indexed permanently? (v3 retains object-hash BIDX; this remains open for v4.)
3. Should v3 receive an explicit media type (e.g.
   `application/vnd.qualia.q42+v3`) before IETF submission?
4. *(Resolved 2026-08-15.)* Playground VFS reads v3 natively. No framed-transport
   translation step.
5. Which public media type should be chosen for `.qchk`?
6. Should `.qualia` be Turtle-only, or also allow N3 / CBOR-LD projections?
7. Should file association be wired first in the Flutter shell, with Tauri
   treated as secondary or legacy?

## 13. Remaining Cleanup Mandate

1. [x] Converge ingest on unified v2 volume (`q42_volume.rs`).
2. [x] Embed lex + BIDX; stop emitting sidecars from `qualia-cli ingest`.
3. [x] Wire `Q42Lexicon::load_for_q42()` and sieve lex paths for embedded lex.
4. [x] Simplify `fetch_wordnet.sh` to output single `wordnet.q42`.
5. [x] Update WASM playground VFS for unified v3 volumes (`vfs.js`).
6. [ ] Rename QCHK public references from `.chk` to `.qchk`.
7. [ ] Add canonical test vectors:
   - one single-block v3 `.q42` with FIDX/PIDX and five-field ECC
   - one multi-block v3 `.q42` with lex entries
   - one Commons-flagged volume that may mint a magnet
   - one Sanctuary volume that MUST fail-closed
   - one `.qchk`
8. [ ] Propose media types for IETF drafting (`application/vnd.qualia.q42+v3`).
9. [ ] Finalize minimal `.qualia` manifest schema for v3 (sidecar terms
   omitted).
10. [x] Stay on v3 (R10). Lex LZ4 inside the same file; no v4 fork.
