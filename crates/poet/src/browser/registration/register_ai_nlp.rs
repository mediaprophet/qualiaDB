//! Wave-21 Live NLP tools for the AI toolbox.
//!
//! Dual-path `NLP.*` tools for tokenization, sentence splitting, coreference,
//! frame extraction, FST lookup, gazetteer compilation, graph-augmented
//! retrieval, relation extraction, and substrate synthesis.

use super::*;

pub(super) fn nlp_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:nlp_tokenize".into(),
                label: "Tokenize".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.tokenize".into()),
                ontology_prefix: "ai".into(),
                description: "Tokenize text into words and punctuation (Host NLP.tokenize).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:nlp_split_sentences".into(),
                label: "Split Sentences".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.split_sentences".into()),
                ontology_prefix: "ai".into(),
                description: "Segment text into discrete sentence spans (Host NLP.split_sentences).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:nlp_coref_resolve".into(),
                label: "Coref Resolve".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.coref_resolve".into()),
                ontology_prefix: "ai".into(),
                description: "Multi-pass sieve coreference resolution (Host NLP.coref_resolve).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:nlp_frame_extract".into(),
                label: "Frame Extract".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.frame_extract".into()),
                ontology_prefix: "ai".into(),
                description: "Extract frame semantics and roles from text (Host NLP.frame_extract).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:nlp_fst_lookup".into(),
                label: "FST Lookup".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.fst_lookup".into()),
                ontology_prefix: "ai".into(),
                description: "Finite-state morphological lemma lookup (Host NLP.fst_lookup).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:nlp_gazetteer_build".into(),
                label: "Gazetteer Build".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.gazetteer_build".into()),
                ontology_prefix: "ai".into(),
                description: "Compile an Aho-Corasick gazetteer index (Host NLP.gazetteer_build).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:nlp_graphrag_query".into(),
                label: "GraphRAG Query".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.graphrag_query".into()),
                ontology_prefix: "ai".into(),
                description: "Graph-augmented retrieval over semantic triples (Host NLP.graphrag_query).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:nlp_relation_extract".into(),
                label: "Relation Extract".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.relation_extract".into()),
                ontology_prefix: "ai".into(),
                description: "Extract RDF-Star relation statements from text (Host NLP.relation_extract).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:nlp_substrate_extract".into(),
                label: "Substrate Extract".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.substrate_extract".into()),
                ontology_prefix: "ai".into(),
                description: "Full symbolic extraction pipeline (Host NLP.substrate_extract).".into(),
            },
            ActionType::Invoke,
        )),
    ]
}
