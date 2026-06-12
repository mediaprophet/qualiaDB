use std::collections::HashMap;
use std::path::Path;

use qualia_core_db::mini_parser::hash_token;

#[derive(Debug, Clone)]
pub enum TargetDatatype {
    Integer,
    Float,
    DateTime,
    StringRef,
}

#[derive(Debug, Clone)]
pub struct ColumnMapping {
    pub source_key: String,
    pub column_index: Option<usize>,
    pub predicate_hash: u64,
    pub datatype: TargetDatatype,
}

pub struct MappingProfile {
    pub base_class_hash: u64,
    pub fields: Vec<ColumnMapping>,
}

/// Boot Phase: parse a `.shacl.ttl` mapping file and compile it into a
/// [`MappingProfile`] for use in the zero-allocation stream phase.
///
/// Expected predicates in the Turtle file:
/// - `sh:targetClass`  — the RDF class of each row (used as subject class hint)
/// - `sh:property [ sh:path <URI> ; sh:datatype xsd:T ; qext:sourceColumn "hdr" ]`
/// - `qext:sourceJsonKey "key"` may substitute `qext:sourceColumn` for JSON inputs
pub fn compile_shacl_mapping(path: &Path) -> Result<MappingProfile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read SHACL mapping file '{}': {e}", path.display()))?;

    // ── 1. Build prefix map ───────────────────────────────────────────────
    let prefixes = parse_prefixes(&content);

    // ── 2. Extract base class ─────────────────────────────────────────────
    let base_class_hash = extract_iri_value(&content, "sh:targetClass", &prefixes)
        .map(|iri| hash_token(&iri))
        .unwrap_or(0);

    // ── 3. Parse sh:property blocks ───────────────────────────────────────
    let mut fields: Vec<ColumnMapping> = Vec::new();
    let mut search = content.as_str();

    while let Some(pos) = search.find("sh:property") {
        search = &search[pos + "sh:property".len()..];

        // Find the anonymous blank node block [ ... ]
        let open = match search.find('[') {
            Some(i) => i,
            None => break,
        };
        search = &search[open + 1..];

        // Allow nested brackets (e.g., sh:or lists)
        let close = match find_matching_bracket(search) {
            Some(i) => i,
            None => break,
        };
        let block = &search[..close];
        search = &search[close + 1..];

        let pred_iri = extract_iri_value(block, "sh:path", &prefixes);
        let dtype_str = extract_iri_value(block, "sh:datatype", &prefixes);
        let source_key = extract_string_literal(block, "qext:sourceColumn")
            .or_else(|| extract_string_literal(block, "qext:sourceJsonKey"));

        if let (Some(pred_iri), Some(key)) = (pred_iri, source_key) {
            let predicate_hash = hash_token(&pred_iri);
            let datatype = map_datatype(dtype_str.as_deref().unwrap_or(""));
            fields.push(ColumnMapping {
                source_key: key,
                column_index: None,
                predicate_hash,
                datatype,
            });
        }
    }

    if fields.is_empty() {
        return Err(format!(
            "No sh:property mappings with qext:sourceColumn/qext:sourceJsonKey found in '{}'.",
            path.display()
        ));
    }

    Ok(MappingProfile { base_class_hash, fields })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build `prefix → IRI` map from `@prefix` / `PREFIX` declarations.
fn parse_prefixes(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        let rest = if line.starts_with("@prefix") {
            line.strip_prefix("@prefix").unwrap_or("").trim()
        } else if line.to_ascii_uppercase().starts_with("PREFIX") {
            line[6..].trim()
        } else {
            continue;
        };

        // Expect:  prefix: <IRI>
        if let Some(colon) = rest.find(':') {
            let pfx = &rest[..colon + 1]; // includes the trailing colon
            let after = rest[colon + 1..].trim();
            if after.starts_with('<') {
                if let Some(end) = after.find('>') {
                    map.insert(pfx.to_string(), after[1..end].to_string());
                }
            }
        }
    }
    map
}

/// Resolve a prefixed name or bracketed IRI from the content following `predicate`.
fn extract_iri_value(block: &str, predicate: &str, prefixes: &HashMap<String, String>) -> Option<String> {
    let pos = block.find(predicate)?;
    let after = block[pos + predicate.len()..].trim_start();

    if after.starts_with('<') {
        let end = after.find('>')?;
        return Some(after[1..end].to_string());
    }

    // Prefixed name: prefix:local
    let end = after.find(|c: char| c.is_whitespace() || c == ';' || c == ',' || c == ']').unwrap_or(after.len());
    let token = &after[..end];
    if let Some(colon) = token.find(':') {
        let pfx = &token[..colon + 1];
        let local = &token[colon + 1..];
        if let Some(base) = prefixes.get(pfx) {
            return Some(format!("{}{}", base, local));
        }
        // Return as-is when prefix not found — still useful for hashing
        return Some(token.to_string());
    }

    None
}

