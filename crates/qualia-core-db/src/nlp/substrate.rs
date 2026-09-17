//! Full symbolic pipeline — orchestrates the NLP modules end-to-end.
//!
//! Pipeline: tokenize → gazetteer → normalize → relation extract →
//! frame extract → coref. The `Substrate` owns the borrowed tokens (which
//! borrow from the caller's `text`), plus owned hits/norms/relations/frames/
//! coref chains.
//!
//! Mention generation is unchanged for NLP-001 (every word becomes a mention).
//! Coreference failures are returned; they are never reported as a successful
//! substrate. Detector quality belongs to NLP-500.

use super::coref::{
    resolve_coreferences_counted, CancellationToken, CorefChain, CorefError, CorefLimits,
    CorefMention, MentionKind,
};
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
pub fn extract_substrate(text: &str) -> Result<Substrate<'_>, CorefError> {
    extract_substrate_with_limits(text, &CorefLimits::DEFAULT)
}

/// Extract the substrate using explicit coreference limits.
pub fn extract_substrate_with_limits<'a>(
    text: &'a str,
    limits: &CorefLimits,
) -> Result<Substrate<'a>, CorefError> {
    if text.len() > limits.max_source_bytes {
        return Err(limits.source_too_large(text.len()));
    }
    let tokens = tokenize(text);
    let hits = Gazetteer::default().find(text);
    let norms = super::normalize::normalize_dates_and_numbers(text);
    let relations = super::relation::extract_relations(text);
    let frames = super::frame::extract_frames(text);

    let mentions = build_mentions(text, &tokens, limits)?;
    let (coref_chains, _) =
        resolve_coreferences_counted(text, &mentions, limits, &CancellationToken::new())?;

    Ok(Substrate {
        tokens,
        hits,
        norms,
        relations,
        frames,
        coref_chains,
    })
}

/// Derive coref mentions from tokens. A word is `Proper` when it is
/// capitalised and not sentence-initial; pronouns are matched against a small
/// closed set; every other word is `Common`. Over-broad by design until NLP-500.
fn build_mentions(
    _text: &str,
    tokens: &[Token<'_>],
    limits: &CorefLimits,
) -> Result<Vec<CorefMention>, CorefError> {
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
        if out.len() >= limits.max_mentions {
            return Err(CorefError::TooManyMentions {
                count: out.len().saturating_add(1),
                max: limits.max_mentions,
            });
        }
        let lower = tok.text.to_ascii_lowercase();
        let kind = if is_pronoun(&lower) {
            MentionKind::Pronoun
        } else {
            let is_capitalised = tok
                .text
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);
            if is_capitalised && !at_sentence_start {
                MentionKind::Proper
            } else {
                MentionKind::Common
            }
        };
        out.push(CorefMention {
            span: tok.span,
            text: tok.text.to_string(),
            kind,
        });
        at_sentence_start = false;
    }
    Ok(out)
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
        let sub = extract_substrate(src).expect("substrate");
        assert!(!sub.tokens.is_empty());
        assert_eq!(sub.frames.len(), 2);
        assert!(sub.frames.iter().any(|f| f.frame_type == "BUY"));
        assert!(sub.frames.iter().any(|f| f.frame_type == "TRANSFER"));
        assert!(!sub.coref_chains.is_empty());
        let mary_she_chain = sub.coref_chains.iter().any(|c| {
            c.mentions.iter().any(|m| m.text == "Mary")
                && c.mentions.iter().any(|m| m.text == "She")
        });
        assert!(mary_she_chain, "Mary and She should corefer");
    }

    #[test]
    fn tokens_borrow_source() {
        let src = "Hello world";
        let sub = extract_substrate(src).expect("substrate");
        assert!(sub.tokens.iter().any(|t| t.text == "Hello"));
        assert!(sub.tokens.iter().any(|t| t.text == "world"));
    }

    #[test]
    fn relations_extracted() {
        let src = "Socrates is a philosopher";
        let sub = extract_substrate(src).expect("substrate");
        assert_eq!(sub.relations.len(), 1);
        assert_eq!(sub.relations[0].predicate, "rdf:type");
    }

    #[test]
    fn empty_input() {
        let sub = extract_substrate("").expect("empty substrate");
        assert!(sub.tokens.is_empty());
        assert!(sub.frames.is_empty());
        assert!(sub.relations.is_empty());
        assert!(sub.coref_chains.is_empty());
    }

    #[test]
    fn coref_failure_is_not_a_successful_substrate() {
        let mut limits = CorefLimits::DEFAULT;
        limits.max_mentions = 2;
        let src = "alpha beta gamma";
        assert!(matches!(
            extract_substrate_with_limits(src, &limits),
            Err(CorefError::TooManyMentions { .. })
        ));
    }
}
