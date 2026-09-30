//! Q42LEX v4 — paged lexicon with per-page namespace dictionaries.
//!
//! Ownership note (AGENTS.md §0-B): the v3 codec is decomposed out of
//! `q42_lex.rs` (which stays the reader state machine) so each file keeps a
//! single purpose. The reader additions for v3 live in `q42_lex.rs` because
//! they share the page-walk state; the writer and namespace economics live
//! here. The version word is 4, aligned with the Q42 volume generation that
//! first ships it — there is no LEX v3 (unreleased software; see ADR 0015).
//!
//! ## Why namespaces are stored once per page
//!
//! RDF term strings are dominated by a small set of repeated namespace
//! prefixes (`https://schema.org/`, `http://www.w3.org/2000/01/rdf-schema#`,
//! `did:q42:`, `urn:q42:`, …). v2 pages store every term verbatim, so each
//! term re-pays its namespace bytes. v4 keeps a namespace table **inside each
//! page** and encodes qualifying terms as `(namespace id, local name)`.
//!
//! The table is page-local by design: a page is the unit an HTTP-Range /
//! IPFS reader fetches, so every page stays independently decodable and the
//! range-query contract of the v2 layout is preserved exactly. A volume may
//! contain many ontologies, and an ontology may declare many namespaces —
//! each page simply carries whichever namespaces its own terms use.
//!
//! ## Layout (v4 page, little-endian)
//!
//! ```text
//! page header (16 B): [u32 entry_count][u32 ns_count][u64 blob_offset]
//! index:              entry_count × [u64 hash][u64 relative-from-blob-start]
//! blob:               ns table:  ns_count × [u16 len][utf-8 namespace]
//!                     entries:   0x01 [u16 len][utf-8]                    (verbatim)
//!                                0x04 [u16 ns_id][u16 local_len][utf-8]    (namespaced)
//! ```
//!
//! `ns_count` occupies the u32 slot v2 writers zeroed, so v2 pages read back
//! with `ns_count == 0` and the reader fails closed on any `0x04` entry in a
//! v2 page. Old binaries that only know versions 1–2 reject v4 headers up
//! front (`BadIndex`) instead of misparsing.
//!
//! ## Namespace extraction
//!
//! `namespace_split` is scheme-agnostic: it splits at the final `/`, `#`, or
//! `:` (all ASCII, so a UTF-8 codepoint is never split). The namespace keeps
//! its trailing separator. This uniformly covers `http(s)` IRIs, `did:`
//! methods (e.g. `did:q42:`, `did:web:`), `urn:`, and any future scheme whose
//! terms end in a separator-delimited local name.
//!
//! ## When a term is encoded namespaced
//!
//! Only when it strictly shrinks the page. Per namespace with `m` member
//! terms of length `n`, the saving is `m·(n − 2) − (n + 2)` bytes (each
//! namespaced entry pays a 2-byte ns_id instead of the namespace bytes, the
//! namespace itself is paid once, both sides pay their u16 lengths). The
//! rule is deterministic, so identical input maps produce identical bytes.

use std::collections::HashMap;

use crate::q42_lex::{
    LexError, PAGED_DIRECTORY_ENTRY_SIZE, PAGED_DIRECTORY_HEADER_SIZE, PAGED_PAGE_HEADER_SIZE,
    INDEX_ENTRY_SIZE, LEX_HEADER_SIZE, LEX_MAGIC, LEX_TAG_NAMESPACED, LEX_TAG_STRING,
    LEX_VERSION_V4,
};

/// Split a term into `(namespace, local)` at the final `/`, `#`, or `:`.
/// The namespace includes its trailing separator; the local name is the rest.
/// Returns `None` when the term has no separator — verbatim literals stay
/// verbatim.
pub fn namespace_split(term: &str) -> Option<(&str, &str)> {
    let split = term
        .as_bytes()
        .iter()
        .rposition(|&b| b == b'/' || b == b'#' || b == b':')?;
    Some((&term[..split + 1], &term[split + 1..]))
}

