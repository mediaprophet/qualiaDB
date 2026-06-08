//! Zero-allocation Lexicon Resolver.
//!
//! Maps 64-bit Quin field values back to human-readable `&[u8]` slices for
//! serialisation into N-Triples or JSON-LD surface syntaxes.
//!
//! # Bit layout of a Quin field value
//!
//! ```text
//! Bit 63  │ Bits 60-62   │ Bits 0-59     │ Interpretation
//! ────────┼──────────────┼───────────────┼──────────────────────────────────────
//!   1     │ any          │ payload       │ did:q42 topological pointer (identifier module)
//!   0     │ 0b000        │ FNV-1a hash   │ IRI / blank-node → lexicon lookup
//!   0     │ 0b001        │ integer       │ Inline xsd:integer literal
//!   0     │ 0b010        │ scaled × 10⁶  │ Inline xsd:decimal literal
//!   0     │ 0b011        │ 0 or 1        │ Inline xsd:boolean literal
//!   0     │ 0b100–0b111  │ reserved      │ Treated as IRI hash (future use)
//! ```
//!
//! The inline-type encoding is applied by the ingest layer, which masks
//! FNV-1a hash values to 60 bits before storing them so there is no
//! ambiguity with the type-tag bits.
//!
//! # Zero-allocation guarantee
//! `format_ntriples_to` writes directly to any `impl io::Write` sink and
//! never touches the heap.  Callers own the output buffer.

use crate::QualiaQuin;
use std::io;

// ---------------------------------------------------------------------------
// Bit-layout constants
// ---------------------------------------------------------------------------

const MSB_FLAG: u64 = 1u64 << 63;
const INLINE_TAG_MASK: u64 = 0b111u64 << 60; // bits 60-62 (only when MSB=0)
const INLINE_TAG_INTEGER: u64 = 0b001u64 << 60;
const INLINE_TAG_DECIMAL: u64 = 0b010u64 << 60;
const INLINE_TAG_BOOLEAN: u64 = 0b011u64 << 60;
/// Mask over bits 0-59 — the value payload when an inline tag is present.
const INLINE_VALUE_MASK: u64 = !(MSB_FLAG | INLINE_TAG_MASK);

// ---------------------------------------------------------------------------
// Demo lexicon
// ---------------------------------------------------------------------------
// In production this static table is replaced by a memory-mapped `.q42`
// dictionary shard with entries sorted by hash for O(log n) binary search.
// For the current phase a linear scan over this small table is sufficient.
//
// Entries are (fnv1a_hash, iri_bytes).  Hashes are computed at compile time
// via `q_hash`, which is `const fn`.

static DEMO_LEXICON: &[(u64, &[u8])] = &[
    (crate::q_hash("Alice"), b"http://qualia-db.org/demo/Alice"),
    (crate::q_hash("Bob"), b"http://qualia-db.org/demo/Bob"),
    (crate::q_hash("Carol"), b"http://qualia-db.org/demo/Carol"),
    (crate::q_hash("knows"), b"http://schema.org/knows"),
    (crate::q_hash("likes"), b"http://schema.org/likes"),
    (
        crate::q_hash("type"),
        b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
    ),
    (
        crate::q_hash("label"),
        b"http://www.w3.org/2000/01/rdf-schema#label",
    ),
    (crate::q_hash("Person"), b"http://schema.org/Person"),
    (crate::q_hash("name"), b"http://schema.org/name"),
    (
        crate::q_hash("guardian"),
        b"http://qualia-db.org/vocab#guardian",
    ),
    (crate::q_hash("ward"), b"http://qualia-db.org/vocab#ward"),
    (
        crate::q_hash("has_symptom"),
        b"http://qualia-db.org/medical#hasSymptom",
    ),
    (crate::q_hash("Fever"), b"http://snomed.info/id/386661006"),
    (
        crate::q_hash("income"),
        b"http://qualia-db.org/finance#income",
    ),
    (
        crate::q_hash("balance"),
        b"http://qualia-db.org/finance#balance",
    ),
];

// ---------------------------------------------------------------------------
// Lexicon struct
// ---------------------------------------------------------------------------

/// Wraps the persistent dictionary block used for hash → IRI resolution.
///
/// In production this struct holds a raw pointer into a memory-mapped `.q42`
/// dictionary shard and resolves lookups via a binary search over the
/// sorted `(hash, byte_offset)` index — zero copies, zero allocations.
///
/// For the current phase it wraps the compile-time `DEMO_LEXICON` table.
pub struct Lexicon {
    entries: &'static [(u64, &'static [u8])],
}

impl Lexicon {
    pub const fn new() -> Self {
        Self {
            entries: DEMO_LEXICON,
        }
    }

    /// Look up `hash` in the dictionary.
    ///
    /// Production upgrade: sort `entries` by hash and replace the linear scan
    /// with `entries.binary_search_by_key(&hash, |&(h, _)| h)`.
    #[inline]
    pub fn resolve(&self, hash: u64) -> Option<&'static [u8]> {
        for &(h, bytes) in self.entries {
            if h == hash {
                return Some(bytes);
            }
        }
        None
    }
}

