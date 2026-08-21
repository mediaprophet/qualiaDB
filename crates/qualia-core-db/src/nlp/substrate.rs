//! Full symbolic pipeline — orchestrates the NLP modules end-to-end.
//!
//! Pipeline: tokenize → gazetteer → normalize → relation extract →
//! frame extract → coref. The `Substrate` owns the borrowed tokens (which
//! borrow from the caller's `text`), plus owned hits/norms/relations/frames/
//! coref chains.
//!
//! Deterministic, WASM-compatible, no LLM.

use super::coref::{resolve_coreferences, CorefChain, CorefMention, MentionKind};
use super::frame::FrameInstance;
use super::gazetteer::{Gazetteer, Hit};
use super::normalize::Normalized;
use super::relation::ExtractedRelation;
use super::tokenize::{tokenize, Token, TokenKind};

/// The full symbolic substrate extracted from a document.
#[derive(Debug, Clone)]
pub struct Substrate<'a> {
    pub tokens: Vec<Token<'a>>,
    pub hits: Vec<Hit>,
    pub norms: Vec<Normalized>,
    pub relations: Vec<ExtractedRelation>,
    pub frames: Vec<FrameInstance>,
    pub coref_chains: Vec<CorefChain>,
}

/// Extract the full substrate from `text`.
pub fn extract_substrate(text: &str) -> Substrate<'_> {
    let tokens = tokenize(text);
    let hits = Gazetteer::default().find(text);
    let norms = super::normalize::normalize_dates_and_numbers(text);
    let relations = super::relation::extract_relations(text);
    let frames = super::frame::extract_frames(text);

    // Build mentions for coref from tokens: proper (capitalised non-sentence-
    // initial words) and pronouns.
    let mentions = build_mentions(text, &tokens);
    let coref_chains = resolve_coreferences(text, mentions);

    Substrate {
        tokens,
        hits,
        norms,
        relations,
        frames,
        coref_chains,
    }
}

/// Derive coref mentions from tokens. A word is `Proper` when it is
/// capitalised and not sentence-initial; pronouns are matched against a small
/// closed set; everything else word is `Common`.
fn build_mentions(text: &str, tokens: &[Token<'_>]) -> Vec<CorefMention> {
    let mut out = Vec::new();
    let mut at_sentence_start = true;
    for tok in tokens {
        if tok.kind == TokenKind::Punct && matches!(tok.text, "." | "!" | "?") {
            at_sentence_start = true;
            continue;
        }
        if tok.kind != TokenKind::Word {
            continue;
        }
        let lower = tok.text.to_ascii_lowercase();
        if is_pronoun(&lower) {
            out.push(CorefMention {
                span: tok.span,
                text: tok.text.to_string(),
                kind: MentionKind::Pronoun,
            });
            at_sentence_start = false;
            continue;
        }
        let is_capitalised = tok
            .text
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);
        if is_capitalised && !at_sentence_start {
            out.push(CorefMention {
                span: tok.span,
                text: tok.text.to_string(),
                kind: MentionKind::Proper,
            });
        } else {
            out.push(CorefMention {
                span: tok.span,
                text: tok.text.to_string(),
                kind: MentionKind::Common,
            });
        }
        at_sentence_start = false;
    }
    let _ = text;
    out
}

fn is_pronoun(s: &str) -> bool {
    matches!(
        s,
        "he" | "him"
            | "his"
            | "himself"
            | "she"
            | "her"
            | "hers"
            | "herself"
            | "it"
            | "its"
            | "itself"
            | "they"
            | "them"
            | "their"
            | "theirs"
            | "themselves"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_paragraph() {
        let src = "John bought a book from Mary. She gave it to John.";
        let sub = extract_substrate(src);
        assert!(!sub.tokens.is_empty());
        assert_eq!(sub.frames.len(), 2);
        assert!(sub.frames.iter().any(|f| f.frame_type == "BUY"));
        assert!(sub.frames.iter().any(|f| f.frame_type == "TRANSFER"));
        // coref: "She" should link to "Mary", "John" mentions should merge.
        assert!(!sub.coref_chains.is_empty());
        // There should be a chain containing "Mary" and "She".
        let mary_she_chain = sub.coref_chains.iter().any(|c| {
            c.mentions.iter().any(|m| m.text == "Mary")
                && c.mentions.iter().any(|m| m.text == "She")
        });
        assert!(mary_she_chain, "Mary and She should corefer");
    }

    #[test]
    fn tokens_borrow_source() {
        let src = "Hello world";
        let sub = extract_substrate(src);
        assert!(sub.tokens.iter().any(|t| t.text == "Hello"));
        assert!(sub.tokens.iter().any(|t| t.text == "world"));
    }

    #[test]
    fn relations_extracted() {
        let src = "Socrates is a philosopher";
        let sub = extract_substrate(src);
        assert_eq!(sub.relations.len(), 1);
        assert_eq!(sub.relations[0].predicate, "rdf:type");
    }

    #[test]
    fn empty_input() {
        let sub = extract_substrate("");
        assert!(sub.tokens.is_empty());
        assert!(sub.frames.is_empty());
        assert!(sub.relations.is_empty());
    }
}
