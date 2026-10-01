//! RDF Format Serializers for QualiaDB
//!
//! Serializes NQuin data to standard RDF formats: N-Triples, Turtle, N-Quads,
//! TriG, N3, JSON-LD.
//!
//! Terms are resolved through the shared resolver primitives
//! (`write_iri_term` / `write_object_term`), so subjects/predicates render as
//! `<iri>` and objects render as `<iri>` **or** a typed literal
//! (`"42"^^<…#integer>`) exactly as in the N-Triples path. The grouped
//! formats (Turtle/TriG/N3) additionally produce *valid* surface syntax —
//! subject joined to its predicate–object list with `;` separators and a single
//! trailing `.`, not the previous malformed "subject `.`" per line.
//!
//! Known boundary (shared by every path here, including N-Triples): a plain or
//! language-tagged **string** literal that was interned into the lexicon at
//! ingest is indistinguishable from an IRI at this layer and renders as `<…>`.
//! Inline-typed literals (integer/decimal/boolean/float) *are* distinguished and
//! render correctly. Resolving interned string literals to `"…"@lang` output is
//! an ingest-layer concern (the term must carry its literal flag) tracked
//! separately; it is not silently faked here.

use std::collections::HashMap;
use std::io::Write;

use crate::query::resolver::{classify_inline_literal, write_iri_term, write_object_term};
use crate::NQuin;

/// Serialize Quins to N-Triples format (zero-heap via resolver).
pub fn serialize_to_ntriples<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    crate::resolver::format_ntriples_to(quins, writer)
        .map_err(|e| format!("Failed to write N-Triples: {e}"))
}

/// Group quins by a key field, preserving first-seen key order for stable,
/// deterministic output (HashMap iteration order is not stable).
fn group_by<'a>(quins: &'a [NQuin], key: impl Fn(&NQuin) -> u64) -> Vec<(u64, Vec<&'a NQuin>)> {
    let mut order: Vec<u64> = Vec::new();
    let mut map: HashMap<u64, Vec<&NQuin>> = HashMap::new();
    for quin in quins {
        let k = key(quin);
        let bucket = map.entry(k).or_default();
        if bucket.is_empty() {
            order.push(k);
        }
        bucket.push(quin);
    }
    order
        .into_iter()
        .map(|k| {
            let v = map.remove(&k).unwrap_or_default();
            (k, v)
        })
        .collect()
}

/// Write a subject and its predicate–object list as one valid Turtle/TriG
/// statement: `<s> <p1> <o1> ;` … `<pn> <on> .`, with `indent` spaces before
/// each continuation predicate.
fn write_subject_block<W: Write>(
    writer: &mut W,
    subject: u64,
    rows: &[&NQuin],
    indent: &str,
) -> std::io::Result<()> {
    write!(writer, "{indent}")?;
    write_iri_term(subject, writer)?;
    for (i, quin) in rows.iter().enumerate() {
        if i == 0 {
            write!(writer, " ")?;
        } else {
            write!(writer, " ;\n{indent}    ")?;
        }
        write_iri_term(quin.predicate, writer)?;
        write!(writer, " ")?;
        write_object_term(quin.object, writer)?;
    }
    writeln!(writer, " .")
}

/// Serialize Quins to Turtle format.
pub fn serialize_to_turtle<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    for (subject, rows) in group_by(quins, |q| q.subject) {
        write_subject_block(writer, subject, &rows, "")
            .map_err(|e| format!("Failed to write Turtle: {e}"))?;
    }
    Ok(())
}

/// Serialize Quins to N-Quads format (zero-heap via resolver).
pub fn serialize_to_nquads<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    crate::resolver::format_nquads_to(quins, writer)
        .map_err(|e| format!("Failed to write N-Quads: {e}"))
}

/// Serialize Quins to TriG format (named graphs of Turtle blocks).
pub fn serialize_to_trig<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    for (context, ctx_quins) in group_by(quins, |q| q.context) {
        write!(writer, "").map_err(|e| format!("Failed to write TriG: {e}"))?;
        write_iri_term(context, writer).map_err(|e| format!("Failed to write TriG graph: {e}"))?;
        writeln!(writer, " {{").map_err(|e| format!("Failed to write TriG graph: {e}"))?;

        // Re-group this graph's quins by subject.
        let owned: Vec<NQuin> = ctx_quins.iter().map(|q| **q).collect();
        for (subject, rows) in group_by(&owned, |q| q.subject) {
            write_subject_block(writer, subject, &rows, "    ")
                .map_err(|e| format!("Failed to write TriG statement: {e}"))?;
        }

        writeln!(writer, "}}").map_err(|e| format!("Failed to write TriG graph end: {e}"))?;
    }
    Ok(())
}

