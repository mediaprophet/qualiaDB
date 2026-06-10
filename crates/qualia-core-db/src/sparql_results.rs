//! SPARQL Result Formatters
//!
//! Formats SPARQL query results as XML, JSON, TSV, and CSV.

use crate::sparql_ast::*;
use std::io::Write;

/// Result formatter
pub struct ResultFormatter;

impl ResultFormatter {
    /// Format results as SPARQL XML
    pub fn format_xml<W: Write>(
        writer: &mut W,
        variables: &[VariableId],
        results: &[BindingRow],
        ctx: &SparqlQueryContext,
    ) -> std::io::Result<()> {
        writeln!(writer, r#"<?xml version="1.0"?>"#)?;
        writeln!(writer, r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">"#)?;
        writeln!(writer, r#"  <head>"#)?;
        writeln!(writer, r#"    <variables>"#)?;
        
        for var in variables {
            let var_name = ctx.variable_hashes[*var as usize];
            writeln!(writer, r#"      <variable name="?{}"/>"#, var_name)?;
        }
        
        writeln!(writer, r#"    </variables>"#)?;
        writeln!(writer, r#"  </head>"#)?;
        writeln!(writer, r#"  <results>"#)?;
        
        for row in results {
            writeln!(writer, r#"    <result>"#)?;
            for var in variables {
                let var_id = *var;
                if let Some(value) = row.get(var_id) {
                    writeln!(writer, r#"      <binding name="?{}">"#, ctx.variable_hashes[var_id as usize])?;
                    writeln!(writer, r#"        <uri>http://example.org/{}</uri>"#, value)?;
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
    ) -> std::io::Result<()> {
        writeln!(writer, r#"{{"#)?;
        writeln!(writer, r#"  "head": {{"vars": ["#)?;
        
        for (i, var) in variables.iter().enumerate() {
            let var_name = ctx.variable_hashes[*var as usize];
            if i > 0 {
                write!(writer, r#", "#)?;
            }
            write!(writer, r#"?{}""#, var_name)?;
        }
        
        writeln!(writer, r#"]}},"#)?;
        writeln!(writer, r#"  "results": {{"#)?;
        
        for (i, row) in results.iter().enumerate() {
            if i > 0 {
                writeln!(writer, r#","#)?;
            }
            writeln!(writer, r#"    "?{}": {{"#, i)?;
            
            for (j, var) in variables.iter().enumerate() {
                let var_id = *var;
                if let Some(value) = row.get(var_id) {
                    if j > 0 {
                        writeln!(writer, r#","#)?;
                    }
                    writeln!(writer, r#"      "?{}": {{"#, ctx.variable_hashes[var_id as usize])?;
                    writeln!(writer, r#"        "type": "uri","#)?;
                    writeln!(writer, r#"        "value": "http://example.org/{}""#, value)?;
                    writeln!(writer, r#"      }}"#)?;
                }
            }
            
            writeln!(writer, r#"    }}"#)?;
        }
        
        writeln!(writer, r#"  }}"#)?;
        writeln!(writer, r#"}}"#)?;
        
        Ok(())
    }

    /// Format results as TSV (Tab-Separated Values)
    pub fn format_tsv<W: Write>(
        writer: &mut W,
        variables: &[VariableId],
        results: &[BindingRow],
        ctx: &SparqlQueryContext,
    ) -> std::io::Result<()> {
        // Write header
        for (i, var) in variables.iter().enumerate() {
            if i > 0 {
                write!(writer, "\t")?;
            }
            write!(writer, "?{}", ctx.variable_hashes[*var as usize])?;
        }
        writeln!(writer)?;
        
        // Write rows
        for row in results {
            for (i, var) in variables.iter().enumerate() {
                if i > 0 {
                    write!(writer, "\t")?;
                }
                if let Some(value) = row.get(*var) {
                    write!(writer, "{}", value)?;
                } else {
                    write!(writer, "")?;
                }
            }
            writeln!(writer)?;
        }
        
        Ok(())
    }

    /// Format results as CSV (Comma-Separated Values)
    pub fn format_csv<W: Write>(
        writer: &mut W,
        variables: &[VariableId],
        results: &[BindingRow],
        ctx: &SparqlQueryContext,
    ) -> std::io::Result<()> {
        // Write header
        for (i, var) in variables.iter().enumerate() {
            if i > 0 {
                write!(writer, ",")?;
            }
            write!(writer, "?{}", ctx.variable_hashes[*var as usize])?;
        }
        writeln!(writer)?;
        
        // Write rows
        for row in results {
            for (i, var) in variables.iter().enumerate() {
                if i > 0 {
                    write!(writer, ",")?;
                }
                if let Some(value) = row.get(*var) {
                    write!(writer, "{}", value)?;
                } else {
                    write!(writer, "")?;
                }
            }
            writeln!(writer)?;
        }
        
        Ok(())
    }

    /// Format ASK query result as XML
    pub fn format_ask_xml<W: Write>(writer: &mut W, result: bool) -> std::io::Result<()> {
        writeln!(writer, r#"<?xml version="1.0"?>"#)?;
        writeln!(writer, r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">"#)?;
        writeln!(writer, r#"  <head>"#)?;
        writeln!(writer, r#"  </head>"#)?;
        writeln!(writer, r#"  <boolean>{}</boolean>"#, result)?;
        writeln!(writer, r#"</sparql>"#)?;
        Ok(())
    }

    /// Format ASK query result as JSON
    pub fn format_ask_json<W: Write>(writer: &mut W, result: bool) -> std::io::Result<()> {
        writeln!(writer, r#"{{"#)?;
        writeln!(writer, r#"  "head": {{"#)?;
        writeln!(writer, r#"  }},"#)?;
        writeln!(writer, r#"  "boolean": {}"#, result)?;
        writeln!(writer, r#"}}"#)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tsv() {
        let mut ctx = SparqlQueryContext::new();
        let var1 = ctx.register_variable("?x").unwrap();
        let var2 = ctx.register_variable("?y").unwrap();
        
        let mut row = BindingRow::new();
        row.set(var1, 1);
        row.set(var2, 2);
        
        let mut output = Vec::new();
        ResultFormatter::format_tsv(&mut output, &[var1, var2], &[row], &ctx).unwrap();
        
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("1"));
        assert!(output_str.contains("2"));
    }

    #[test]
    fn test_format_csv() {
        let mut ctx = SparqlQueryContext::new();
        let var1 = ctx.register_variable("?x").unwrap();
        let var2 = ctx.register_variable("?y").unwrap();
        
        let mut row = BindingRow::new();
        row.set(var1, 1);
        row.set(var2, 2);
        
        let mut output = Vec::new();
        ResultFormatter::format_csv(&mut output, &[var1, var2], &[row], &ctx).unwrap();
        
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("1"));
        assert!(output_str.contains("2"));
    }
}