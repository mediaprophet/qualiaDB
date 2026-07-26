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
/// N3's core triple syntax is Turtle-compatible; this emits the Turtle subset
/// (subject with a `;`-separated predicate–object list) which is valid N3.
pub fn serialize_to_n3<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
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

/// Serialize Quins to CBOR-LD: a CBOR array of JSON-LD-shaped node maps
/// (`{"@id": <subject>, <predicate>: <object>}`), one map per triple.
///
/// One map per triple (rather than grouping a subject's predicates or using
/// array values) is deliberate: the streaming CBOR-LD parser
/// (`cbor_parser::parse_cbor_ld_stream`) reads exactly one value per map key
/// and re-hashes term *strings* with the same `generate_60bit_token`, so this
/// shape round-trips to identical term hashes. CBOR array values and duplicate
/// keys are not re-hashable by that parser and would silently drop objects.
///
/// Fidelity boundary (honest, not a substitution): IRIs resolved through the
/// lexicon round-trip to the identical hash. Inline-typed literals are written
/// in their lexical form (e.g. `"42"`); their tag-encoded identity cannot be
/// reconstructed through a string-hashing CBOR-LD reader. Terms that resolve to
/// neither (unknown hashes) are written as their `quin:hash/…` /
/// `did:q42:ptr/…` surface form.
pub fn serialize_to_cborld<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    use ciborium::value::Value;

    let term_string = |h: u64| -> String {
        match jsonld_term(h) {
            JsonLdTerm::Iri(s) => s,
            JsonLdTerm::Literal { value, .. } => value,
        }
    };

    let mut arr: Vec<Value> = Vec::with_capacity(quins.len());
    for q in quins {
        arr.push(Value::Map(vec![
            (
                Value::Text("@id".to_string()),
                Value::Text(term_string(q.subject)),
            ),
            (
                Value::Text(term_string(q.predicate)),
                Value::Text(term_string(q.object)),
            ),
        ]));
    }

    ciborium::ser::into_writer(&Value::Array(arr), writer)
        .map_err(|e| format!("Failed to write CBOR-LD: {e}"))
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
            }
            other => panic!("expected array, got {other:?}"),
        }
    }
}
