# Mutable Q42 Sessions: Checkpoint and Journal Proposal

Status: **design proposal, not an allocated format or shipped API**
(2026-10-03). Motivated by Rolling Commons' RTS world, but intended as a
general QualiaDB capability. The game must consume the accepted QualiaDB
implementation through `wasm-full`; it must not implement a private save
format. This proposal does not change QualiaDB's licence or publication policy.

## Recommendation

Use a **sealed `.q42` snapshot plus a versioned append-only Q42 journal** for
mutable sessions. Provisional extension: `.q42j` (Q42 journal). The journal
is a new *persistence profile* of the Q42 family, not a second semantic
database or a mutable rewrite of the unified Q42 volume. Retain the existing
48-byte `NQuin` exactly as the semantic atom. Do not allocate `.t48` or
"T48" as a public file type at this stage: 48 bytes describes one NQuin,
not a transaction, journal frame, save, or audio asset.
This journal is for **mutable semantic world state and event receipts**,
not a universal wrapper for every mutable file. Qualia Audio sessions,
authored Vibe source and large media have their own appropriate content
contracts and can reference Q42 identities without being encoded as NQuins.

The in-memory world is `base.q42 + committed q42j transactions`. Periodic
compaction creates a new sealed `.q42` checkpoint and starts a journal
against that checkpoint. Immutable game assets remain `.10d`/HCF/audio in
the versioned HMC content pack. The mutable save stores pack references and
digests, not asset payloads. Optional P64 weights remain separate.

## Evidence in the current tree

- `NQuin` in `crates/qualia-core-db/src/lib.rs` is six little-endian `u64`
  fields (`subject`, `predicate`, `object`, `context`, `metadata`, `parity`),
  48 bytes total. Canonical parity folds **all five** non-parity fields.
- `docs/manuals/standards/q42-format-internal-draft.md` describes the
  unified v4 `.q42` as a 256-byte header, embedded lexicon/index sections,
  and independently addressed 40,960-byte SuperBlocks (850 NQuins each).
  It is a sealed, indexed snapshot layout; ordinary transaction appends
  would require index/header/root maintenance and failure handling.
- `crates/qualia-core-db/src/wal.rs` already appends packed 48-byte NQuins
  after a 32-byte previous-DAG-hash header on native targets. Its WASM
  methods currently return success without persisting mutations. The native
  log also has no explicit multi-record transaction boundary in that layout.
- `crates/qualia-core-db/src/q42/volume/publication.rs` has fail-closed
  Commons/Sanctuary classification. A local player save must default to
  restricted/Sanctuary and never acquire public magnet status implicitly.
- `crates/qualia-audio/`, `crates/qualia-core-db/src/audio/` and
  QualiaPortal's acoustic APIs already form the audio capability family.
  PCM and large media payloads do not belong in NQuins or the mutable Q42
  journal; Q42 holds semantic references and event receipts.

## Candidate `.q42j` contract

### File header

The header identifies journal magic/version, byte order, feature flags,
world/save identity, base `.q42` digest and generation, required content-pack
digest(s), schema/rule version, and the active publication/privacy class.
Allocate exact byte offsets only after codec and migration tests; do not
reuse Q42 v4 magic with a different body. The reader rejects unknown
required flags and a base or pack mismatch.

### Transaction frames

Use bounded, length-delimited frames with a type, sequence number, tick,
transaction ID, previous committed-frame digest, payload length/digest,
and integrity check. A transaction can contain:

- a typed game command and its deterministic validation/rule receipt;
- add/remove/upsert operations over canonical 48-byte NQuins (with exact
  prior-record identity for removals and explicit conflict policy);
- lexicon additions or name bindings needed for new runtime entities;
- event/output receipts and a declared resulting state digest;
- a final **commit** record covering the whole transaction.

Only complete, validly committed transactions affect recovered state.
Unknown optional frame types can be skipped by length; unknown required
types fail closed. Set maximum frame/transaction sizes and bounded parser
scratch. A frame boundary is **not** forced to 48 bytes: its envelope has
different responsibilities from an NQuin. NQuin arrays inside a frame
remain exact 48-byte records with five-field parity verification.

### Append, checkpoint and recovery

1. Validate a proposed command against schema, permissions, SHACL/N3 and
   the deterministic reducer in memory. Determine its complete Q42 delta
   and receipt before durable append.
2. Append the transaction frames and a commit marker; flush the journal
   through the Qualia storage adapter. Expose success to the game only after
   the selected durability contract is satisfied. Never return success from
   a stub, short write, quota failure or lost lock.
