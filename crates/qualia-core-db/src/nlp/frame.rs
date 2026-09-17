//! Frame semantics — basic frame element identification.
//!
//! A small built-in frame lexicon (BUY, TRANSFER) is matched against lexical
//! triggers (e.g. "bought", "gave"). When a trigger fires, the surrounding
//! clause is scanned for role fillers using shallow syntactic cues
//! (prepositions "from"/"to", subject/object position). All extracted
//! elements carry exact byte-span provenance.
//!
//! Deterministic, WASM-compatible, no LLM.

use super::span::DocSpan;
use super::tokenize::{tokenize, TokenKind};

/// One frame element: a role label and the text filling it, with provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameElement {
    pub role: String,
    pub text: String,
    pub span: DocSpan,
}

/// One instantiated frame (a trigger + its filled elements).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameInstance {
    pub frame_type: String,
    pub elements: Vec<FrameElement>,
}

/// A lexical trigger for a frame.
struct Trigger {
    /// Lowercased surface that fires the frame.
    surface: &'static str,
    frame_type: &'static str,
    /// Roles filled by the clause, in canonical order.
    roles: &'static [&'static str],
}

const BUY_TRIGGER: Trigger = Trigger {
    surface: "bought",
    frame_type: "BUY",
    roles: &["buyer", "goods", "seller"],
};

const TRANSFER_TRIGGER: Trigger = Trigger {
    surface: "gave",
    frame_type: "TRANSFER",
    roles: &["donor", "recipient", "theme"],
};

const TRIGGERS: &[&Trigger] = &[&BUY_TRIGGER, &TRANSFER_TRIGGER];

/// Extract frame instances from `text`.
pub fn extract_frames(text: &str) -> Vec<FrameInstance> {
    let tokens = tokenize(text);
    let mut out = Vec::new();
    for (i, tok) in tokens.iter().enumerate() {
        if tok.kind != TokenKind::Word {
            continue;
        }
        let lower = tok.text.to_ascii_lowercase();
        let trig = TRIGGERS.iter().find(|t| t.surface == lower.as_str());
        let Some(trig) = trig else { continue };
        let elements = match trig.frame_type {
            "BUY" => extract_buy(text, &tokens, i),
            "TRANSFER" => extract_transfer(text, &tokens, i),
            _ => Vec::new(),
        };
        let elements = elements
            .into_iter()
            .filter(|e| trig.roles.contains(&e.role.as_str()))
            .collect();
        out.push(FrameInstance {
            frame_type: trig.frame_type.to_string(),
            elements,
        });
    }
    out
}

/// BUY frame: buyer = subject before "bought", goods = object after,
/// seller = noun after "from".
fn extract_buy(
    text: &str,
    tokens: &[crate::nlp::tokenize::Token<'_>],
    trig: usize,
) -> Vec<FrameElement> {
    let mut els = Vec::new();
    // buyer: nearest preceding word token.
    if let Some(buyer) = last_word_before(tokens, trig) {
        els.push(make_element("buyer", text, buyer));
    }
    // goods: first word token after the trigger (skip leading determiner "a"/"the").
    if let Some(goods) = first_word_after(tokens, trig) {
        els.push(make_element("goods", text, goods));
    }
    // seller: word token after "from".
    if let Some(from_pos) = find_word(tokens, "from", trig + 1) {
        if let Some(seller) = first_word_after(tokens, from_pos) {
            els.push(make_element("seller", text, seller));
        }
    }
    els
}

/// TRANSFER frame: donor = subject before "gave", recipient = noun after "to",
/// theme = noun between "gave" and "to".
fn extract_transfer(
    text: &str,
    tokens: &[crate::nlp::tokenize::Token<'_>],
    trig: usize,
) -> Vec<FrameElement> {
    let mut els = Vec::new();
    if let Some(donor) = last_word_before(tokens, trig) {
        els.push(make_element("donor", text, donor));
    }
    // theme: first word after trigger (until "to").
    let to_pos = find_word(tokens, "to", trig + 1);
    if let Some(theme) = first_word_after(tokens, trig) {
        els.push(make_element("theme", text, theme));
    }
    if let Some(tp) = to_pos {
        if let Some(recipient) = first_word_after(tokens, tp) {
            els.push(make_element("recipient", text, recipient));
        }
    }
    els
}

fn make_element(role: &str, _text: &str, tok: &crate::nlp::tokenize::Token<'_>) -> FrameElement {
    FrameElement {
        role: role.to_string(),
        text: tok.text.to_string(),
        span: tok.span,
    }
}

fn last_word_before<'a>(
    tokens: &'a [crate::nlp::tokenize::Token<'a>],
    idx: usize,
) -> Option<&'a crate::nlp::tokenize::Token<'a>> {
    (0..idx).rev().find_map(|i| {
        let t = &tokens[i];
        (t.kind == TokenKind::Word && !is_det(t.text)).then_some(t)
    })
}

