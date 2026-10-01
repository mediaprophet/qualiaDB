//! JSON-LD 1.1 ↔ vendor nquin-CBOR transform honesty tests (UE-015).
//!
//! Qualia's compact CBOR quin encoding is **`application/vnd.qualia.nquin-cbor`**
//! until a complete bidirectional transform to the admitted JSON-LD graph is
//! proven. These tests document what round-trips today and what fails closed.

use crate::sparql_library::rdf_formats::{
    parse_rdf, serialize_rdf, QuinCollector, RdfFormat, RdfStarMode,
};
use crate::NQuin;
use std::io::Cursor;

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat JSON-LD (no remote @context) parses, and serialize→reparse keeps a
    /// non-empty graph. Exact SPO hash identity is **not** claimed here: the
    /// JSON-LD writer may emit `quin:hash/…` surfaces that re-hash differently.
    #[test]
    fn jsonld_flat_parse_serialize_reparse_nonempty() {
        let src = r#"{
          "@id": "ex:alice",
          "ex:knows": { "@id": "ex:bob" },
          "ex:name": "Alice"
        }"#;
        let mut a = QuinCollector::new();
        let n = parse_rdf(RdfFormat::JsonLd, Cursor::new(src.as_bytes()), 0, &mut a)
            .expect("parse jsonld");
        assert!(n >= 1, "expected at least one quin");
        let first: Vec<NQuin> = a.as_slice().to_vec();

        let mut buf = Vec::new();
        serialize_rdf(RdfFormat::JsonLd, RdfStarMode::Plain, &first, &mut buf)
            .expect("serialize jsonld");
        assert!(!buf.is_empty());
        assert!(
            buf[0] == b'{' || buf[0] == b'[',
            "JSON-LD must remain text JSON, got first byte {}",
            buf[0]
        );

        let mut b = QuinCollector::new();
        let n2 = parse_rdf(RdfFormat::JsonLd, Cursor::new(&buf[..]), 0, &mut b)
            .expect("reparse jsonld");
        assert!(n2 >= 1, "reparsed graph must be non-empty");
    }

    /// Lossless vendor packing: raw 48-byte NQuin frames (not CBOR-LD WD).
    #[test]
    fn vendor_packed_48byte_quin_round_trip_bit_exact() {
        use crate::lexicon::generate_60bit_token;
        let q = NQuin {
            subject: generate_60bit_token(b"ex:alice"),
            predicate: generate_60bit_token(b"ex:knows"),
            object: generate_60bit_token(b"ex:bob"),
            context: 7,
            metadata: 9,
            parity: 0,
        };
        let bytes: &[u8] = bytemuck::bytes_of(&q);
        assert_eq!(bytes.len(), 48);
        let back: &NQuin = bytemuck::from_bytes(bytes);
        assert_eq!(back.subject, q.subject);
        assert_eq!(back.predicate, q.predicate);
        assert_eq!(back.object, q.object);
        assert_eq!(back.context, q.context);
        assert_eq!(back.metadata, q.metadata);
    }

    /// CBOR array-of-maps encode is decodable; string re-hash may remint IRIs.
    /// Profile remains vendor — not W3C CBOR-LD.
    #[test]
    fn vendor_cbor_array_decodes_one_triple() {
        use crate::lexicon::generate_60bit_token;
        use crate::sparql_library::parsers::cbor_parser::parse_cbor_ld_into;
        use crate::sparql_library::serialisers::rdf_serializers::serialize_to_cborld;

        let s = generate_60bit_token(b"ex:alice");
        let p = generate_60bit_token(b"ex:knows");
        let o = generate_60bit_token(b"ex:bob");
        let quins = [NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: s ^ p ^ o,
        }];

        let mut buf = Vec::new();
        serialize_to_cborld(&mut buf, &quins).expect("encode vendor cbor");
        assert!(
            (0x80..=0x97).contains(&buf[0]) || buf[0] == 0x9f,
            "vendor profile must be a CBOR array, got {:#x}",
            buf[0]
        );

        let mut out = QuinCollector::new();
        let n = parse_cbor_ld_into(Cursor::new(&buf[..]), 0, &mut out).expect("decode");
        assert_eq!(n, 1, "one map → one quin");
        assert_ne!(out.as_slice()[0].subject, 0);
    }

    /// Inline-typed integers MUST round-trip losslessly through vendor CBOR
    /// (`@value`/`@type` → `INLINE_TAG_INTEGER`).
    #[test]
    fn vendor_nquin_cbor_lossless_on_inline_integer() {
        use crate::frame_layout::INLINE_TAG_INTEGER;
        use crate::lexicon::generate_60bit_token;
        use crate::sparql_library::parsers::cbor_parser::parse_cbor_ld_into;
        use crate::sparql_library::serialisers::rdf_serializers::serialize_to_cborld;
        use crate::resolver::classify_inline_literal;

        let s = generate_60bit_token(b"ex:alice");
        let p = generate_60bit_token(b"ex:age");
        let o = INLINE_TAG_INTEGER | 42;
        let quins = [NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }];
        let mut buf = Vec::new();
        serialize_to_cborld(&mut buf, &quins).expect("encode");
        let mut out = QuinCollector::new();
        let n = parse_cbor_ld_into(Cursor::new(&buf[..]), 0, &mut out).expect("decode");
        assert_eq!(n, 1);
        assert_eq!(
            out.as_slice()[0].object, o,
            "inline integer must survive vendor CBOR round-trip bit-exactly"
        );
        assert!(matches!(
            classify_inline_literal(out.as_slice()[0].object),
            Some(crate::resolver::InlineLiteral::Integer(42))
        ));
    }

    /// JSON text must not be admitted by the binary gatekeeper.
    #[test]
    fn json_text_rejected_by_vendor_cbor_gate() {
        use crate::query::cbor_compiler::ingest_network_payload;
        assert!(ingest_network_payload(b"{\"@id\":\"x\"}").is_err());
    }
}
