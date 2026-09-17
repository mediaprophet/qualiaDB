//! Wave-21 Live NLP tools for the AI toolbox.
//!
//! Dual-path `NLP.*` tools. These are experimental prototypes: UTF-8 tokenize /
//! sentence split, exact-string coref, a tiny suffix FST, default-lexicon
//! gazetteer metadata, keyword-triple overlap (`NLP.graphrag_query` is not
//! GraphRAG), and small rule demos for frames/relations. Not FrameNet, OpenIE,
//! NER, or a morphology language pack.

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
                description: "Experimental UTF-8 tokenizer prototype: words and punctuation with byte spans (Host NLP.tokenize). Not Stanford/spaCy.".into(),
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
                description: "Experimental UTF-8 sentence-split prototype (Host NLP.split_sentences). Not Stanford/spaCy.".into(),
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
                description: "Bounded exact-string grouping plus experimental pronoun sieve (Host NLP.coref_resolve). Not multi-pass sieve quality; empty mentions skip detection.".into(),
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
                description: "Small lexical-trigger rule demo, not FrameNet (Host NLP.frame_extract).".into(),
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
                description: "Trie lookup over caller-supplied entries plus English suffix stripping (-s/-es/-ies/-ed/-ing). Empty entries yield no results. Not a language pack (Host NLP.fst_lookup).".into(),
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
                description: "Reports default compiled gazetteer lexicon size, not NER (Host NLP.gazetteer_build).".into(),
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
                description: "Experimental keyword-triple term overlap (Host NLP.graphrag_query). Not GraphRAG, embeddings, or graph-neighborhood retrieval.".into(),
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
                description: "Small is/has/located-in rule demo, not OpenIE (Host NLP.relation_extract).".into(),
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
                description: "Experimental symbolic pipeline; every word becomes a mention (Host NLP.substrate_extract).".into(),
            },
            ActionType::Invoke,
        )),
    ]
}
