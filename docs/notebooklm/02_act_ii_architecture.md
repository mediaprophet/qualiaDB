# Act II — Architecture

> *The 48-byte NQuin and the 42-megabyte SLG Arena.*

---

## Thesis

> **Every semantic fact in the engine is exactly forty-eight bytes wide, and
> the engine's working memory is exactly forty-two megabytes. These two
> numbers are not arbitrary; they are the discipline that makes the rest of
> the system possible.**

---

## Voice-over script

### Shot 1 — A single NQuin, hex-dumped, fills the screen. Each of the six `u64` fields is labeled. [SLOW]

> This is an NQuin. [PAUSE]
> Forty-eight bytes. Six unsigned sixty-four-bit integers. [PAUSE]
> Every semantic fact in the engine — every triple, every norm, every claim,
> every credential — fits in one of these. [PAUSE]

### Shot 2 — The six fields are highlighted one at a time. [ITEM]

> The first field is the subject. The high bit flags a meta-statement. [PAUSE] [ITEM]
> The second field is the predicate. The high bit is the defeater sentinel.
> The low byte is the modality opcode. [PAUSE] [ITEM]
> The third field is the object. The high bit flags a topological pointer.
> The next three bits are the type tag — IRI, integer, decimal, boolean. [PAUSE] [ITEM]
> The fourth field is the context. The sensitivity class lives in the top
> byte. The contract hash lives in the bottom fifty-five bits. [PAUSE] [ITEM]
> The fifth field is the metadata. The routing lane, the Lamport clock,
> and the modality payload. [PAUSE] [ITEM]
> The sixth field is the parity — an XOR fold of the other five. A
> tamper-evident checksum. [END LIST] [PAUSE]

### Shot 3 — A second NQuin appears. It is identical except for one bit. The parity field changes. [SLOW]

> Touch any field, and the parity field changes. [PAUSE]
> The engine does not need a separate hash table to detect tampering; the
> tampering is visible in the wire format. [PAUSE]

### Shot 4 — Cut to a wide view: 917,504 NQuins arranged in a grid. [ITEM]

> This is the SLG Arena. [PAUSE]
> Forty-two megabytes. Nine hundred seventeen thousand, five hundred and
> four slots. Fixed capacity. [PAUSE] [ITEM]
> No allocation in the hot path. [PAUSE] [ITEM]
> No garbage collector. [PAUSE] [ITEM]
> No `Vec`. No `String`. No `Box`. [PAUSE] [ITEM]
> The caller supplies the output buffer. The engine writes into it. [END LIST] [PAUSE]

### Shot 5 — A rule fires. A single NQuin is read; a verdict is written to a caller-supplied buffer. [SLOW]

> This is the discipline. [PAUSE]
> Forty-eight bytes for a fact. Forty-two megabytes for a working memory. [PAUSE]
> Everything else is built on top of these two numbers. [PAUSE]

### Shot 6 — The grid spins into a sphere. The cyan-blue node graph reappears behind it. [SLOW]

> The reason this matters is not the speed. [PAUSE]
> The reason this matters is the audit. [PAUSE]
> A system that knows exactly how much memory it is using, and exactly
> where every fact lives, can be checked. [PAUSE]
> It can be checked by a regulator. It can be checked by a court. It can
> be checked by the person whose data it holds. [PAUSE]

### Shot 7 — Title card: **The NQuin ABI.** [SLOW]

> This is the wire format. [PAUSE]
> This is what we mean by "the engine." [PAUSE]

---

## On-screen notes

- **Shot 1:** Real bytes from `crates/qualia-core-db/src/lib.rs` `NQuin` definition. The hex dump is real, not stock. The six fields are labeled with the names from `AGENTS.md §1`.
- **Shot 2:** Each field highlight is a 1.5-second beat. The camera does not move; the highlight does.
- **Shot 3:** A single bit flip in `subject[63]`. The `parity` field changes. The XOR fold is visible.
- **Shot 4:** A grid of 917,504 cells, each 48 bytes wide. The number is exact. The camera pulls back to show the grid filling the screen.
- **Shot 5:** A deontic rule firing. Source: `crates/qualia-core-db/src/deontic_logic.rs` `evaluate_deontic_contract`. The camera is close; the verdict is a single NQuin.
- **Shot 6:** The grid morphs into a sphere; the cyan-blue node graph appears behind it. The music resolves on a major chord.
- **Shot 7:** Title card. Same typography as the master title card.

---

## Source code anchors

- `crates/qualia-core-db/src/lib.rs` — `pub struct NQuin { subject, predicate, object, context, metadata, parity }`.
- `crates/qualia-core-db/src/foundation/frame_layout.rs` — `INLINE_TAG_INTEGER`, `INLINE_TAG_DECIMAL`, `INLINE_TAG_BOOLEAN`, `INLINE_TAG_FLOAT`, `pack_float_object`, `unpack_float_object`, `parity_valid`, `object_datatype_tags_are_distinct`.
- `crates/qualia-core-db/src/governance/webizen.rs` — `pub struct SlgArena`, `pub const ARENA_BYTES: usize = 42 * 1024 * 1024`, `pub const ARENA_SLOTS: usize = 917_504`.
- `AGENTS.md §0` and `§1` — the universal Quin bit layout table (the canonical reference).
- `CLAUDE.md §6` — the core invariants (zero-heap, 48-byte, 42 MB, `q_hash`).

---

## Duration

Approximately 120 seconds. This is the act where the viewer learns the grammar of the engine.