/// Extract the contents of a double-quoted string literal following `predicate`.
fn extract_string_literal(block: &str, predicate: &str) -> Option<String> {
    let pos = block.find(predicate)?;
    let after = block[pos + predicate.len()..].trim_start();
    if !after.starts_with('"') {
        return None;
    }
    let inner = &after[1..];
    let mut result = String::new();
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            result.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            break;
        } else {
            result.push(ch);
        }
    }
    Some(result)
}

/// Find the index of the `]` that closes the opening `[` already consumed.
/// Handles one level of nesting for inner blank nodes.
fn find_matching_bracket(s: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut in_string = false;
    let mut escaped = false;
    for (i, ch) in s.char_indices() {
        if escaped { escaped = false; continue; }
        if ch == '\\' && in_string { escaped = true; continue; }
        if ch == '"' { in_string = !in_string; continue; }
        if in_string { continue; }
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 { return Some(i); }
            }
            _ => {}
        }
    }
    None
}

/// Map an XSD datatype IRI to [`TargetDatatype`].
fn map_datatype(datatype_iri: &str) -> TargetDatatype {
    let lower = datatype_iri.to_ascii_lowercase();
    if lower.contains("integer") || lower.ends_with("#int") || lower.ends_with("#long") {
        TargetDatatype::Integer
    } else if lower.contains("double") || lower.contains("float") || lower.contains("decimal") {
        TargetDatatype::Float
    } else if lower.contains("datetime") || lower.contains("date") {
        TargetDatatype::DateTime
    } else {
        TargetDatatype::StringRef
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const SAMPLE_SHACL: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix qext: <http://qualia.systems/ext#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:HealthShape a sh:NodeShape ;
    sh:targetClass ex:HealthRecord ;
    sh:property [
        sh:path ex:stepCount ;
        sh:datatype xsd:integer ;
        qext:sourceColumn "Step count"
    ] ;
    sh:property [
        sh:path ex:heartRate ;
        sh:datatype xsd:decimal ;
        qext:sourceColumn "Heart rate (bpm)"
    ] ;
    sh:property [
        sh:path ex:recordedAt ;
        sh:datatype xsd:dateTime ;
        qext:sourceColumn "Date"
    ] ;
    sh:property [
        sh:path ex:deviceName ;
        sh:datatype xsd:string ;
        qext:sourceJsonKey "device"
    ] .
"#;

    fn write_shacl(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        let mut f = std::fs::File::create(&path).expect("create shacl file");
        f.write_all(content.as_bytes()).expect("write shacl file");
        path
    }

    #[test]
    fn compile_basic_shacl() {
        let path = write_shacl("test_shacl_basic.ttl", SAMPLE_SHACL);
        let profile = compile_shacl_mapping(&path).expect("compile_shacl_mapping failed");
        assert_eq!(profile.fields.len(), 4, "should find 4 property mappings");
    }

    #[test]
    fn shacl_field_source_keys() {
        let path = write_shacl("test_shacl_keys.ttl", SAMPLE_SHACL);
        let profile = compile_shacl_mapping(&path).expect("compile");
        let keys: Vec<&str> = profile.fields.iter().map(|f| f.source_key.as_str()).collect();
        assert!(keys.contains(&"Step count"));
        assert!(keys.contains(&"Heart rate (bpm)"));
        assert!(keys.contains(&"Date"));
        assert!(keys.contains(&"device"));
    }

    #[test]
    fn shacl_datatypes_parsed_correctly() {
        let path = write_shacl("test_shacl_dtypes.ttl", SAMPLE_SHACL);
        let profile = compile_shacl_mapping(&path).expect("compile");
        for field in &profile.fields {
            match field.source_key.as_str() {
                "Step count" => assert!(matches!(field.datatype, TargetDatatype::Integer)),
                "Heart rate (bpm)" => assert!(matches!(field.datatype, TargetDatatype::Float)),
                "Date" => assert!(matches!(field.datatype, TargetDatatype::DateTime)),
                "device" => assert!(matches!(field.datatype, TargetDatatype::StringRef)),
                _ => {}
            }
        }
    }

    #[test]
    fn shacl_predicate_hashes_are_nonzero() {
        let path = write_shacl("test_shacl_hashes.ttl", SAMPLE_SHACL);
        let profile = compile_shacl_mapping(&path).expect("compile");
        for field in &profile.fields {
            assert_ne!(field.predicate_hash, 0, "predicate hash should be nonzero for '{}'", field.source_key);
        }
    }

    #[test]
    fn shacl_error_on_missing_file() {
        let result = compile_shacl_mapping(Path::new("/nonexistent/path.shacl.ttl"));
        assert!(result.is_err());
    }

    #[test]
    fn shacl_error_on_empty_mapping() {
        let path = write_shacl("test_shacl_empty.ttl", "@prefix sh: <http://www.w3.org/ns/shacl#> .");
        let result = compile_shacl_mapping(&path);
        assert!(result.is_err(), "should error when no mappings found");
    }
}
