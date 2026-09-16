//! Honest maturity metadata for public `NLP.*` invoke IDs.
//!
//! Existing IDs are preserved, including `NLP.graphrag_query`. Algorithm class,
//! maturity, version, and evidence are labels of what the engine actually runs
//! today — not Stanford/Stanza/spaCy parity, FrameNet, OpenIE, or GraphRAG.

use crate::ENGINE_VERSION;

/// One public NLP invoke: what it is, what class of algorithm it uses, and
/// what evaluation evidence currently exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NlpCapability {
    pub id: &'static str,
    /// `stable`, `partial`, `experimental`, or `fail-closed`.
    pub maturity: &'static str,
    /// Algorithm actually implemented. Must not name a system we do not ship.
    pub algorithm_class: &'static str,
    /// Engine semver (`ENGINE_VERSION`); not a linguistic model card.
    pub version: &'static str,
    /// What evaluation evidence currently exists (availability, not a score).
    pub evidence: &'static str,
}

/// Public `NLP.*` descriptors. Order matches `ids.rs` bind groups, not quality.
pub const NLP_CAPABILITIES: &[NlpCapability] = &[
    NlpCapability {
        id: "NLP.tokenize",
        maturity: "experimental",
        algorithm_class: "deterministic UTF-8 prototype tokenizer (byte spans; not Stanford/spaCy)",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
    NlpCapability {
        id: "NLP.split_sentences",
        maturity: "experimental",
        algorithm_class: "deterministic UTF-8 sentence-split prototype (not Stanford/spaCy)",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
    NlpCapability {
        id: "NLP.gazetteer_run",
        maturity: "experimental",
        algorithm_class: "compiled Aho-Corasick patterns over the default lexicon (not NER)",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
    NlpCapability {
        id: "NLP.gazetteer_build",
        maturity: "experimental",
        algorithm_class: "reports default compiled lexicon size (not NER; does not compile caller patterns)",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
    NlpCapability {
        id: "NLP.fst_lookup",
        maturity: "experimental",
        algorithm_class: "trie lookup over caller-supplied entries plus English suffix stripping (-s/-es/-ies/-ed/-ing); empty entries yield no results (not a language pack)",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
    NlpCapability {
        id: "NLP.coref_resolve",
        maturity: "experimental",
        algorithm_class: "bounded exact-string grouping plus experimental pronoun sieve; empty mentions skip detection (not multi-pass sieve quality)",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
    NlpCapability {
        id: "NLP.frame_extract",
        maturity: "experimental",
        algorithm_class: "small lexical-trigger rule demo (not FrameNet)",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
    NlpCapability {
        id: "NLP.relation_extract",
        maturity: "experimental",
        algorithm_class: "small pattern rule demo (is/has/located-in; not OpenIE)",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
    NlpCapability {
        id: "NLP.substrate_extract",
        maturity: "experimental",
        algorithm_class: "symbolic pipeline (tokenize → gazetteer → ISO/units → rules → exact-match coref); every word becomes a mention",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
    NlpCapability {
        id: "NLP.graphrag_query",
        maturity: "experimental",
        algorithm_class: "keyword-triple term overlap over caller-supplied triples (not GraphRAG, embeddings, or graph-neighborhood retrieval)",
        version: ENGINE_VERSION,
        evidence: "unit tests",
    },
];

/// Look up honest metadata for a public `NLP.*` invoke id.
pub fn lookup(id: &str) -> Option<&'static NlpCapability> {
    NLP_CAPABILITIES.iter().find(|cap| cap.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC_NLP_IDS: &[&str] = &[
        "NLP.tokenize",
        "NLP.split_sentences",
        "NLP.gazetteer_run",
        "NLP.gazetteer_build",
        "NLP.fst_lookup",
        "NLP.coref_resolve",
        "NLP.frame_extract",
        "NLP.relation_extract",
        "NLP.substrate_extract",
        "NLP.graphrag_query",
    ];

    #[test]
    fn table_lookup_covers_each_public_nlp_id() {
        assert_eq!(NLP_CAPABILITIES.len(), PUBLIC_NLP_IDS.len());
        for id in PUBLIC_NLP_IDS {
            let cap = lookup(id).unwrap_or_else(|| panic!("missing NLP capability for {id}"));
            assert_eq!(cap.id, *id);
            assert_eq!(cap.maturity, "experimental");
            assert_eq!(cap.version, ENGINE_VERSION);
            assert!(!cap.algorithm_class.is_empty());
            assert_eq!(cap.evidence, "unit tests");
        }
        assert!(lookup("NLP.does_not_exist").is_none());
        assert!(lookup("nlp.analyze").is_none());
    }

    #[test]
    fn fst_algorithm_class_discloses_caller_supplied_entries() {
        let cap = lookup("NLP.fst_lookup").expect("NLP.fst_lookup");
        let class = cap.algorithm_class.to_ascii_lowercase();
        assert!(
            class.contains("caller-supplied"),
            "algorithm_class must disclose caller-supplied entries, got {}",
            cap.algorithm_class
        );
        assert!(
            class.contains("empty"),
            "algorithm_class must disclose empty-entries yield no results, got {}",
            cap.algorithm_class
        );
    }

    #[test]
    fn graphrag_algorithm_class_contains_keyword() {
        let cap = lookup("NLP.graphrag_query").expect("NLP.graphrag_query");
        assert!(
            cap.algorithm_class.to_ascii_lowercase().contains("keyword"),
            "algorithm_class must contain keyword, got {}",
            cap.algorithm_class
        );
        assert_eq!(cap.maturity, "experimental");
        assert_eq!(cap.version, ENGINE_VERSION);
    }

    #[cfg(not(all(
        target_arch = "wasm32",
        feature = "wasm-ontology",
        not(any(
            feature = "wasm-logic",
            feature = "wasm-scientific",
            feature = "wasm-full"
        ))
    )))]
    #[test]
    fn every_bound_nlp_dot_id_is_in_the_table() {
        for id in crate::poet_host::invoke::ids::ALL_BOUND
            .iter()
            .copied()
            .filter(|id| id.starts_with("NLP."))
        {
            assert!(
                lookup(id).is_some(),
                "bound invoke {id} missing from NLP_CAPABILITIES"
            );
        }
    }
}
