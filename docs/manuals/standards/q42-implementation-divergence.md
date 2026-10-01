# Q42 / HCF implementation divergence log

- **Date:** 2026-09-29  
- **Branch:** `0.0.40.1-dev`  
- **Scope:** Integrity-relevant claims in `docs/manuals/standards` vs live code.  
- **Rule:** Prefer this log + targeted fixes over silently rewriting standards to match bugs.

| Claim | Manual | Code locus | Status | Evidence |
|-------|--------|------------|--------|----------|
| `verify-graph` proves import encoding | implied by integrity UX | Was `hash_token` whitespace NT; now `--encoder=import` (default) uses Rio + `q_hash` | **Fixed (0.0.40.1-dev)** | `graph_proof::prove_import_rdf_q42_equivalence`; legacy gated `--encoder=legacy-semantic` |
| Blank-bearing encode-fidelity vs RDF isomorphism | integrity UX / consultant Claim A | Sets match + blanks → CLI exit 0 + NOTICE; `--require-isomorphism` fail-closed | **Match (policy)** | `handlers/misc.rs` `handle_verify_graph`; studio still shows BlankNodeCaution |
| Complete ingest retains lexicon | q42-format-internal-draft | `IngestMode::Complete` | Match | `ingest.rs` |
| StripLiterals empties lexicon (data loss, not compression) | format draft / ingest docs | `IngestMode::StripLiterals` | Match | `ingest.rs` module docs |
| Empty lex on “Complete” volumes possible | honesty in inspect | `princeton.q42` / hashed-only writes | **Diverge / caution** | `Q42InspectReport::lexicon_has_no_terms`; integrity page receipt C |
| Lex term max 65535 UTF-8 bytes | format / lex | `q42_lex` u16 length; `TermTooLong` | Match | `q42_lex.rs`; `integrity_omit::MAX_LEX_TERM_BYTES` |
| Some test helpers truncate lex terms | — | test-only serialize path | **Manual stale / test debt** | `q42_lex` test helper `.min(65535)` |
| HMC is transparent; no entry compression | HCF + bundle module docs | `bundle/` concatenates intact files | Match | `bundle/mod.rs` |
| HMC uses Bao / BLAKE3 1-KiB tree | `hypermedia-content-format-hcf.md` | CRC-32C whole-file + per-entry SHA-256 | **Diverge** | `bundle/reader.rs`, `bundle/format.rs` — code authoritative for shipped `.hmc` |
| HMC AEAD Levels 0–3 | HCF draft | flags reserved; not enforced | **Unimplemented** | defer |
| Media catalogue inside `.q42` | Gemini HMC note assumption | Wrong default | **Rejected** | Media → `.hmc`; prose overflow → `FLAG_PAYLOAD_CATALOGUE` (ADR 0014 stub) |
| `did:q42` = HMC byte offset | Gemini note | HMC uses key+offset index | **Rejected** | `bundle/format.rs` `BundleEntry` |
| Distinct `p64\0` magic | p64 standard | p64 writer/reader | Match | must not `verify-graph` as Q42 |
| Payload catalogue section | ADR 0014 | `FLAG_PAYLOAD_CATALOGUE = 0x0080` stub only | **Unimplemented** (planned) | ADR 0014 |
| Q42 v4 volume version | this draft §1.3 (2026-09-29) | `Q42_VERSION_V4 = 4`; writers stamp v4; gates accept 3\|4 | **Implemented (0.0.40.1-dev)** | `q42_volume.rs`, `stream_writer.rs`, `range_volume.rs`, `vfs.js` |
| Namespaced paged Q42LEX v4 | this draft + ADR 0015 | `q42_lex_ns.rs` writer; `Q42LexMmap::lookup_parts` reader; Range + JS decoders | **Implemented (0.0.40.1-dev)** | `v4_round_trips_multi_ontology_namespaces`, `v4_pages_decode_from_isolated_byte_ranges` |
| `lookup_hash`/`string_at` verbatim-only | ADR 0015 §6 | namespaced `0x04` entries return `None` from the borrow APIs | **Contract (policy)** | production call sites migrated to `lookup_parts`/`lookup_owned`/`contains` |
| Pre-v4 bundled volumes still LEX v2 | regeneration follow-up | `bundled/ontologies/w3c-archives`, `data/schemaorg/30.0`, `princeton.q42`, `core-ontologies/dist/q42` | **Pending regen** | readable via v3+v2 paths until re-ingested |

## Integrity callouts (first)

1. Prefer `verify-graph --encoder=import` for any volume built with `import`.  
2. Treat empty-lex volumes as hashed-only unless a volume-set root shards lexicon.  
3. Do not equate OEWN 2025-plus TTL with Princeton `princeton.q42`.  
4. HCF Bao wording must not be cited as a conformance claim for `bundle/` until reimplemented.

## Planned extensions (not standards claims yet)

- Payload catalogue writer (ADR 0014).  
- Optional `verify-graph` of embedded `.q42` slices inside `.hmc` (`hmc verify --q42-entry`).