const LEXICON: Lexicon = Lexicon::new();

// ---------------------------------------------------------------------------
// Public resolution API
// ---------------------------------------------------------------------------

/// Resolve a 64-bit Quin field value to its original URI bytes.
///
/// Returns `None` when:
/// - the value has MSB=1 (topological pointer — not a lexicon entry), or
/// - the hash is genuinely absent from the dictionary.
pub fn resolve_hash(hash: u64) -> Option<&'static [u8]> {
    // Topological pointers (MSB=1 and NOT in the lexicon) are not dictionary
    // entries.  Check the lexicon first so that hashes whose FNV-1a value
    // naturally has bit 63 set are still resolved correctly.
    if let Some(uri) = LEXICON.resolve(hash) {
        return Some(uri);
    }
    if (hash & MSB_FLAG) != 0 {
        return None; // confirmed topological pointer
    }
    None
}

// ---------------------------------------------------------------------------
// Term formatters  (all write to impl io::Write — no heap allocation)
// ---------------------------------------------------------------------------

/// Write a subject or predicate term.
/// These positions hold only IRI hashes or did:q42 topological pointers —
/// no inline-typed literals.
///
/// **Lexicon takes priority over bit-flag detection.**
/// A hash stored in the dictionary is always rendered as an IRI, regardless of
/// which bits happen to be set by FNV-1a.  Only values that are absent from the
/// lexicon AND have MSB=1 are interpreted as `did:q42` topological pointers.
/// This correctly handles terms like `q_hash("knows")` whose FNV-1a output
/// naturally has bit 63 set.
#[inline]
fn write_iri_term<W: io::Write>(val: u64, out: &mut W) -> io::Result<()> {
    // 1. Lexicon lookup — exact value, no bit-stripping.
    if let Some(uri) = LEXICON.resolve(val) {
        out.write_all(b"<")?;
        out.write_all(uri)?;
        return out.write_all(b">");
    }
    // 2. Not in lexicon + MSB set → did:q42 topological pointer.
    if (val & MSB_FLAG) != 0 {
        let ptr = val & !MSB_FLAG;
        return write!(out, "<did:q42:ptr/{ptr:016x}>");
    }
    // 3. Unknown hash — hex fallback.
    write!(out, "<quin:hash/{val:016x}>")
}

/// Write an object term, applying inline-type detection on bits 60-62.
///
/// Priority order (same lexicon-first reasoning as `write_iri_term`):
/// 1. Lexicon match → IRI
/// 2. MSB=1 and not in lexicon → did:q42 pointer
/// 3. Bits 60-62 match a known inline tag → typed literal
/// 4. Fallback → hex placeholder
#[inline]
fn write_object_term<W: io::Write>(val: u64, out: &mut W) -> io::Result<()> {
    // 1. Lexicon first — a known IRI hash wins over any bit-pattern check.
    if let Some(uri) = LEXICON.resolve(val) {
        out.write_all(b"<")?;
        out.write_all(uri)?;
        return out.write_all(b">");
    }
    // 2. MSB=1 → topological pointer.
    if (val & MSB_FLAG) != 0 {
        let ptr = val & !MSB_FLAG;
        return write!(out, "<did:q42:ptr/{ptr:016x}>");
    }
    // 3. Inline-type detection (only reached for values NOT in the lexicon).
    //    The ingest layer encodes typed literals with explicit tag bits, so
    //    there is no ambiguity with normalised IRI hashes in a live database.
    match val & INLINE_TAG_MASK {
        INLINE_TAG_INTEGER => {
            let n = val & INLINE_VALUE_MASK;
            write!(out, "\"{n}\"^^<http://www.w3.org/2001/XMLSchema#integer>")
        }
        INLINE_TAG_DECIMAL => {
            // Fixed-point: lower 60 bits encode value × 10⁶.
            let raw = val & INLINE_VALUE_MASK;
            let whole = raw / 1_000_000;
            let frac = raw % 1_000_000;
            write!(
                out,
                "\"{whole}.{frac:06}\"^^<http://www.w3.org/2001/XMLSchema#decimal>"
            )
        }
        INLINE_TAG_BOOLEAN => {
            let lit = if (val & 1) != 0 { "true" } else { "false" };
            write!(out, "\"{lit}\"^^<http://www.w3.org/2001/XMLSchema#boolean>")
        }
        _ => write!(out, "<quin:hash/{val:016x}>"),
    }
}

// ---------------------------------------------------------------------------
// Public streaming formatter
// ---------------------------------------------------------------------------

