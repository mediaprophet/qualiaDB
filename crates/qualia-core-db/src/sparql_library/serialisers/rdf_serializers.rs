//! RDF Format Serializers for QualiaDB
//!
//! Serializes NQuin data to standard RDF formats: N-Triples, Turtle, N-Quads, TriG, N3, JSON-LD

use std::io::Write;

use crate::NQuin;

/// Serialize Quins to N-Triples format (zero-heap via resolver).
pub fn serialize_to_ntriples<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    crate::resolver::format_ntriples_to(quins, writer)
        .map_err(|e| format!("Failed to write N-Triples: {e}"))
}

/// Serialize Quins to Turtle format
pub fn serialize_to_turtle<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    // Group by subject for Turtle's subject-based structure
    let mut subjects: std::collections::HashMap<u64, Vec<&NQuin>> = std::collections::HashMap::new();
    for quin in quins {
        subjects.entry(quin.subject).or_insert_with(Vec::new).push(quin);
    }

    for (subject, quins) in subjects {
        writeln!(writer, "<{}> .", format_hash(subject))
            .map_err(|e| format!("Failed to write Turtle subject: {}", e))?;
        
        for quin in quins {
            writeln!(writer, "    <{}> <{}> ;",
                format_hash(quin.predicate),
                format_hash(quin.object)
            ).map_err(|e| format!("Failed to write Turtle predicate: {}", e))?;
        }
        
        writeln!(writer, "    .")
            .map_err(|e| format!("Failed to write Turtle terminator: {}", e))?;
    }
    Ok(())
}

/// Serialize Quins to N-Quads format (zero-heap via resolver).
pub fn serialize_to_nquads<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    crate::resolver::format_nquads_to(quins, writer)
        .map_err(|e| format!("Failed to write N-Quads: {e}"))
}

/// Serialize Quins to TriG format
pub fn serialize_to_trig<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    // Group by context for TriG's graph-based structure
    let mut contexts: std::collections::HashMap<u64, Vec<&NQuin>> = std::collections::HashMap::new();
    for quin in quins {
        contexts.entry(quin.context).or_insert_with(Vec::new).push(quin);
    }

    for (context, quins) in contexts {
        writeln!(writer, "<{}> {{", format_hash(context))
            .map_err(|e| format!("Failed to write TriG graph: {}", e))?;
        
        // Group by subject within each context
        let mut subjects: std::collections::HashMap<u64, Vec<&NQuin>> = std::collections::HashMap::new();
        for quin in quins {
            subjects.entry(quin.subject).or_insert_with(Vec::new).push(quin);
        }

        for (subject, quins) in subjects {
            writeln!(writer, "    <{}> .", format_hash(subject))
                .map_err(|e| format!("Failed to write TriG subject: {}", e))?;
            
            for quin in quins {
                writeln!(writer, "        <{}> <{}> ;",
                    format_hash(quin.predicate),
                    format_hash(quin.object)
                ).map_err(|e| format!("Failed to write TriG predicate: {}", e))?;
            }
            
            writeln!(writer, "        .")
                .map_err(|e| format!("Failed to write TriG terminator: {}", e))?;
        }
        
        writeln!(writer, "}}")
            .map_err(|e| format!("Failed to write TriG graph end: {}", e))?;
    }
    Ok(())
}

/// Serialize Quins to N3 format
pub fn serialize_to_n3<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    // N3 is similar to Turtle but with different syntax
    let mut subjects: std::collections::HashMap<u64, Vec<&NQuin>> = std::collections::HashMap::new();
    for quin in quins {
        subjects.entry(quin.subject).or_insert_with(Vec::new).push(quin);
    }

    for (subject, quins) in subjects {
        write!(writer, "<{}> ", format_hash(subject))
            .map_err(|e| format!("Failed to write N3 subject: {}", e))?;
        
        for (i, quin) in quins.iter().enumerate() {
            if i > 0 {
                write!(writer, ", ")
                    .map_err(|e| format!("Failed to write N3 comma: {}", e))?;
            }
            write!(writer, "<{}> <{}>",
                format_hash(quin.predicate),
                format_hash(quin.object)
            ).map_err(|e| format!("Failed to write N3 predicate: {}", e))?;
        }
        
        writeln!(writer, " .")
            .map_err(|e| format!("Failed to write N3 terminator: {}", e))?;
    }
    Ok(())
}

/// Serialize Quins to JSON-LD format
pub fn serialize_to_jsonld<W: Write>(writer: &mut W, quins: &[NQuin]) -> Result<(), String> {
    writeln!(writer, "[")
        .map_err(|e| format!("Failed to write JSON-LD array start: {}", e))?;

    let mut first = true;
    for quin in quins {
        if !first {
            writeln!(writer, ",")
                .map_err(|e| format!("Failed to write JSON-LD comma: {}", e))?;
        }
        first = false;

        writeln!(writer, "  {{")
            .map_err(|e| format!("Failed to write JSON-LD object start: {}", e))?;
        writeln!(writer, "    \"@id\": \"{}\",", format_hash(quin.subject))
            .map_err(|e| format!("Failed to write JSON-LD @id: {}", e))?;
        writeln!(writer, "    \"{}\": [", format_hash(quin.predicate))
            .map_err(|e| format!("Failed to write JSON-LD predicate: {}", e))?;
        writeln!(writer, "      {{")
            .map_err(|e| format!("Failed to write JSON-LD object start: {}", e))?;
        writeln!(writer, "        \"@id\": \"{}\"", format_hash(quin.object))
            .map_err(|e| format!("Failed to write JSON-LD object: {}", e))?;
        writeln!(writer, "      }}")
            .map_err(|e| format!("Failed to write JSON-LD object end: {}", e))?;
        writeln!(writer, "    ]")
            .map_err(|e| format!("Failed to write JSON-LD array end: {}", e))?;
        write!(writer, "  }}")
            .map_err(|e| format!("Failed to write JSON-LD object end: {}", e))?;
    }

    writeln!(writer, "\n]")
        .map_err(|e| format!("Failed to write JSON-LD array end: {}", e))?;

    Ok(())
}

/// Format a hash as a placeholder IRI (in practice, you'd use lexicon lookup)
fn format_hash(hash: u64) -> String {
    format!("urn:hash:{:x}", hash)
}