/// Serialize Quins to N3 format.
///
/// Emits a Solid-friendly `text/n3` document: common `@prefix` declarations
/// plus Turtle-compatible statements (valid N3). Full N3 formulae (`{…}`) are
/// not synthesised from flat Quins — those come from the N3 rule parser path.
pub fn serialize_to_n3<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    let err = |e: std::io::Error| format!("Failed to write N3: {e}");
    writeln!(writer, "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .").map_err(err)?;
    writeln!(writer, "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .").map_err(err)?;
    writeln!(writer, "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .").map_err(err)?;
    writeln!(writer, "@prefix owl: <http://www.w3.org/2002/07/owl#> .").map_err(err)?;
    writeln!(writer).map_err(err)?;
    for (subject, rows) in group_by(quins, |q| q.subject) {
        write_subject_block(writer, subject, &rows, "")
            .map_err(|e| format!("Failed to write N3: {e}"))?;
    }
    Ok(())
}

/// A resolved JSON-LD node: an IRI reference or a typed literal value.
enum JsonLdTerm {
    Iri(String),
    Literal { value: String, datatype: String },
}

/// Resolve a term hash for JSON-LD (bare IRI string, no angle brackets; or a
/// typed literal). Mirrors the resolver's lexicon-first priority.
fn jsonld_term(hash: u64) -> JsonLdTerm {
    if let Some(bytes) = crate::resolver::resolve_hash(hash) {
        return JsonLdTerm::Iri(String::from_utf8_lossy(bytes).into_owned());
    }
    if (hash & crate::resolver::MSB_FLAG) != 0 {
        return JsonLdTerm::Iri(format!(
            "did:q42:ptr/{:016x}",
            hash & !crate::resolver::MSB_FLAG
        ));
    }
    if let Some(lit) = classify_inline_literal(hash) {
        return JsonLdTerm::Literal {
            value: lit.to_string(),
            datatype: lit.datatype_iri().to_string(),
        };
    }
    JsonLdTerm::Iri(format!("quin:hash/{hash:016x}"))
}

/// Minimal JSON string escaping (quote, backslash, control chars).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Serialize Quins to JSON-LD (expanded node objects, grouped by subject).
///
/// Each subject becomes one node object; predicates map to arrays of value
/// objects (`{"@id": …}` for IRIs, `{"@value": …, "@type": …}` for literals).
pub fn serialize_to_jsonld<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    let err = |e: std::io::Error| format!("Failed to write JSON-LD: {e}");
    writeln!(writer, "[").map_err(err)?;

    let subjects = group_by(quins, |q| q.subject);
    for (si, (subject, rows)) in subjects.iter().enumerate() {
        if si > 0 {
            writeln!(writer, ",").map_err(err)?;
        }
        let subj_iri = match jsonld_term(*subject) {
            JsonLdTerm::Iri(s) => s,
            // A subject can only be an IRI/blank node; a literal here is a
            // malformed inline value — surface its lexical form as the @id
            // rather than dropping the statement.
            JsonLdTerm::Literal { value, .. } => value,
        };
        writeln!(writer, "  {{").map_err(err)?;
        write!(writer, "    \"@id\": \"{}\"", json_escape(&subj_iri)).map_err(err)?;

        // Group this subject's rows by predicate to build one array per predicate.
        let owned: Vec<NQuin> = rows.iter().map(|q| **q).collect();
        let by_pred = group_by(&owned, |q| q.predicate);
        for (predicate, pred_rows) in &by_pred {
            let pred_iri = match jsonld_term(*predicate) {
                JsonLdTerm::Iri(s) => s,
                JsonLdTerm::Literal { value, .. } => value,
            };
            writeln!(writer, ",").map_err(err)?;
            writeln!(writer, "    \"{}\": [", json_escape(&pred_iri)).map_err(err)?;
            for (oi, quin) in pred_rows.iter().enumerate() {
                if oi > 0 {
                    writeln!(writer, ",").map_err(err)?;
                }
                match jsonld_term(quin.object) {
                    JsonLdTerm::Iri(iri) => {
                        write!(writer, "      {{ \"@id\": \"{}\" }}", json_escape(&iri))
                            .map_err(err)?;
                    }
                    JsonLdTerm::Literal { value, datatype } => {
                        write!(
                            writer,
                            "      {{ \"@value\": \"{}\", \"@type\": \"{}\" }}",
                            json_escape(&value),
                            json_escape(&datatype)
                        )
                        .map_err(err)?;
                    }
                }
            }
            write!(writer, "\n    ]").map_err(err)?;
        }
        write!(writer, "\n  }}").map_err(err)?;
    }

    writeln!(writer, "\n]").map_err(err)?;
    Ok(())
}

