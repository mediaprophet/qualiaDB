//! Vocab chunks: bounded N3/Turtle slices with a content hash (P17.1).
//!
//! Ontology extends Vibe by compiling **referenced terms**, not by shipping
//! SNOMED (or any whole ontology) in the binary. Canonical IRIs stay so
//! SPARQL/N3/SHACL need no rename layer. Versioning (hash-lock vs latest)
//! is a principal decision (P17.5); this module stores the hash and can
//! verify a lock, but never fetches "latest".

mod parse;
mod shake;

pub use parse::{parse_chunk, VocabChunk, VocabError, VocabTerm, MAX_CHUNK_BYTES, MAX_TERMS};
pub use shake::{project_referenced_iris, tree_shake, unknown_prefixed};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_program;

    const CLINIC: &str = r#"
# vibe-vocab-0.1
@prefix snomed: <http://snomed.info/id/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

snomed:386661006 rdfs:label "Fever" .
snomed:386661006 rdfs:subClassOf snomed:64572001 .
snomed:64572001 rdfs:label "Disease" .
snomed:404684003 rdfs:label "Clinical finding" .
snomed:64572001 rdfs:subClassOf snomed:404684003 .
"#;

    #[test]
    fn parse_clinic_chunk_hashes_stably() {
        let a = parse_chunk(CLINIC.as_bytes()).expect("parse");
        let b = parse_chunk(CLINIC.as_bytes()).expect("parse");
        assert_eq!(a.content_hash, b.content_hash);
        assert!(a.terms.len() >= 3);
        assert!(a.prefix_iri("snomed").unwrap().contains("snomed.info"));
    }

    #[test]
    fn tree_shake_keeps_parents() {
        let chunk = parse_chunk(CLINIC.as_bytes()).expect("parse");
        let small = tree_shake(&chunk, &["snomed:386661006"]);
        let iris: Vec<&str> = small.terms.iter().map(|t| t.iri.as_str()).collect();
        assert!(iris.iter().any(|i| i.ends_with("386661006")));
        assert!(iris.iter().any(|i| i.ends_with("64572001")));
        assert!(iris.iter().any(|i| i.ends_with("404684003")));
        assert!(small.terms.len() < chunk.terms.len() || chunk.terms.len() == 3);
    }

    #[test]
    fn oversized_chunk_fails_closed() {
        let mut src = String::from("@prefix x: <http://example.org/> .\n");
        while src.len() < MAX_CHUNK_BYTES + 8 {
            src.push_str("x:a rdfs:label \"pad\" .\n");
        }
        assert!(matches!(parse_chunk(src.as_bytes()), Err(VocabError::TooLarge { .. })));
    }

    #[test]
    fn unknown_prefixed_is_reported() {
        let chunk = parse_chunk(CLINIC.as_bytes()).expect("parse");
        let prog = parse_program(
            r#"
            prefix snomed: <http://snomed.info/id/>;
            pure fn f() { return snomed:not_a_concept; }
            "#,
        )
        .expect("parse");
        let unknown = unknown_prefixed(&prog, &[chunk]);
        assert!(
            unknown.iter().any(|(_, local, _)| local == "not_a_concept"),
            "{unknown:?}"
        );
    }

    #[test]
    fn project_iris_from_script() {
        let chunk = parse_chunk(CLINIC.as_bytes()).expect("parse");
        let prog = parse_program(
            r#"
            prefix snomed: <http://snomed.info/id/>;
            pure fn f() { return snomed:386661006; }
            "#,
        )
        .expect("parse");
        let iris = project_referenced_iris(&prog, &chunk);
        assert!(iris.iter().any(|i| i.ends_with("386661006")));
    }

    #[test]
    fn lock_hex_round_trips() {
        let chunk = parse_chunk(CLINIC.as_bytes()).expect("parse");
        let hex = chunk.hash_hex();
        assert_eq!(hex.len(), 64);
        assert!(chunk.lock_matches(&hex));
        assert!(!chunk.lock_matches("00"));
    }
}
