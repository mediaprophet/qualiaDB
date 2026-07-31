# ADR 0008: FrameLayout ABI for the NQuin's Computational Bytes

## Status
Accepted (2026-06-21, 0.0.28)

## Context
[ADR 0001](0001-the-48-byte-qualia-quin-alignment.md) fixes the `QualiaQuin`/`NQuin` at
exactly 48 bytes (`6 × u64`: subject, predicate, object, context, metadata, parity). Of
those, roughly **42 bytes are semantics** (the field hashes / packed literals) and roughly
**6 bytes are computational support** — an opcode, datatype tags, per-modality flags, a
clock, and ECC parity — which is what makes a Frame both *data* and *executable code*.

Those computational bits were being allocated **ad-hoc across many modules**, and two
conventions had silently collided:

1. **Object float tag.** Computed `f32` values were tagged `0b001 << 60` in the object
   field — the *same* bits the resolver uses for `xsd:integer`. A stored computed float was
   therefore misread as an integer (the long-standing "AGENTS.md §4-D" conflict).
2. **Truth degree.** Fuzzy/probabilistic modalities read a degree as a 16-bit fixed value
   `(metadata & 0xFFFF) / 65535`, while other code stored a raw `f32` — two encodings for
   the same concept.

There was also a latent risk in the `metadata` field, whose high 32 bits are used by
several subsystems (a tensor-bake clock, per-modality flags, an ODRL sensitivity tier, the
quin-type nibble, and a routing lane) with **overlapping bit positions**.

## Decision
Introduce `crates/qualia-core-db/src/frame_layout.rs` as the **single canonical registry**
for the NQuin's computational bytes. Every modality reads/writes those bits through it, and
collision invariants are enforced by unit tests.

1. **Predicate** = `[0..7]` opcode · `[8..62]` property-path hash · `[63]` defeater
   (co-resident; must not overlap — enforced).
2. **Object inline datatype tags** are owned by `resolver.rs` (the serialiser) and
   re-exported by `frame_layout`. A new tag **`INLINE_TAG_FLOAT = 0b101 << 60`** is
   formally allocated for inline `xsd:float` (low 32 bits = IEEE-754 `f32`), ending the
   float-vs-integer clash. The VM (`core.rs`), `frame_layout`, and the resolver all agree;
   the resolver's reserved range narrows to `0b0110–0b0111`.
3. **Metadata is a ROLE-KEYED OVERLAY, not a flat field.** Its 32 high bits are a union of
   overlays valid only for a particular quin *role* (tensor-bake clock `[32..60]`,
   per-modality flags `[50..59]`, ODRL sensitivity `[56..59]`, quin-type nibble `[60..63]`
   whose `[61..62]` doubles as the routing lane). Two overlays may share bits **iff** their
   roles are mutually exclusive. The one always-true invariant — **the low-32 payload is
   disjoint from every high overlay** — is the one the tests enforce.
4. **`quin_type` is NOT relocated** off `[60..63]`. Every lower slot lands inside the
   tensor-bake clock `[32..60]`, which would be a *worse* (cross-role) collision than the
   documented, role-exclusive overlap with the routing lane.
5. **Truth degree is unified** to a single canonical encoding: an IEEE-754 `f32` in the low
   32 metadata bits (`frame_layout::truth_degree` / `with_truth_degree`).

## Consequences
- **Positive:** One source of truth for the computational ABI; no two modalities can
  silently disagree on a bit. Computed floats now serialise correctly as `xsd:float` end to
  end (VM → frame → resolver), instead of as a bogus integer or `<quin:hash/...>` IRI.
- **Positive:** The metadata field's real (oversubscribed) structure is documented honestly
  and the load-bearing invariant is test-enforced, rather than an assumed-flat map.
- **Neutral:** `metadata` overlays remain role-exclusive by construction — correct, but it
  means future fields must be added through `frame_layout` with a collision test, not by
  grabbing "free" bits.
- **Negative / migration:** the object float-tag value changed (`0b001 → 0b101`). This is a
  frame-format change, accepted because `.q42` is pre-release (no deployed vaults/peers) and
  the old encoding was already ambiguous. No on-disk migration is provided; new writes use
  the allocated tag.

## References
- `crates/qualia-core-db/src/frame_layout.rs` (the ABI + enforcement tests)
- `crates/qualia-core-db/src/resolver.rs` (inline-tag spec table + `xsd:float` decode)
- Project memory `project-frame-layout-abi`; `ALGEBRA_MANIFOLD_PLAN.md`
- Supersedes the "known conflict, do not fix unilaterally" note in AGENTS.md §4-D.