3. On open, verify header/base/pack versions, then scan in sequence. Apply
   only the longest complete, hash-valid, parity-valid committed prefix.
   Ignore a torn final transaction; fail or offer recovery for corruption
   earlier in the committed prefix. Replay is idempotent by transaction ID.
4. Checkpoint by writing and verifying a **new** sealed Q42 generation,
   then atomically selecting that generation with a small validated
   generation pointer/manifest. Retain the old generation until the new
   snapshot and cursor are proven readable. A journal reset is last.
5. Save export/backup packages the selected snapshot and journal plus
   version/digest manifest. Import validates both and never silently
   reinterprets a missing game pack or incompatible rule version.

For browser use, implement the journal storage adapter in QualiaDB over
OPFS. A dedicated worker and synchronous access handle are suitable
candidates where supported; test flush, exclusive lock, quota exhaustion,
interrupted write and browser restart. OPFS storage may be evicted, so
player-facing backup/export remains a separate requirement. Native uses
its own fsync/locking adapter with the same codec and replay semantics.

### Privacy and audio

Default player saves to local restricted/Sanctuary classification; do not
publish journal bytes or digests through public Q42 transport. Sensitive
identity/LifeChapter content needs explicit scope and deletion semantics.
Encryption at rest is a separate, explicit feature decision and must not be
implied by the extension or by OPFS privacy.

Audio event state in a journal should be a Q42-linked cue, timing and
provenance record (for example, an outage alert), not PCM, spectral sheets,
MIDI streams or generated sound. Use Qualia Audio and HMC/content assets for
the latter. On replay, presentation may re-trigger cues by policy without
duplicating authoritative events.

## Why `.q42j`, not `T48`

| Option | Assessment |
|---|---|
| Mutate unified `.q42` in place | Its indexes, compressed SuperBlocks, roots and publication metadata make partial writes and crash recovery hard; retain it as a verified checkpoint. |
| Existing raw Quin WAL unchanged | Reuses 48-byte atoms but lacks browser durability, explicit transaction boundaries, versioned frame checks and a clear checkpoint cut. Evolve it rather than merely rename it. |
| New `T48` file of fixed 48-byte frames | Conflates semantic NQuins with transaction envelopes; 48 bytes cannot conveniently carry commit identity, length, version, sequence and strong integrity for a variable-size transaction. |
| Q42 snapshot + `.q42j` | Reuses the Q42 semantic ABI and existing WAL direction while giving mutable commits and recovery an explicit, versioned contract. **Recommended for a prototype and conformance review.** |

The extension is provisional until format ownership, registry collision,
native/WASM codec, migration policy and conformance tests are approved.
If QualiaDB already has an equivalent durable journal elsewhere, adopt and
version it instead of creating a duplicate `.q42j` implementation.

## Upstream work plan and release gate

1. Audit `wal.rs`, Q42 volume writer/reader, graph mutation APIs, OPFS VFS
   and existing durable transaction code. Decide whether to evolve the WAL
   module or a more complete existing store. Record the exact crate/API.
2. Write a short normative format draft for header, frame types, ordering,
   add/remove semantics, lexicon deltas, checksums/hashes, commit, version
   negotiation, privacy flags, checkpoint pointer and migration.
3. Implement a shared no-heap hot-path parser/encoder over caller buffers
   and bounded cold construction; supply native and WASM storage adapters.
   Remove the WASM success-without-persistence stubs from the selected path.
4. Build conformance vectors: one/large transaction, multi-Quin atomicity,
   new entity/lexicon, duplicate transaction, torn header/payload/commit,
   altered committed byte, wrong base/pack, quota failure, failed checkpoint,
   old/new format version, Sanctuary publication denial.
5. Integrate a mutable Q42 session API and Vibe read/proposal capabilities.
   Migrate Rolling Commons' temporary N3 text and event-line save to the
   accepted Qualia interface.
6. Prove 1,000 game commands native and in browser: checkpoint, interrupt,
   reopen, replay and compare canonical graph/event digests. Record file
   sizes, append latency, recovery time and memory high-water against a
   declared device/browser baseline. Only then close game QG-05/QG-15.

Open design choices: exact filename/extension, transaction frame layout,
hash/checksum algorithm, compaction threshold, concurrency/multi-writer
scope, browser durability level and whether a save export wraps both files
in a separate Qualia container. None changes the 48-byte NQuin ABI.