fn first_word_after<'a>(
    tokens: &'a [crate::nlp::tokenize::Token<'a>],
    idx: usize,
) -> Option<&'a crate::nlp::tokenize::Token<'a>> {
    (idx + 1..tokens.len()).find_map(|i| {
        let t = &tokens[i];
        (t.kind == TokenKind::Word && !is_det(t.text)).then_some(t)
    })
}

fn find_word(
    tokens: &[crate::nlp::tokenize::Token<'_>],
    surface: &str,
    from: usize,
) -> Option<usize> {
    (from..tokens.len()).find(|&i| {
        tokens[i].kind == TokenKind::Word && tokens[i].text.eq_ignore_ascii_case(surface)
    })
}

fn is_det(text: &str) -> bool {
    matches!(text.to_ascii_lowercase().as_str(), "a" | "an" | "the")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_frame_full() {
        let src = "John bought a book from Mary";
        let frames = extract_frames(src);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].frame_type, "BUY");
        let roles: Vec<_> = frames[0].elements.iter().map(|e| e.role.as_str()).collect();
        assert!(roles.contains(&"buyer"));
        assert!(roles.contains(&"goods"));
        assert!(roles.contains(&"seller"));
        let buyer = frames[0]
            .elements
            .iter()
            .find(|e| e.role == "buyer")
            .unwrap();
        assert_eq!(buyer.text, "John");
        assert_eq!(&src[buyer.span.as_range()], "John");
        let goods = frames[0]
            .elements
            .iter()
            .find(|e| e.role == "goods")
            .unwrap();
        assert_eq!(goods.text, "book");
        let seller = frames[0]
            .elements
            .iter()
            .find(|e| e.role == "seller")
            .unwrap();
        assert_eq!(seller.text, "Mary");
    }

    #[test]
    fn transfer_frame() {
        let src = "Mary gave John a book";
        let frames = extract_frames(src);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].frame_type, "TRANSFER");
        let donor = frames[0]
            .elements
            .iter()
            .find(|e| e.role == "donor")
            .unwrap();
        assert_eq!(donor.text, "Mary");
    }

    #[test]
    fn transfer_frame_with_to() {
        let src = "Mary gave a book to John";
        let frames = extract_frames(src);
        assert_eq!(frames.len(), 1);
        let recipient = frames[0]
            .elements
            .iter()
            .find(|e| e.role == "recipient")
            .unwrap();
        assert_eq!(recipient.text, "John");
    }

    #[test]
    fn no_frame_for_unrelated_text() {
        let frames = extract_frames("The sky is blue today.");
        assert!(frames.is_empty());
    }

    #[test]
    fn span_provenance_exact() {
        let src = "John bought a book from Mary";
        let frames = extract_frames(src);
        let seller = frames[0]
            .elements
            .iter()
            .find(|e| e.role == "seller")
            .unwrap();
        assert_eq!(&src[seller.span.as_range()], "Mary");
    }
}