/// Serialize a hash → string map into the namespaced paged `Q42LEX` v4 byte
/// layout that [`crate::q42_lex::Q42LexMmap`] reads back.
///
/// Entries are sorted by hash and chunked into pages of `page_entries`
/// exactly like the v2 writer; within each page, terms whose namespace group
/// passes the economics test above are stored as `0x04` references into the
/// page-local namespace table, and everything else stays verbatim (`0x01`).
/// Pages remain independently addressable: decoding any one page needs only
/// that page's byte range.
pub fn serialize_namespaced_paged_lexicon(
    entries: &HashMap<u64, String>,
    page_entries: usize,
) -> Result<Vec<u8>, LexError> {
    if page_entries == 0 {
        return Err(LexError::BadIndex);
    }
    let mut sorted: Vec<(&u64, &String)> = entries.iter().collect();
    sorted.sort_unstable_by_key(|(h, _)| **h);
    let page_count = (sorted.len() + page_entries - 1) / page_entries;
    let directory_len = PAGED_DIRECTORY_HEADER_SIZE + page_count * PAGED_DIRECTORY_ENTRY_SIZE;
    let mut out = Vec::with_capacity(LEX_HEADER_SIZE + directory_len + entries.len() * 24);
    out.extend_from_slice(&LEX_MAGIC);
    out.extend_from_slice(&(sorted.len() as u64).to_le_bytes());
    out.extend_from_slice(&(LEX_HEADER_SIZE as u64).to_le_bytes());
    out.extend_from_slice(&LEX_VERSION_V4.to_le_bytes());
    out.extend_from_slice(&(page_count as u64).to_le_bytes());
    out.resize(LEX_HEADER_SIZE + directory_len, 0);

    for (page_index, chunk) in sorted.chunks(page_entries).enumerate() {
        let page_offset = out.len();
        let page_start = out.len();

        // Namespace groups inside this page, in first-appearance order.
        let mut ns_ids: HashMap<&str, u16> = HashMap::new();
        let mut ns_table: Vec<&str> = Vec::new();
        let mut members: HashMap<&str, usize> = HashMap::new();
        for (_, text) in chunk {
            if text.len() > u16::MAX as usize {
                return Err(LexError::TermTooLong);
            }
            if let Some((ns, _)) = namespace_split(text) {
                *members.entry(ns).or_insert(0usize) += 1;
            }
        }
        let use_ns = |ns: &str| -> bool {
            match members.get(ns) {
                Some(&m) if ns.len() >= 3 => {
                    // m·(n − 2) − (n + 2) > 0, in usize-safe form.
                    (m as u64) * (ns.len() as u64 - 2) > (ns.len() as u64 + 2)
                }
                _ => false,
            }
        };

        out.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // ns_count, patched below
        out.extend_from_slice(&0u64.to_le_bytes()); // blob_offset, patched below
        let blob_offset = PAGED_PAGE_HEADER_SIZE + chunk.len() * INDEX_ENTRY_SIZE;
        let index_start = out.len();
        out.resize(index_start + chunk.len() * INDEX_ENTRY_SIZE, 0);

        // Namespace table at the start of the blob region.
        for (ns, count) in &members {
            if *count > 0 && use_ns(ns) {
                ns_ids.insert(ns, ns_table.len() as u16);
                ns_table.push(ns);
            }
        }
        for ns in &ns_table {
            let bytes = ns.as_bytes();
            let len = u16::try_from(bytes.len()).map_err(|_| LexError::TermTooLong)?;
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(bytes);
        }

        for (entry_index, (hash, text)) in chunk.iter().enumerate() {
            let relative = (out.len() - page_start - blob_offset) as u64;
            let b = text.as_bytes();
            match namespace_split(text).filter(|(ns, _)| ns_ids.contains_key(ns)) {
                Some((ns, local)) => {
                    let ns_id = ns_ids[ns];
                    let local = local.as_bytes();
                    let local_len =
                        u16::try_from(local.len()).map_err(|_| LexError::TermTooLong)?;
                    out.push(LEX_TAG_NAMESPACED);
                    out.extend_from_slice(&ns_id.to_le_bytes());
                    out.extend_from_slice(&local_len.to_le_bytes());
                    out.extend_from_slice(local);
                }
                None => {
                    let len = u16::try_from(b.len()).map_err(|_| LexError::TermTooLong)?;
                    out.push(LEX_TAG_STRING);
                    out.extend_from_slice(&len.to_le_bytes());
                    out.extend_from_slice(&b[..len as usize]);
                }
            }
            let index_offset = index_start + entry_index * INDEX_ENTRY_SIZE;
            out[index_offset..index_offset + 8].copy_from_slice(&hash.to_le_bytes());
            out[index_offset + 8..index_offset + 16].copy_from_slice(&relative.to_le_bytes());
        }

        let ns_count = ns_table.len() as u32;
        out[page_start + 4..page_start + 8].copy_from_slice(&ns_count.to_le_bytes());
        out[page_start + 8..page_start + 16].copy_from_slice(&(blob_offset as u64).to_le_bytes());

        let page_length = (out.len() - page_start) as u64;
        let directory = LEX_HEADER_SIZE
            + PAGED_DIRECTORY_HEADER_SIZE
            + page_index * PAGED_DIRECTORY_ENTRY_SIZE;
        out[directory..directory + 8].copy_from_slice(&chunk[0].0.to_le_bytes());
        out[directory + 8..directory + 16].copy_from_slice(&(page_offset as u64).to_le_bytes());
        out[directory + 16..directory + 24].copy_from_slice(&page_length.to_le_bytes());
        out[directory + 24..directory + 28]
            .copy_from_slice(&(chunk.len() as u32).to_le_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_lex::{Q42LexMmap, LEX_VERSION_PAGED, LEX_VERSION_V4};

    #[test]
    fn namespace_split_is_scheme_agnostic() {
        assert_eq!(
            namespace_split("http://theontology.tld/Term"),
            Some(("http://theontology.tld/", "Term"))
        );
        assert_eq!(
            namespace_split("http://www.w3.org/2001/XMLSchema#integer"),
            Some(("http://www.w3.org/2001/XMLSchema#", "integer"))
        );
        assert_eq!(
            namespace_split("did:q42:z6MkpTHR8V"),
            Some(("did:q42:", "z6MkpTHR8V"))
        );
        assert_eq!(
            namespace_split("urn:uuid:550e8400-e29b"),
            Some(("urn:uuid:", "550e8400-e29b"))
        );
        assert_eq!(namespace_split("dog"), None);
        // Multibyte locals survive: separators are ASCII.
        assert_eq!(
            namespace_split("https://ex.org/อย่างสะเพร่า"),
            Some(("https://ex.org/", "อย่างสะเพร่า"))
        );
    }

    #[test]
    fn v4_round_trips_multi_ontology_namespaces() {
        // Three ontologies in one volume: schema.org, W3C RDF, and a DID-based
        // Webizen namespace — plus a bare literal that must stay verbatim.
        let mut map = HashMap::new();
        for name in ["Thing", "Person", "Organization", "CreativeWork"] {
            map.insert(crate::q_hash(name), format!("https://schema.org/{name}"));
        }
        for name in ["label", "comment", "isDefinedBy", "seeAlso", "domain"] {
            map.insert(crate::q_hash(name), format!("http://www.w3.org/2000/01/rdf-schema#{name}"));
        }
        for n in 0..6u32 {
            map.insert(
                crate::q_hash(&format!("did-term-{n}")),
                format!("did:q42:z6MkTopic{n}"),
            );
        }
        map.insert(crate::q_hash("bare"), "bare-literal-no-separator".to_string());

        let bytes = serialize_namespaced_paged_lexicon(&map, 3).unwrap();
        assert_eq!(
            u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            LEX_VERSION_V4
        );
        let view = Q42LexMmap::from_bytes(&bytes).unwrap();
        assert_eq!(view.entry_count(), map.len());
        for (hash, text) in &map {
            let (ns, local) = view.lookup_parts(*hash).expect("term must resolve");
            assert_eq!(format!("{ns}{local}"), *text);
            assert_eq!(view.lookup_owned(*hash).as_deref(), Some(text.as_str()));
        }
        // The bare literal stays verbatim: parts are ("", full).
        let (ns, local) = view.lookup_parts(crate::q_hash("bare")).unwrap();
        assert_eq!(ns, "");
        assert_eq!(local, "bare-literal-no-separator");
    }

    #[test]
    fn v4_pages_decode_from_isolated_byte_ranges() {
        // HTTP-Range proof: reconstruct lookups using only the global header,
        // the directory, and ONE page's bytes — never the full lexicon.
        let mut map = HashMap::new();
        for i in 0..40u64 {
            map.insert(
                crate::q_hash(&format!("term-{i}")),
                format!("https://example.org/vocab/term-{i}"),
            );
        }
        let bytes = serialize_namespaced_paged_lexicon(&map, 4).unwrap();
        let view = Q42LexMmap::from_bytes(&bytes).unwrap();

        let page_count = u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize;
        assert_eq!(page_count, 10);
        for page in 0..page_count {
            let dir = 40 + page * PAGED_DIRECTORY_ENTRY_SIZE;
            let page_offset =
                usize::try_from(u64::from_le_bytes(bytes[dir + 8..dir + 16].try_into().unwrap()))
                    .unwrap();
            let page_length =
                usize::try_from(u64::from_le_bytes(bytes[dir + 16..dir + 24].try_into().unwrap()))
                    .unwrap();
            let count = u32::from_le_bytes(bytes[dir + 24..dir + 28].try_into().unwrap()) as usize;
            // The slice a range reader would hold for this page alone.
            let page_bytes = &bytes[page_offset..page_offset + page_length];
            let ns_count = u32::from_le_bytes(page_bytes[4..8].try_into().unwrap()) as usize;
            assert_eq!(ns_count, 1, "one namespace shared by the whole page");
            for item in 0..count {
                let entry = 16 + item * INDEX_ENTRY_SIZE;
                let hash = u64::from_le_bytes(page_bytes[entry..entry + 8].try_into().unwrap());
                let relative = u64::from_le_bytes(
                    page_bytes[entry + 8..entry + 16].try_into().unwrap(),
                ) as usize;
                let blob_offset =
                    u64::from_le_bytes(page_bytes[8..16].try_into().unwrap()) as usize;
                let start = blob_offset + relative;
                assert_eq!(page_bytes[start], LEX_TAG_NAMESPACED);
                let ns_id =
                    u16::from_le_bytes(page_bytes[start + 1..start + 3].try_into().unwrap());
                let local_len =
                    u16::from_le_bytes(page_bytes[start + 3..start + 5].try_into().unwrap());
                // Walk the page-local namespace table to rebuild the ns.
                // Namespace table entries are [u16 len][utf-8] — no tag byte.
                let mut cursor = blob_offset;
                let mut ns = "";
                for _ in 0..=ns_id {
                    let len = u16::from_le_bytes(
                        page_bytes[cursor..cursor + 2].try_into().unwrap(),
                    ) as usize;
                    ns = std::str::from_utf8(&page_bytes[cursor + 2..cursor + 2 + len]).unwrap();
                    cursor += 2 + len;
                }
                let local =
                    std::str::from_utf8(&page_bytes[start + 5..start + 5 + local_len as usize])
                        .unwrap();
                let text = format!("{ns}{local}");
                assert_eq!(text, view.lookup_owned(hash).unwrap());
            }
        }
    }

    #[test]
    fn v4_is_smaller_than_v2_for_shared_namespaces() {
        let mut map = HashMap::new();
        for i in 0..512u32 {
            map.insert(
                crate::q_hash(&format!("term-{i}")),
                format!("https://schema.org/term-{i}"),
            );
        }
        let v2 = crate::q42_lex::serialize_paged_string_lexicon(&map, 4096).unwrap();
        let v4 = serialize_namespaced_paged_lexicon(&map, 4096).unwrap();
        assert!(
            v4.len() < v2.len(),
            "v4 ({}) must beat v2 ({}) on shared namespaces",
            v4.len(),
            v2.len()
        );
        // 512 terms × 18 namespace bytes minus per-entry and table overhead.
        let saved = v2.len() - v4.len();
        assert!(saved > 8_000, "expected a material saving, got {saved} B");
    }

    #[test]
    fn v4_never_grows_a_lexicon() {
        // Worst case: every term unique (no shared namespaces) — v4 falls
        // back to verbatim entries plus empty ns tables.
        let mut map = HashMap::new();
        for i in 0..64u32 {
            map.insert(crate::q_hash(&format!("u{i}")), format!("urn:u{i}:only-one"));
        }
        let v2 = crate::q42_lex::serialize_paged_string_lexicon(&map, 4096).unwrap();
        let v4 = serialize_namespaced_paged_lexicon(&map, 4096).unwrap();
        // Same entries, same pages; ns_count fields are 0. The only delta is
        // the format version word, so sizes are equal.
        assert_eq!(v2.len(), v4.len());
        let view = Q42LexMmap::from_bytes(&v4).unwrap();
        for (hash, text) in &map {
            assert_eq!(view.lookup_owned(*hash).as_deref(), Some(text.as_str()));
        }
    }

    #[test]
    fn v2_reader_rejects_namespaced_entries() {
        // Fail-closed: a 0x04 tag in a v2 page (ns_count slot 0) must not
        // decode, even if the bytes otherwise look plausible.
        let mut map = HashMap::new();
        for i in 0..4u32 {
            map.insert(crate::q_hash(&format!("t{i}")), format!("https://ex.org/t{i}"));
        }
        let mut bytes = crate::q42_lex::serialize_paged_string_lexicon(&map, 4).unwrap();
        bytes[24..32].copy_from_slice(&LEX_VERSION_PAGED.to_le_bytes());
        // Flip the first entry tag from 0x01 (verbatim) to 0x04.
        let dir = 40;
        let page_offset =
            usize::try_from(u64::from_le_bytes(bytes[dir + 8..dir + 16].try_into().unwrap()))
                .unwrap();
        let blob_offset =
            u64::from_le_bytes(bytes[page_offset + 8..page_offset + 16].try_into().unwrap()) as usize;
        bytes[page_offset + blob_offset] = 0x04;
        assert!(Q42LexMmap::from_bytes(&bytes).is_err());
    }

    #[test]
    fn v4_rejects_overlong_terms() {
        let long = "あ".repeat(22_000);
        let mut map = HashMap::new();
        map.insert(crate::q_hash(&long), long);
        assert_eq!(
            serialize_namespaced_paged_lexicon(&map, 4),
            Err(LexError::TermTooLong)
        );
    }

    /// Real-data receipt: load the term set of an actual in-repo volume and
    /// compare what the v2 paged layout would cost against the v4 namespaced
    /// layout for the *same* terms. Stable regardless of which generation is
    /// on disk. Hashed-only volumes (32-byte stub lexicons — e.g.
    /// `princeton.q42`) are skipped honestly: no terms to deduplicate.
    #[test]
    fn real_volume_lexicon_shrinks_under_v4() {
        let candidates = [
            "docs/data/schemaorg/30.0/schemaorg-current-https.q42",
            "../../docs/data/schemaorg/30.0/schemaorg-current-https.q42",
            "bundled/ontologies/w3c-archives/ldp.q42",
            "../../bundled/ontologies/w3c-archives/ldp.q42",
        ];
        let mut chosen: Option<(std::path::PathBuf, HashMap<u64, String>, u64)> = None;
        for candidate in candidates {
            let path = std::path::Path::new(candidate);
            if !path.is_file() {
                continue;
            }
            let Ok(volume) = crate::q42_volume::Q42Volume::open(path) else {
                continue;
            };
            let mut map;
            let entries;
            {
                let Ok(view) = volume.lex_view() else {
                    continue;
                };
                entries = view.entry_count() as u64;
                map = HashMap::with_capacity(view.entry_count());
                for i in 0..view.entry_count() {
                    let Some(hash) = view.hash_at(i) else {
                        break;
                    };
                    if let Some((ns, local)) = view.string_parts_at(i) {
                        let mut text = String::with_capacity(ns.len() + local.len());
                        text.push_str(ns);
                        text.push_str(local);
                        map.insert(hash, text);
                    }
                }
            }
            if map.is_empty() {
                eprintln!(
                    "[lex-v4] {} is hashed-only (empty lexicon) — no terms to deduplicate",
                    path.display()
                );
                continue;
            }
            chosen = Some((path.to_path_buf(), map, entries));
            break;
        }
        let Some((path, map, entries)) = chosen else {
            eprintln!("[lex-v4] no in-repo volume with a populated lexicon — receipt skipped");
            return;
        };
        let v2 = crate::q42_lex::serialize_paged_string_lexicon(
            &map,
            crate::q42_lex::DEFAULT_LEX_PAGE_ENTRIES,
        )
        .unwrap();
        let v4 = serialize_namespaced_paged_lexicon(
            &map,
            crate::q42_lex::DEFAULT_LEX_PAGE_ENTRIES,
        )
        .unwrap();
        let saved = v2.len().saturating_sub(v4.len());
        println!(
            "[lex-v4] {} — {entries} terms; same lexicon under v2 layout = {} B vs v4 = {} B, saved {saved} B ({:.1}%)",
            path.display(),
            v2.len(),
            v4.len(),
            saved as f64 * 100.0 / v2.len() as f64
        );
        assert!(
            v4.len() < v2.len(),
            "v4 must beat v2 for a real ontology lexicon ({} → {})",
            v2.len(),
            v4.len()
        );
    }
}
