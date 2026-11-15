//! Shared omit-predicate filters for ingest and import-aligned verify-graph.
//!
//! Applied identically on both sides so intentional omissions are proven, not accidental.

use crate::q_hash;

/// Well-known comment / definition / gloss IRIs (RDFS, SKOS, common WordNet).
pub const COMMENT_GLOSS_PREDICATE_IRIS: &[&str] = &[
    "http://www.w3.org/2000/01/rdf-schema#comment",
    "http://www.w3.org/2000/01/rdf-schema#label",
    "http://www.w3.org/2004/02/skos/core#definition",
    "http://www.w3.org/2004/02/skos/core#scopeNote",
    "http://www.w3.org/2004/02/skos/core#example",
    "http://www.w3.org/2004/02/skos/core#prefLabel",
    "http://www.w3.org/2004/02/skos/core#altLabel",
    "https://globalwordnet.org/ontology#definition",
    "http://globalwordnet.org/ontology#definition",
    "https://en-word.net/ontology#definition",
    "http://www.w3.org/2006/03/wn/wn20/schema/gloss",
];

/// Resolve CLI / script omit lists into FNV-1a predicate hashes (ingest encoding).
pub fn resolve_omit_predicate_hashes(
    iris: &[String],
    preset: Option<&str>,
) -> Result<Vec<u64>, String> {
    let mut out = Vec::new();
    if let Some(name) = preset {
        let name = name.trim().to_ascii_lowercase();
        match name.as_str() {
            "comment-gloss" | "comment_gloss" | "gloss" => {
                for iri in COMMENT_GLOSS_PREDICATE_IRIS {
                    out.push(q_hash(iri));
                }
            }
            "" => {}
            other => {
                return Err(format!(
                    "unknown omit preset '{other}' (known: comment-gloss)"
                ));
            }
        }
    }
    for iri in iris {
        let iri = iri.trim();
        if iri.is_empty() {
            continue;
        }
        out.push(q_hash(iri));
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// Lexicon string length limit (u16) — terms longer than this fail closed in Complete ingest.
pub const MAX_LEX_TERM_BYTES: usize = 65535;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_gloss_preset_is_non_empty_and_stable() {
        let a = resolve_omit_predicate_hashes(&[], Some("comment-gloss")).unwrap();
        let b = resolve_omit_predicate_hashes(&[], Some("gloss")).unwrap();
        assert!(!a.is_empty());
        assert_eq!(a, b);
        assert!(a.contains(&q_hash(COMMENT_GLOSS_PREDICATE_IRIS[0])));
    }

    #[test]
    fn unknown_preset_fails() {
        assert!(resolve_omit_predicate_hashes(&[], Some("nope")).is_err());
    }
}
