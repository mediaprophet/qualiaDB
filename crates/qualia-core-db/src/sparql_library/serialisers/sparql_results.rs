//! SPARQL Result Formatters
//!
//! Formats SPARQL query results as XML, JSON, TSV, and CSV.

use crate::sparql_ast::*;
use std::io::Write;

/// Result formatter
pub struct ResultFormatter;

impl ResultFormatter {
    fn format_value_xml<W: Write>(
        writer: &mut W,
        value: u64,
        lexicon: Option<&crate::q42_lex::Q42LexMmap>,
    ) -> std::io::Result<()> {
        // 1. SPARQL-Star embedded triple (only when the lexicon resolves it —
        //    the tag collides with xsd:integer).
        if (value & crate::resolver::MSB_FLAG) == 0
            && (value & crate::resolver::INLINE_TAG_MASK) == crate::resolver::TAG_EMBEDDED
        {
            if let Some(lex) = lexicon {
                if let Some([s, p, o]) = lex.lookup_embedded_triple(value) {
                    writeln!(writer, r#"        <triple>"#)?;
                    writeln!(writer, r#"          <subject>"#)?;
                    Self::format_value_xml(writer, s, lexicon)?;
                    writeln!(writer, r#"          </subject>"#)?;
                    writeln!(writer, r#"          <predicate>"#)?;
                    Self::format_value_xml(writer, p, lexicon)?;
                    writeln!(writer, r#"          </predicate>"#)?;
                    writeln!(writer, r#"          <object>"#)?;
                    Self::format_value_xml(writer, o, lexicon)?;
                    writeln!(writer, r#"          </object>"#)?;
                    return writeln!(writer, r#"        </triple>"#);
                }
            }
        }

        // 2. Inline-typed literal → <literal datatype="...">…</literal>
        //    (previously always emitted as <uri>).
        if let Some(lit) = crate::resolver::classify_inline_literal(value) {
            return writeln!(
                writer,
                r#"        <literal datatype="{}">{}</literal>"#,
                lit.datatype_iri(),
                lit
            );
        }

        // 3. IRI: lexicon-resolved, else did:q42 pointer, else hash fallback.
        let uri = if let Some(bytes) = crate::resolver::resolve_hash(value) {
            String::from_utf8_lossy(bytes).into_owned()
        } else if (value & crate::resolver::MSB_FLAG) != 0 {
            format!("did:q42:ptr/{:016x}", value & !crate::resolver::MSB_FLAG)
        } else {
            format!("urn:hash:{:016x}", value)
        };
        writeln!(writer, r#"        <uri>{}</uri>"#, uri)
    }

    fn format_value_json<W: Write>(
        writer: &mut W,
        value: u64,
        lexicon: Option<&crate::q42_lex::Q42LexMmap>,
    ) -> std::io::Result<()> {
        // 1. SPARQL-Star embedded triple. Its tag shares the bit pattern of
        //    xsd:integer, so only take this branch when the lexicon actually
        //    resolves the value to a triple (else it is an inline integer).
        if (value & crate::resolver::MSB_FLAG) == 0
            && (value & crate::resolver::INLINE_TAG_MASK) == crate::resolver::TAG_EMBEDDED
        {
            if let Some(lex) = lexicon {
                if let Some([s, p, o]) = lex.lookup_embedded_triple(value) {
                    writeln!(writer, r#"      {{"#)?;
                    writeln!(writer, r#"        "type": "triple","#)?;
                    writeln!(writer, r#"        "value": {{"#)?;
                    write!(writer, r#"          "subject": "#)?;
                    Self::format_value_json(writer, s, lexicon)?;
                    writeln!(writer, r#","#)?;
                    write!(writer, r#"          "predicate": "#)?;
                    Self::format_value_json(writer, p, lexicon)?;
                    writeln!(writer, r#","#)?;
                    write!(writer, r#"          "object": "#)?;
                    Self::format_value_json(writer, o, lexicon)?;
                    writeln!(writer, r#""#)?;
                    writeln!(writer, r#"        }}"#)?;
                    return write!(writer, r#"      }}"#);
                }
            }
        }

        // 2. Inline-typed literal (xsd:integer/decimal/boolean/float). Previously
        //    every non-embedded value was emitted as "type":"uri", so literals
        //    were mistyped in the SPARQL 1.1 JSON results.
        if let Some(lit) = crate::resolver::classify_inline_literal(value) {
            writeln!(writer, r#"      {{"#)?;
            writeln!(writer, r#"        "type": "literal","#)?;
            writeln!(writer, r#"        "value": "{}","#, lit)?;
            writeln!(writer, r#"        "datatype": "{}""#, lit.datatype_iri())?;
            return write!(writer, r#"      }}"#);
        }

        // 3. IRI: lexicon-resolved, else did:q42 pointer (MSB set), else hash
        //    fallback. (Blank nodes are hashed into the IRI space at ingest and
        //    cannot be distinguished here — a known ingest-layer limitation.)
        let uri = if let Some(bytes) = crate::resolver::resolve_hash(value) {
            String::from_utf8_lossy(bytes).into_owned()
        } else if (value & crate::resolver::MSB_FLAG) != 0 {
            format!("did:q42:ptr/{:016x}", value & !crate::resolver::MSB_FLAG)
        } else {
            format!("urn:hash:{:016x}", value)
        };
        writeln!(writer, r#"      {{"#)?;
        writeln!(writer, r#"        "type": "uri","#)?;
        writeln!(writer, r#"        "value": "{}""#, uri)?;
        write!(writer, r#"      }}"#)
    }

    fn format_value_tsv<W: Write>(
        writer: &mut W,
        value: u64,
        lexicon: Option<&crate::q42_lex::Q42LexMmap>,
    ) -> std::io::Result<()> {
        // 1. SPARQL-Star embedded triple — only when the lexicon resolves it
        //    (the tag bits collide with xsd:integer, so a failed lookup must
        //    fall through to the inline-literal case, not be emitted as `<<…>>`).
        if (value & crate::resolver::MSB_FLAG) == 0
            && (value & crate::resolver::INLINE_TAG_MASK) == crate::resolver::TAG_EMBEDDED
        {
            if let Some(lex) = lexicon {
                if let Some([s, p, o]) = lex.lookup_embedded_triple(value) {
                    write!(writer, "<<")?;
                    Self::format_value_tsv(writer, s, lexicon)?;
                    write!(writer, " ")?;
                    Self::format_value_tsv(writer, p, lexicon)?;
                    write!(writer, " ")?;
                    Self::format_value_tsv(writer, o, lexicon)?;
                    return write!(writer, ">>");
                }
            }
        }

        // 2. Inline-typed literal → SPARQL term syntax (TSV/CSV encode RDF terms
        //    as in the query language), e.g. `"42"^^<…#integer>`.
        if let Some(lit) = crate::resolver::classify_inline_literal(value) {
            return write!(writer, "\"{}\"^^<{}>", lit, lit.datatype_iri());
        }

        // 3. IRI: lexicon-resolved, else did:q42 pointer, else hash fallback.
        let uri = if let Some(bytes) = crate::resolver::resolve_hash(value) {
            String::from_utf8_lossy(bytes).into_owned()
        } else if (value & crate::resolver::MSB_FLAG) != 0 {
            format!("did:q42:ptr/{:016x}", value & !crate::resolver::MSB_FLAG)
        } else {
            format!("urn:hash:{:016x}", value)
        };
        write!(writer, "<{}>", uri)
    }

    /// Format results as SPARQL XML
    pub fn format_xml<W: Write>(
        writer: &mut W,
        variables: &[VariableId],
        results: &[BindingRow],
        ctx: &SparqlQueryContext,
        lexicon: Option<&crate::q42_lex::Q42LexMmap>,
    ) -> std::io::Result<()> {
        writeln!(writer, r#"<?xml version="1.0"?>"#)?;
        writeln!(
            writer,
            r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">"#
        )?;
        writeln!(writer, r#"  <head>"#)?;
        writeln!(writer, r#"    <variables>"#)?;

        for var in variables {
            let var_name = ctx.variable_hashes[*var as usize];
            writeln!(writer, r#"      <variable name="{}"/>"#, var_name)?;
        }

        writeln!(writer, r#"    </variables>"#)?;
        writeln!(writer, r#"  </head>"#)?;
        writeln!(writer, r#"  <results>"#)?;

        for row in results {
            writeln!(writer, r#"    <result>"#)?;
            for var in variables {
                let var_id = *var;
                if let Some(value) = row.get(var_id) {
                    let var_name = ctx.variable_hashes[var_id as usize];
                    writeln!(writer, r#"      <binding name="{}">"#, var_name)?;
                    Self::format_value_xml(writer, value, lexicon)?;
                    writeln!(writer, r#"      </binding>"#)?;
                }
            }
            writeln!(writer, r#"    </result>"#)?;
        }

        writeln!(writer, r#"  </results>"#)?;
        writeln!(writer, r#"</sparql>"#)?;

        Ok(())
    }

    /// Format results as SPARQL JSON
    pub fn format_json<W: Write>(
        writer: &mut W,
        variables: &[VariableId],
        results: &[BindingRow],
        ctx: &SparqlQueryContext,
        lexicon: Option<&crate::q42_lex::Q42LexMmap>,
    ) -> std::io::Result<()> {
        writeln!(writer, r#"{{"#)?;
        writeln!(writer, r#"  "head": {{"vars": ["#)?;

        for (i, var) in variables.iter().enumerate() {
            let var_name = ctx.variable_hashes[*var as usize];
            if i > 0 {
                write!(writer, r#", "#)?;
            }
            write!(writer, r#""{}""#, var_name)?;
        }

        writeln!(writer, r#"]}},"#)?;
        writeln!(writer, r#"  "results": {{"#)?;
        writeln!(writer, r#"    "bindings": ["#)?;

        for (i, row) in results.iter().enumerate() {
            if i > 0 {
                writeln!(writer, r#","#)?;
            }
            writeln!(writer, r#"      {{"#)?;

            let mut first = true;
            for var in variables {
                let var_id = *var;
                if let Some(value) = row.get(var_id) {
                    if !first {
                        writeln!(writer, r#","#)?;
                    }
                    first = false;
                    let var_name = ctx.variable_hashes[var_id as usize];
                    writeln!(writer, r#"        "{}": "#, var_name)?;
                    Self::format_value_json(writer, value, lexicon)?;
                }
            }

            if !first {
                writeln!(writer)?
            };
            write!(writer, r#"      }}"#)?;
        }
        writeln!(writer)?;

        writeln!(writer, r#"    ]"#)?;
        writeln!(writer, r#"  }}"#)?;
        writeln!(writer, r#"}}"#)?;

        Ok(())
    }

    /// Format results as TSV
    pub fn format_tsv<W: Write>(
        writer: &mut W,
        variables: &[VariableId],
        results: &[BindingRow],
        ctx: &SparqlQueryContext,
        lexicon: Option<&crate::q42_lex::Q42LexMmap>,
    ) -> std::io::Result<()> {
        for (i, var) in variables.iter().enumerate() {
            if i > 0 {
                write!(writer, "\t")?;
            }
            write!(writer, "?{}", ctx.variable_hashes[*var as usize])?;
        }
        writeln!(writer)?;

        for row in results {
            for (i, var) in variables.iter().enumerate() {
                if i > 0 {
                    write!(writer, "\t")?;
                }
                if let Some(value) = row.get(*var) {
                    Self::format_value_tsv(writer, value, lexicon)?;
                }
            }
            writeln!(writer)?;
        }
        Ok(())
    }

    /// Format results as CSV
    pub fn format_csv<W: Write>(
        writer: &mut W,
        variables: &[VariableId],
        results: &[BindingRow],
        ctx: &SparqlQueryContext,
        lexicon: Option<&crate::q42_lex::Q42LexMmap>,
    ) -> std::io::Result<()> {
        for (i, var) in variables.iter().enumerate() {
            if i > 0 {
                write!(writer, ",")?;
            }
            write!(writer, "{}", ctx.variable_hashes[*var as usize])?;
        }
        writeln!(writer)?;

        for row in results {
            for (i, var) in variables.iter().enumerate() {
                if i > 0 {
                    write!(writer, ",")?;
                }
                if let Some(value) = row.get(*var) {
                    let mut temp = Vec::new();
                    Self::format_value_tsv(&mut temp, value, lexicon)?;
                    let s = String::from_utf8_lossy(&temp);
                    if s.contains(',') || s.contains('"') || s.contains('\n') {
                        write!(writer, "\"{}\"", s.replace('"', "\"\""))?;
                    } else {
                        write!(writer, "{}", s)?;
                    }
                }
            }
            writeln!(writer)?;
        }
        Ok(())
    }

    pub fn format_ntriples<W: Write>(
        writer: &mut W,
        results: &[BindingRow],
    ) -> std::io::Result<()> {
        for row in results {
            let s = row.get(0).unwrap_or(0);
            let p = row.get(1).unwrap_or(0);
            let o = row.get(2).unwrap_or(0);
            let quin = crate::NQuin {
                subject: s,
                predicate: p,
                object: o,
                context: 0,
                metadata: 0,
                parity: 0,
            };
            crate::resolver::format_ntriples_to(&[quin], writer)?;
        }
        Ok(())
    }

    pub fn format_ask_xml<W: Write>(writer: &mut W, result: bool) -> std::io::Result<()> {
        writeln!(writer, r#"<?xml version="1.0"?>"#)?;
        writeln!(
            writer,
            r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">"#
        )?;
        writeln!(writer, r#"  <head></head>"#)?;
        writeln!(writer, r#"  <boolean>{}</boolean>"#, result)?;
        writeln!(writer, r#"</sparql>"#)?;
        Ok(())
    }

    #[cfg(test)]
    fn value_json_string(value: u64) -> String {
        let mut buf = Vec::new();
        Self::format_value_json(&mut buf, value, None).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[cfg(test)]
    fn value_xml_string(value: u64) -> String {
        let mut buf = Vec::new();
        Self::format_value_xml(&mut buf, value, None).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[cfg(test)]
    fn value_tsv_string(value: u64) -> String {
        let mut buf = Vec::new();
        Self::format_value_tsv(&mut buf, value, None).unwrap();
        String::from_utf8(buf).unwrap()
    }

    pub fn format_ask_json<W: Write>(writer: &mut W, result: bool) -> std::io::Result<()> {
        writeln!(writer, r#"{{"#)?;
        writeln!(writer, r#"  "head": {{}},"#)?;
        writeln!(writer, r#"  "boolean": {}"#, result)?;
        writeln!(writer, r#"}}"#)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ResultFormatter;
    use crate::resolver::{
        INLINE_TAG_BOOLEAN, INLINE_TAG_DECIMAL, INLINE_TAG_FLOAT, INLINE_TAG_INTEGER,
    };

    // Regression: inline-typed literals were previously emitted as `"type":"uri"`.
    // These assert the SPARQL 1.1 Results form now reports literal + xsd datatype.

    #[test]
    fn inline_integer_serialises_as_typed_literal_json() {
        let json = ResultFormatter::value_json_string(INLINE_TAG_INTEGER | 42);
        assert!(json.contains(r#""type": "literal""#), "got: {json}");
        assert!(json.contains(r#""value": "42""#), "got: {json}");
        assert!(
            json.contains("XMLSchema#integer"),
            "expected xsd:integer datatype, got: {json}"
        );
        assert!(!json.contains(r#""type": "uri""#), "must not be a uri: {json}");
    }

    #[test]
    fn inline_boolean_serialises_as_typed_literal_json() {
        let json = ResultFormatter::value_json_string(INLINE_TAG_BOOLEAN | 1);
        assert!(json.contains(r#""type": "literal""#), "got: {json}");
        assert!(json.contains(r#""value": "true""#), "got: {json}");
        assert!(json.contains("XMLSchema#boolean"), "got: {json}");
    }

    #[test]
    fn inline_decimal_serialises_as_typed_literal_json() {
        // 3.5 encoded as fixed-point ×10^6 = 3_500_000.
        let json = ResultFormatter::value_json_string(INLINE_TAG_DECIMAL | 3_500_000);
        assert!(json.contains("XMLSchema#decimal"), "got: {json}");
        assert!(json.contains(r#""value": "3.500000""#), "got: {json}");
    }

    #[test]
    fn inline_float_serialises_as_typed_literal_json() {
        let val = INLINE_TAG_FLOAT | (0.5f32.to_bits() as u64);
        let json = ResultFormatter::value_json_string(val);
        assert!(json.contains("XMLSchema#float"), "got: {json}");
        assert!(json.contains(r#""value": "0.5""#), "got: {json}");
    }

    #[test]
    fn inline_integer_serialises_as_typed_literal_xml() {
        let xml = ResultFormatter::value_xml_string(INLINE_TAG_INTEGER | 7);
        assert!(xml.contains("<literal"), "expected <literal>, got: {xml}");
        assert!(xml.contains("XMLSchema#integer"), "got: {xml}");
        assert!(xml.contains(">7</literal>"), "got: {xml}");
        assert!(!xml.contains("<uri>"), "must not be a <uri>: {xml}");
    }

    #[test]
    fn unresolved_hash_still_serialises_as_uri_json() {
        // A plain (untagged, non-lexicon) hash keeps the uri fallback.
        let json = ResultFormatter::value_json_string(0x0123_4567_89ab_cdef);
        assert!(json.contains(r#""type": "uri""#), "got: {json}");
    }

    // Regression: an inline xsd:integer shares the embedded-triple tag bits.
    // Without a lexicon it must serialise as a typed literal in TSV/CSV, NOT be
    // mis-emitted as an `<<…>>` embedded triple.
    #[test]
    fn inline_integer_serialises_as_typed_literal_tsv() {
        let tsv = ResultFormatter::value_tsv_string(INLINE_TAG_INTEGER | 42);
        assert!(tsv.contains(r#""42"^^<"#), "got: {tsv}");
        assert!(tsv.contains("XMLSchema#integer"), "got: {tsv}");
        assert!(!tsv.contains("<<"), "must not be an embedded triple: {tsv}");
    }
}
