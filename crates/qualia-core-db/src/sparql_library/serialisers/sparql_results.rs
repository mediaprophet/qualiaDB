//! SPARQL Result Formatters
//!
//! Formats SPARQL query results as XML, JSON, TSV, and CSV.

use crate::sparql_ast::*;
use std::io::Write;

/// Result formatter
pub struct ResultFormatter;

impl ResultFormatter {
    fn format_value_xml<W: Write>(writer: &mut W, value: u64, lexicon: Option<&crate::q42_lex::Q42LexMmap>) -> std::io::Result<()> {
        if (value >> 60) == 1 {
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
            return writeln!(writer, r#"        <uri>&lt;&lt;{:016x}&gt;&gt;</uri>"#, value);
        }

        let uri = if let Some(bytes) = crate::resolver::resolve_hash(value) {
            String::from_utf8_lossy(bytes).into_owned()
        } else {
            format!("urn:hash:{:016x}", value)
        };
        writeln!(writer, r#"        <uri>{}</uri>"#, uri)
    }

    fn format_value_json<W: Write>(writer: &mut W, value: u64, lexicon: Option<&crate::q42_lex::Q42LexMmap>) -> std::io::Result<()> {
        if (value >> 60) == 1 {
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
            writeln!(writer, r#"      {{"#)?;
            writeln!(writer, r#"        "type": "uri","#)?;
            write!(writer, r#"        "value": "<<{:016x}>>""#, value)?;
            return writeln!(writer, r#"      }}"#);
        }

        let uri = if let Some(bytes) = crate::resolver::resolve_hash(value) {
            String::from_utf8_lossy(bytes).into_owned()
        } else {
            format!("urn:hash:{:016x}", value)
        };
        writeln!(writer, r#"      {{"#)?;
        writeln!(writer, r#"        "type": "uri","#)?;
        writeln!(writer, r#"        "value": "{}""#, uri)?;
        write!(writer, r#"      }}"#)
    }

    fn format_value_tsv<W: Write>(writer: &mut W, value: u64, lexicon: Option<&crate::q42_lex::Q42LexMmap>) -> std::io::Result<()> {
        if (value >> 60) == 1 {
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
            return write!(writer, "<<{:016x}>>", value);
        }
        
        let uri = if let Some(bytes) = crate::resolver::resolve_hash(value) {
            String::from_utf8_lossy(bytes).into_owned()
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
        writeln!(writer, r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">"#)?;
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
            
            if !first { writeln!(writer)? };
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
            if i > 0 { write!(writer, "\t")?; }
            write!(writer, "?{}", ctx.variable_hashes[*var as usize])?;
        }
        writeln!(writer)?;
        
        for row in results {
            for (i, var) in variables.iter().enumerate() {
                if i > 0 { write!(writer, "\t")?; }
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
            if i > 0 { write!(writer, ",")?; }
            write!(writer, "{}", ctx.variable_hashes[*var as usize])?;
        }
        writeln!(writer)?;
        
        for row in results {
            for (i, var) in variables.iter().enumerate() {
                if i > 0 { write!(writer, ",")?; }
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
        writeln!(writer, r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">"#)?;
        writeln!(writer, r#"  <head></head>"#)?;
        writeln!(writer, r#"  <boolean>{}</boolean>"#, result)?;
        writeln!(writer, r#"</sparql>"#)?;
        Ok(())
    }

    pub fn format_ask_json<W: Write>(writer: &mut W, result: bool) -> std::io::Result<()> {
        writeln!(writer, r#"{{"#)?;
        writeln!(writer, r#"  "head": {{}},"#)?;
        writeln!(writer, r#"  "boolean": {}"#, result)?;
        writeln!(writer, r#"}}"#)?;
        Ok(())
    }
}