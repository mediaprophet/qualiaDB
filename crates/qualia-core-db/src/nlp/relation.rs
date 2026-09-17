//! Relation extraction — RDF-Star triple extraction via shallow patterns.
//!
//! Pattern-based, deterministic. Recognised patterns:
//!   - "X is Y"        → (X, rdf:type, Y)
//!   - "X has Y"       → (X, has, Y)
//!   - "X located in Y"→ (X, locatedIn, Y)
//!
//! Subjects/objects are noun-phrase windows around the trigger; spans are
//! exact byte offsets into the source. WASM-compatible, no LLM.

use super::span::DocSpan;
use super::tokenize::{tokenize, TokenKind};

/// One extracted relation with subject/object provenance and a confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedRelation {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub subject_span: DocSpan,
    pub object_span: DocSpan,
    pub confidence: f64,
}

/// Extract relations from `text`.
pub fn extract_relations(text: &str) -> Vec<ExtractedRelation> {
    let tokens = tokenize(text);
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        if tok.kind != TokenKind::Word {
            i += 1;
            continue;
        }
        let lower = tok.text.to_ascii_lowercase();
        match lower.as_str() {
            "is" => {
                if let Some(rel) = relation_is_a(text, &tokens, i) {
                    out.push(rel);
                }
                i += 1;
            }
            "has" => {
                if let Some(rel) = relation_has(text, &tokens, i) {
                    out.push(rel);
                }
                i += 1;
            }
            "located" => {
                // Expect "located in".
                if i + 1 < tokens.len()
                    && tokens[i + 1].kind == TokenKind::Word
                    && tokens[i + 1].text.eq_ignore_ascii_case("in")
                {
                    if let Some(rel) = relation_located_in(text, &tokens, i) {
                        out.push(rel);
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    out
}

/// "X is Y" → (X, rdf:type, Y). X = noun window before "is", Y = noun window after.
fn relation_is_a(
    text: &str,
    tokens: &[crate::nlp::tokenize::Token<'_>],
    trig: usize,
) -> Option<ExtractedRelation> {
    let subj = noun_window_before(tokens, trig)?;
    let obj = noun_window_after(tokens, trig + 1)?;
    Some(ExtractedRelation {
        subject: span_text(text, subj),
        predicate: "rdf:type".to_string(),
        object: span_text(text, obj),
        subject_span: subj,
        object_span: obj,
        confidence: 0.9,
    })
}

/// "X has Y" → (X, has, Y).
fn relation_has(
    text: &str,
    tokens: &[crate::nlp::tokenize::Token<'_>],
    trig: usize,
) -> Option<ExtractedRelation> {
    let subj = noun_window_before(tokens, trig)?;
    let obj = noun_window_after(tokens, trig + 1)?;
    Some(ExtractedRelation {
        subject: span_text(text, subj),
        predicate: "has".to_string(),
        object: span_text(text, obj),
        subject_span: subj,
        object_span: obj,
        confidence: 0.85,
    })
}

/// "X located in Y" → (X, locatedIn, Y). Trigger occupies two tokens.
fn relation_located_in(
    text: &str,
    tokens: &[crate::nlp::tokenize::Token<'_>],
    trig: usize,
) -> Option<ExtractedRelation> {
    let subj = noun_window_before(tokens, trig)?;
    let obj = noun_window_after(tokens, trig + 2)?;
    Some(ExtractedRelation {
        subject: span_text(text, subj),
        predicate: "locatedIn".to_string(),
        object: span_text(text, obj),
        subject_span: subj,
        object_span: obj,
        confidence: 0.8,
    })
}

/// Collect a contiguous noun window (word tokens, skipping leading
/// determiners) ending just before `trig`. Returns the combined span.
fn noun_window_before(tokens: &[crate::nlp::tokenize::Token<'_>], trig: usize) -> Option<DocSpan> {
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;
    for j in (0..trig).rev() {
        let t = &tokens[j];
        if t.kind == TokenKind::Word {
            if is_det(t.text) {
                continue;
            }
            start = Some(j);
            if end.is_none() {
                end = Some(j);
            }
        } else if end.is_some() {
            break;
        }
    }
    let (s, e) = (start?, end?);
    Some(DocSpan::new(
        tokens[s].span.start_utf8,
        tokens[e].span.end_utf8,
    ))
}

/// Collect a contiguous noun window starting just after `from`.
fn noun_window_after(tokens: &[crate::nlp::tokenize::Token<'_>], from: usize) -> Option<DocSpan> {
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;
    let mut j = from;
    while j < tokens.len() {
        let t = &tokens[j];
        if t.kind == TokenKind::Word {
            if is_det(t.text) && start.is_none() {
                j += 1;
                continue;
            }
            if start.is_none() {
                start = Some(j);
            }
            end = Some(j);
            j += 1;
        } else if start.is_some() {
            break;
        } else {
            j += 1;
        }
    }
    let (s, e) = (start?, end?);
    Some(DocSpan::new(
        tokens[s].span.start_utf8,
        tokens[e].span.end_utf8,
    ))
}

fn span_text(text: &str, span: DocSpan) -> String {
    span.slice(text).unwrap_or("").to_string()
}

fn is_det(text: &str) -> bool {
    matches!(text.to_ascii_lowercase().as_str(), "a" | "an" | "the")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_a_relation() {
        let src = "Socrates is a philosopher";
        let rels = extract_relations(src);
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].subject, "Socrates");
        assert_eq!(rels[0].predicate, "rdf:type");
        assert_eq!(rels[0].object, "philosopher");
        assert_eq!(&src[rels[0].subject_span.as_range()], "Socrates");
        assert_eq!(&src[rels[0].object_span.as_range()], "philosopher");
    }

    #[test]
    fn has_relation() {
        let src = "A cat has a tail";
        let rels = extract_relations(src);
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].subject, "cat");
        assert_eq!(rels[0].predicate, "has");
        assert_eq!(rels[0].object, "tail");
    }

    #[test]
    fn located_in_relation() {
        let src = "Paris located in France";
        let rels = extract_relations(src);
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].subject, "Paris");
        assert_eq!(rels[0].predicate, "locatedIn");
        assert_eq!(rels[0].object, "France");
    }

    #[test]
    fn no_relation_for_plain_text() {
        let rels = extract_relations("The quick brown fox jumps.");
        assert!(rels.is_empty());
    }

    #[test]
    fn multiple_relations() {
        let src = "Socrates is a philosopher. A cat has a tail.";
        let rels = extract_relations(src);
        assert_eq!(rels.len(), 2);
    }
}
