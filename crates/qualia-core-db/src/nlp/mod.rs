//! Document NLP in the engine: bytes → tokens → gazetteer → span plans.
//!
//! Lives next to `text_span` and `lexicon`. Not VibeScript. Not an MCP-only tool.
//! Year-one is tokenize + gazetteer + spans. Not FrameNet, RST, OpenIE, NLI, or MT.

pub mod coref;
pub mod emit;
pub mod frame;
pub mod fst;
pub mod gazetteer;
pub mod graphrag;
pub mod hash;
pub mod link;
pub mod normalize;
pub mod relation;
pub mod span;
pub mod substrate;
pub mod terms;
pub mod tokenize;

use emit::AnnotationPlan;
use gazetteer::{Gazetteer, Hit};
use normalize::Normalized;
use tokenize::{Sentence, Token};

/// One-shot document analysis. Desktop and Vibe hosts call this; they do not
/// reimplement tokenisation.
#[derive(Debug, Clone)]
pub struct DocumentAnalysis {
    pub source_hash: u64,
    pub token_count: usize,
    pub sentence_count: usize,
    pub hits: Vec<Hit>,
    pub norms: Vec<Normalized>,
    pub plans: Vec<AnnotationPlan>,
}

pub fn analyze_document(source: &str) -> DocumentAnalysis {
    let tokens: Vec<Token<'_>> = tokenize::tokenize(source);
    let sentences: Vec<Sentence> = tokenize::split_sentences(source);
    let hits = link::filter_known(Gazetteer::default().find(source));
    let norms = normalize::normalize_dates_and_numbers(source);
    let mut plans = emit::emit_from_hits(source, &hits);
    plans.extend(emit::emit_from_normalized(source, &norms));
    DocumentAnalysis {
        source_hash: hash::hash60(source.as_bytes()),
        token_count: tokens.len(),
        sentence_count: sentences.len(),
        hits,
        norms,
        plans,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn year_one_pipeline_on_catchment_paragraph() {
        let src = "North Spring is the reference site. Timothy Charles Holborn recorded 12.5 mm of rain on 2026-08-15.";
        let a = analyze_document(src);
        assert!(a.token_count >= 10);
        assert!(a.sentence_count >= 2);
        assert!(a.plans.len() >= 4);
        assert_ne!(a.source_hash, 0);
    }
}