/// Serialise `quins` as N-Triples, writing each line directly to `out`.
///
/// The function itself performs **no heap allocation** — it writes bytes
/// directly to the caller-supplied `W` sink.  Callers that need an in-memory
/// buffer should pass `&mut Vec<u8>`.
///
/// Subject values are written via `write_iri_term` (which accounts for the
/// MSB / did:q42 flag).  Object values additionally check bits 60-62 for
/// inline-typed literals.
pub fn format_ntriples_to<W: io::Write>(quins: &[QualiaQuin], out: &mut W) -> io::Result<()> {
    for q in quins {
        // Subject: the nested-Quin bit (bit 63) doubles as the did:q42 flag;
        // pass the raw value so `write_iri_term` can render it correctly.
        write_iri_term(q.subject, out)?;
        out.write_all(b" ")?;
        write_iri_term(q.predicate, out)?;
        out.write_all(b" ")?;
        write_object_term(q.object, out)?;
        out.write_all(b" .\n")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn quin(s: u64, p: u64, o: u64) -> QualiaQuin {
        QualiaQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    fn render(quins: &[QualiaQuin]) -> String {
        let mut buf = Vec::new();
        format_ntriples_to(quins, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    // --- resolve_hash -------------------------------------------------------

    #[test]
    fn known_hash_resolves_to_uri() {
        let hash = crate::q_hash("Alice");
        let result = resolve_hash(hash).unwrap();
        assert_eq!(result, b"http://qualia-db.org/demo/Alice");
    }

    #[test]
    fn unknown_hash_returns_none() {
        assert!(resolve_hash(0xDEAD_BEEF_1234_5678).is_none());
    }

    #[test]
    fn topological_pointer_returns_none() {
        let ptr = crate::q_hash("z6Mk") | (1u64 << 63);
        assert!(resolve_hash(ptr).is_none());
    }

    // --- write_iri_term / write_object_term ---------------------------------

    #[test]
    fn known_iri_rendered_with_angle_brackets() {
        let mut buf = Vec::new();
        write_iri_term(crate::q_hash("Alice"), &mut buf).unwrap();
        assert_eq!(buf, b"<http://qualia-db.org/demo/Alice>");
    }

    #[test]
    fn unknown_hash_fallback_is_hex() {
        let mut buf = Vec::new();
        write_iri_term(0x00_00_00_00_00_00_00_2A, &mut buf).unwrap();
        // value 42 decimal = 0x2a hex
        assert_eq!(buf, b"<quin:hash/000000000000002a>");
    }

    #[test]
    fn topological_pointer_renders_as_did_q42_ptr() {
        let val = 42u64 | (1u64 << 63);
        let mut buf = Vec::new();
        write_iri_term(val, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("<did:q42:ptr/"), "got: {s}");
    }

    #[test]
    fn inline_integer_object() {
        let val = INLINE_TAG_INTEGER | 99;
        let mut buf = Vec::new();
        write_object_term(val, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s, "\"99\"^^<http://www.w3.org/2001/XMLSchema#integer>");
    }

    #[test]
    fn inline_boolean_true() {
        let val = INLINE_TAG_BOOLEAN | 1;
        let mut buf = Vec::new();
        write_object_term(val, &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>"
        );
    }

    #[test]
    fn inline_boolean_false() {
        let val = INLINE_TAG_BOOLEAN | 0;
        let mut buf = Vec::new();
        write_object_term(val, &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "\"false\"^^<http://www.w3.org/2001/XMLSchema#boolean>"
        );
    }

    #[test]
    fn inline_decimal_object() {
        // Encode 3.141592 → raw = 3_141_592
        let val = INLINE_TAG_DECIMAL | 3_141_592u64;
        let mut buf = Vec::new();
        write_object_term(val, &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "\"3.141592\"^^<http://www.w3.org/2001/XMLSchema#decimal>"
        );
    }

    // --- format_ntriples_to -------------------------------------------------

    #[test]
    fn empty_slice_writes_nothing() {
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn known_terms_resolve_to_iris() {
        let q = quin(
            crate::q_hash("Alice"),
            crate::q_hash("knows"),
            crate::q_hash("Bob"),
        );
        let out = render(&[q]);
        assert!(
            out.contains("<http://qualia-db.org/demo/Alice>"),
            "got: {out}"
        );
        assert!(out.contains("<http://schema.org/knows>"), "got: {out}");
        assert!(
            out.contains("<http://qualia-db.org/demo/Bob>"),
            "got: {out}"
        );
        assert!(out.ends_with(" .\n"));
    }

    #[test]
    fn unknown_terms_use_hex_fallback() {
        let q = quin(1, 2, 3);
        let out = render(&[q]);
        assert!(out.contains("<quin:hash/0000000000000001>"), "got: {out}");
        assert!(out.contains("<quin:hash/0000000000000002>"), "got: {out}");
        assert!(out.contains("<quin:hash/0000000000000003>"), "got: {out}");
    }

    #[test]
    fn multiple_quins_produce_multiple_lines() {
        let qs = [quin(1, 2, 3), quin(4, 5, 6)];
        let out = render(&qs);
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn subject_msb_renders_as_topological_pointer() {
        // A subject with MSB=1 is a did:q42 coordinate — not a nested-hash lookup.
        let q = quin((1u64 << 63) | 42, 2, 3);
        let out = render(&[q]);
        assert!(out.starts_with("<did:q42:ptr/"), "got: {out}");
    }
}
