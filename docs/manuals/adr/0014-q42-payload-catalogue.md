# ADR 0014 — Q42 payload catalogue for oversized prose (not media)

- **Status:** Accepted (design + flag stub; writer deferred).
- **Date:** 2026-09-29
- **Branch:** `0.0.40.1-dev`
- **Relates:** Q42 format internal draft; HCF/HMC transparent bundles; integrity toolkit.

---

## Context

Lexicon string lengths are stored as `u16` (max 65535 UTF-8 bytes). Complete ingest
fails closed on longer terms (`LexError::TermTooLong`). Some ontologies (long OWL
annotations, chemical prose) need those bytes for *display/export* without putting
them on the BIDX hot path.

Separately, images / audio / video / SVG / `.p64` / `.10d` belong in a **transparent
`.hmc` (QBDL) pack** beside the graph volume — not inside the graph `.q42`.

## Decision

1. Introduce an optional Q42 section **Payload Catalogue**: length-prefixed blobs
   keyed by content hash. Quin/lex holds only the hash ref for non-query prose.
2. Reserve header flag **`FLAG_PAYLOAD_CATALOGUE = 0x0080`** (stub in
   `q42_volume.rs`). Offset/length reclaim reserved header slots in a later writer.
3. Catalogue payloads are **not** indexed by BIDX by default.
4. **Non-goal:** media bytes. Media and multi-asset packs use `.hmc`
   (`crates/qualia-core-db/src/bundle/`). Do not compress HMC entries.

## Consequences

- Older readers ignore the unknown flag / section (fail soft).
- Integrity tooling (`audit-source`, `verify-graph`) already counts oversize terms
  and points operators at this ADR or `--omit-preset=comment-gloss`.
- Full catalogue writer ships after import-aligned verify is green.

## Non-goals

- AEAD binding tiers on HMC (flags reserved elsewhere; unimplemented).
- Per-entry HMC compression (would break transparency).
- Replacing Princeton WordNet assets with OEWN in this ADR.
