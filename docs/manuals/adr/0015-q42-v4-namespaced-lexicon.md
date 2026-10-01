# ADR 0015 — Q42 v4: namespaced paged Q42LEX

- **Status:** Accepted (implemented; writer + readers shipped together).
- **Date:** 2026-09-29
- **Branch:** `0.0.40.1-dev`
- **Relates:** Q42 format internal draft; ADR 0014 (payload catalogue);
  `q42-implementation-divergence.md`; `docs/playground/vfs.js` (HTTP-Range VFS).

---

## Context

RDF term strings are dominated by a small set of repeated namespace prefixes
(`https://schema.org/`, `http://www.w3.org/2000/01/rdf-schema#`, `did:q42:`,
`urn:q42:`, …). The Q42LEX paged layout (format version 2) stores every term
verbatim, so a multi-ontology volume re-pays those bytes on every term. A
volume may carry several ontologies, and one ontology may declare several
namespaces — including non-HTTP schemes (DID methods, `urn:`).

The hard constraint is **HTTP-Range (and IPFS-gateway) retrieval**: a reader
must be able to resolve any hash by fetching the lexicon header, the page
directory, and exactly one page. Any namespace dictionary that lives outside
the page breaks that property by adding a second required fetch.

The software is unreleased, so the design is not bent around byte-compatibility
with previous writers; readers still accept v3 volumes and LEX v1/v2 so
existing pre-release data stays loadable until regenerated.

## Decision

1. **Q42 volume version 4** (`Q42_VERSION_V4 = 4`; header layout unchanged
   from v3). All new writes stamp v4; readers accept v3 and v4. There is no
   header-field change — the version marks the writer generation.
2. **Q42LEX format version 4** (there is no LEX v3 — the number aligns with
   the volume generation that first ships it). Pages gain a **page-local
   namespace dictionary** stored at the start of the page blob region:
   ```text
   page header (16 B): [u32 entry_count][u32 ns_count][u64 blob_offset]
   index:              entry_count × [u64 hash][u64 relative-from-blob-start]
   blob:               ns table:  ns_count × [u16 len][utf-8 namespace]
                       entries:   0x01 [u16 len][utf-8]                  (verbatim)
                                  0x04 [u16 ns_id][u16 local_len][utf-8]  (namespaced)
   ```
   `ns_count` occupies the u32 slot v2 writers always zeroed, so v2 pages
   read back with `ns_count == 0` and the reader fails closed on any `0x04`
   entry in a v2 page. Old binaries reject the v4 header word up front.
3. **Namespace extraction is scheme-agnostic**: split at the final `/`, `#`,
   or `:` (ASCII separators — a UTF-8 codepoint is never split), keeping the
   separator in the namespace. This uniformly covers `http(s)` IRIs,
   `did:` methods (`did:q42:`, `did:web:`, …), `urn:`, and future schemes.
   A volume may mix many ontologies and namespaces; each page simply
   carries whichever namespaces its own terms use.
4. **A term is encoded namespaced only when it strictly shrinks the page.**
   For a namespace of length `n` with `m` member terms in the page, the saving
   is `m·(n − 2) − (n + 2)` bytes. The rule is deterministic; small or
   single-member namespaces stay verbatim, so v4 never grows a lexicon.
5. **Pages stay uncompressed** (deliberate design, not a compat hold-over):
   that is what keeps (a) zero-copy mmap borrows of both parts
   (`lookup_parts`) and (b) single-page HTTP-Range decode. Namespace dedup
   *is* the compression mechanism. Whole-page LZ4 remains a future
   candidate (`codec_probe.rs`) at the cost of the zero-copy property.
6. **Reader ABI**: `Q42LexMmap` gains `lookup_parts` / `string_parts_at`
   (zero-copy `(ns, local)` borrows), `lookup_owned`, `resolve_term_into`
   (caller-owned `String` reassembly), and `contains`. `lookup_hash` /
   `string_at` remain for **verbatim entries only** and are documented as
   such; production call sites are migrated to the parts-aware APIs. The
   HTTP-Range resolver (`range_volume.rs::lookup_lexicon_hash_into`) and the
   browser VFS (`docs/playground/vfs.js`) decode v4 pages with the same
   page-local table walk.

## Consequences

- A v4 lexicon for shared-namespace corpora is materially smaller than v2
  (e.g. 512 `https://schema.org/…` terms save ~8 KB on the lexicon section
  alone; see `q42_lex_ns` tests for the measured receipt).
- Range retrieval cost is unchanged: header + directory + one page per
  lookup; the page now includes its namespace table.
- Readers that have not been migrated off `lookup_hash` see namespaced
  entries as absent — every production call site is migrated in the same
  change set, and the divergence log records the verbatim-only contract.
- Bundled/hosted volumes written before v4 (w3c-archives, schema.org 30.0,
  WordNet `princeton.q42`, `core-ontologies/dist/q42`) remain readable (v3
  + LEX v2) until regenerated; regeneration is tracked as follow-up work.

## Non-goals

- Embedded-triple (`0x02`) and Webizen-identity (`0x03`) entries in paged
  volumes — they still flow through the flat v1 writer. Folding them into
  the paged v4 writer is future work.
- Whole-page LZ4 compression of the lexicon (see point 5).
- Media bytes inside `.q42` — that is `.hmc` territory (ADR 0014).