/// Compact JSON-LD 1.1 document for Solid / LDP: pinned Qualia `@context` + `@graph`.
///
/// The context bytes are exactly [`crate::sparql_library::rdf_formats::QUALIA_JSONLD_CONTEXT_V1`]
/// so package receipts can pin `context_digest_sha256`. Nodes in `@graph` use the same
/// expanded term shape as [`serialize_to_jsonld`] (full IRIs) — compaction of arbitrary
/// vocabularies is not claimed; embedding the pinned context is what Solid clients need
/// for offline resolution without remote `@context` fetches.
pub fn serialize_to_jsonld_compact<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    use crate::sparql_library::rdf_formats::QUALIA_JSONLD_CONTEXT_V1;
    let err = |e: std::io::Error| format!("Failed to write compact JSON-LD: {e}");

    // QUALIA_JSONLD_CONTEXT_V1 is `{ "@context": {…}, "qualia:contextVersion": "1" }`.
    // Prefer embedding the inner `@context` object when present.
    let ctx_fragment = if let Some(start) = QUALIA_JSONLD_CONTEXT_V1.find("\"@context\"") {
        let after = &QUALIA_JSONLD_CONTEXT_V1[start..];
        if let Some(brace) = after.find('{') {
            let rest = &after[brace..];
            let mut depth = 0i32;
            let mut end = None;
            for (i, c) in rest.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(i + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            end.map(|e| rest[..e].to_string())
        } else {
            None
        }
    } else {
        None
    };
    let ctx_json = ctx_fragment.unwrap_or_else(|| QUALIA_JSONLD_CONTEXT_V1.to_string());

    write!(writer, "{{\n  \"@context\": ").map_err(err)?;
    write!(writer, "{ctx_json}").map_err(err)?;
    write!(writer, ",\n  \"@graph\": ").map_err(err)?;
    serialize_to_jsonld(writer, quins)?;
    // serialize_to_jsonld ends with `]\n`; close the outer object.
    write!(writer, "}}\n").map_err(err)?;
    Ok(())
}

/// Serialize Quins to the Qualia vendor CBOR profile (`application/vnd.qualia.nquin-cbor`):
/// a CBOR array of JSON-LD-shaped node maps, one map per triple.
///
/// Object terms use JSON-LD value objects so **inline-typed literals round-trip
/// losslessly** (`{"@value","@type"}` → `INLINE_TAG_*`). IRI objects use
/// `{"@id": …}`. Bare string values remain accepted by the parser for legacy
/// fixtures.
///
/// This is **not** W3C CBOR-LD 1.0 (Working Draft). Lexicon-resolved IRIs keep
/// their hash; unknown hashes serialize as `quin:hash/…` surfaces.
pub fn serialize_to_cborld<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    use ciborium::value::Value;

    let cbor_object_term = |h: u64| -> Value {
        match jsonld_term(h) {
            JsonLdTerm::Iri(s) => Value::Map(vec![(
                Value::Text("@id".to_string()),
                Value::Text(s),
            )]),
            JsonLdTerm::Literal { value, datatype } => Value::Map(vec![
                (Value::Text("@value".to_string()), Value::Text(value)),
                (Value::Text("@type".to_string()), Value::Text(datatype)),
            ]),
        }
    };

    let mut arr: Vec<Value> = Vec::with_capacity(quins.len());
    for q in quins {
        let subj = match jsonld_term(q.subject) {
            JsonLdTerm::Iri(s) => s,
            JsonLdTerm::Literal { value, .. } => value,
        };
        let pred = match jsonld_term(q.predicate) {
            JsonLdTerm::Iri(s) => s,
            JsonLdTerm::Literal { value, .. } => value,
        };
        arr.push(Value::Map(vec![
            (Value::Text("@id".to_string()), Value::Text(subj)),
            (Value::Text(pred), cbor_object_term(q.object)),
        ]));
    }

    ciborium::ser::into_writer(&Value::Array(arr), writer)
        .map_err(|e| format!("Failed to write vendor nquin-CBOR: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::resolver::{INLINE_TAG_INTEGER, MSB_FLAG};

    fn s(quins: &[NQuin], f: impl Fn(&mut Vec<u8>, &[NQuin]) -> Result<(), String>) -> String {
        let mut buf = Vec::new();
        f(&mut buf, quins).unwrap();
        String::from_utf8(buf).unwrap()
    }

    // A quin whose object is an inline-typed integer literal.
    fn quin(subject: u64, predicate: u64, object: u64) -> NQuin {
        NQuin {
            subject,
            predicate,
            object,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    #[test]
    fn turtle_is_valid_and_groups_by_subject() {
        // Same subject, two predicates → one block ending in a single '.'.
        let s1 = MSB_FLAG | 0x11;
        let quins = [
            quin(s1, MSB_FLAG | 0x22, MSB_FLAG | 0x33),
            quin(s1, MSB_FLAG | 0x44, MSB_FLAG | 0x55),
        ];
        let out = s(&quins, serialize_to_turtle);
        // Exactly one statement terminator.
        assert_eq!(out.matches(" .\n").count(), 1, "one '.' per subject: {out}");
        // Predicate separator present.
        assert!(out.contains(" ;\n"), "predicate list uses ';': {out}");
        // No malformed 'subject .' line (the old bug).
        assert!(
            !out.lines().next().unwrap().trim_end().ends_with("> ."),
            "subject must not be terminated alone: {out}"
        );
    }

    #[test]
    fn turtle_object_integer_is_typed_literal_not_iri() {
        let quins = [quin(
            MSB_FLAG | 0x11,
            MSB_FLAG | 0x22,
            INLINE_TAG_INTEGER | 42,
        )];
        let out = s(&quins, serialize_to_turtle);
        assert!(
            out.contains(r#""42"^^<"#),
            "integer object as typed literal: {out}"
        );
        assert!(out.contains("XMLSchema#integer"), "{out}");
    }

    #[test]
    fn jsonld_literal_object_uses_value_and_type() {
        let quins = [quin(
            MSB_FLAG | 0x11,
            MSB_FLAG | 0x22,
            INLINE_TAG_INTEGER | 7,
        )];
        let out = s(&quins, serialize_to_jsonld);
        assert!(out.contains(r#""@value": "7""#), "{out}");
        assert!(out.contains(r#""@type""#), "{out}");
        assert!(out.contains("XMLSchema#integer"), "{out}");
    }

    #[test]
    fn jsonld_iri_object_uses_id() {
        let quins = [quin(MSB_FLAG | 0x11, MSB_FLAG | 0x22, MSB_FLAG | 0x33)];
        let out = s(&quins, serialize_to_jsonld);
        assert!(out.contains(r#""@id""#), "{out}");
        // did:q42 pointer form for an unresolved MSB term.
        assert!(out.contains("did:q42:ptr/"), "{out}");
    }

    #[test]
    fn trig_wraps_statements_in_graph_braces() {
        let quins = [quin(MSB_FLAG | 0x11, MSB_FLAG | 0x22, MSB_FLAG | 0x33)];
        let out = s(&quins, serialize_to_trig);
        assert!(out.contains(" {\n"), "graph opens with '{{': {out}");
        assert!(
            out.trim_end().ends_with('}'),
            "graph closes with '}}': {out}"
        );
    }

    #[test]
    fn cborld_emits_decodable_cbor_array_of_maps() {
        // CBOR is binary, so this asserts on bytes directly (not via `s`).
        let quins = [
            quin(MSB_FLAG | 0x11, MSB_FLAG | 0x22, MSB_FLAG | 0x33),
            quin(MSB_FLAG | 0x44, MSB_FLAG | 0x55, INLINE_TAG_INTEGER | 9),
        ];
        let mut buf = Vec::new();
        serialize_to_cborld(&mut buf, &quins).unwrap();
        // Top-level CBOR array of length 2 (0x82).
        assert_eq!(buf[0], 0x82, "expected CBOR array(2), got {:#x}", buf[0]);
        // Decodes cleanly as an array of two 2-entry maps.
        let val: ciborium::value::Value = ciborium::de::from_reader(&buf[..]).unwrap();
        match val {
            ciborium::value::Value::Array(a) => {
                assert_eq!(a.len(), 2, "one map per triple");
                match &a[0] {
                    ciborium::value::Value::Map(m) => assert_eq!(m.len(), 2, "@id + one predicate"),
                    other => panic!("expected map, got {other:?}"),
                }
                // Second triple's object must be a typed-literal map, not a bare string.
                match &a[1] {
                    ciborium::value::Value::Map(m) => {
                        let obj = m
                            .iter()
                            .find(|(k, _)| !matches!(k, ciborium::value::Value::Text(t) if t == "@id"))
                            .map(|(_, v)| v)
                            .expect("predicate entry");
                        match obj {
                            ciborium::value::Value::Map(om) => {
                                let keys: Vec<_> = om
                                    .iter()
                                    .filter_map(|(k, _)| match k {
                                        ciborium::value::Value::Text(t) => Some(t.as_str()),
                                        _ => None,
                                    })
                                    .collect();
                                assert!(keys.contains(&"@value"), "{om:?}");
                                assert!(keys.contains(&"@type"), "{om:?}");
                            }
                            other => panic!("typed literal must be a map, got {other:?}"),
                        }
                    }
                    other => panic!("expected map, got {other:?}"),
                }
            }
            other => panic!("expected array, got {other:?}"),
        }
    }

    #[test]
    fn cborld_inline_integer_round_trips_bit_exact() {
        use crate::sparql_library::parsers::cbor_parser::parse_cbor_ld_into;
        use crate::sparql_library::rdf_formats::QuinCollector;
        use std::io::Cursor;

        let quins = [quin(
            MSB_FLAG | 0x11,
            MSB_FLAG | 0x22,
            INLINE_TAG_INTEGER | 42,
        )];
        let mut buf = Vec::new();
        serialize_to_cborld(&mut buf, &quins).unwrap();
        let mut out = QuinCollector::new();
        let n = parse_cbor_ld_into(Cursor::new(&buf[..]), 0, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out.as_slice()[0].object, INLINE_TAG_INTEGER | 42);
    }

    #[test]
    fn cborld_inline_bool_decimal_float_round_trip_bit_exact() {
        use crate::query::resolver::{INLINE_TAG_BOOLEAN, INLINE_TAG_DECIMAL};
        use crate::sparql_library::parsers::cbor_parser::parse_cbor_ld_into;
        use crate::sparql_library::rdf_formats::QuinCollector;
        use std::io::Cursor;

        let cases = [
            INLINE_TAG_BOOLEAN | 1,
            INLINE_TAG_DECIMAL | 3_500_000, // 3.5
            crate::frame_layout::pack_float_object(0.5),
        ];
        for object in cases {
            let quins = [quin(MSB_FLAG | 0x11, MSB_FLAG | 0x22, object)];
            let mut buf = Vec::new();
            serialize_to_cborld(&mut buf, &quins).unwrap();
            let mut out = QuinCollector::new();
            let n = parse_cbor_ld_into(Cursor::new(&buf[..]), 0, &mut out).unwrap();
            assert_eq!(n, 1, "object={object:#x}");
            assert_eq!(out.as_slice()[0].object, object, "object={object:#x}");
        }
    }

    #[test]
    fn solid_turtle_n3_jsonld_round_trip_preserve_triple_count() {
        use crate::sparql_library::rdf_formats::{parse_rdf, QuinCollector, RdfFormat};
        use std::io::Cursor;

        let s1 = MSB_FLAG | 0xAB;
        let quins = [
            quin(s1, MSB_FLAG | 0x11, MSB_FLAG | 0x21),
            quin(s1, MSB_FLAG | 0x12, INLINE_TAG_INTEGER | 7),
        ];

        assert_eq!(
            RdfFormat::from_media_type("text/turtle; charset=utf-8"),
            Some(RdfFormat::Turtle)
        );
        assert_eq!(
            RdfFormat::from_media_type("application/ld+json"),
            Some(RdfFormat::JsonLd)
        );
        assert_eq!(RdfFormat::from_media_type("text/n3"), Some(RdfFormat::N3));

        let turtle = s(&quins, serialize_to_turtle);
        let mut out = QuinCollector::new();
        let n = parse_rdf(
            RdfFormat::Turtle,
            Cursor::new(turtle.as_bytes()),
            0,
            &mut out,
        )
        .unwrap();
        assert_eq!(n, 2, "turtle round-trip: {turtle}");

        let n3 = s(&quins, serialize_to_n3);
        assert!(n3.contains("@prefix"), "n3 emits prefixes: {n3}");
        let mut out = QuinCollector::new();
        let n = parse_rdf(RdfFormat::N3, Cursor::new(n3.as_bytes()), 0, &mut out).unwrap();
        assert_eq!(n, 2, "n3 round-trip count={n}: {n3}");

        let compact = s(&quins, serialize_to_jsonld_compact);
        assert!(
            compact.contains("\"@context\""),
            "compact json-ld has @context: {compact}"
        );
        assert!(
            compact.contains("\"@graph\""),
            "compact json-ld has @graph: {compact}"
        );
        let mut out = QuinCollector::new();
        let n = parse_rdf(
            RdfFormat::JsonLd,
            Cursor::new(compact.as_bytes()),
            0,
            &mut out,
        )
        .unwrap();
        assert_eq!(n, 2, "compact json-ld round-trip: {compact}");
    }
}
