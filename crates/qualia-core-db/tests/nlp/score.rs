//! NLP-006 local gold scorer. Exact span match against current kernels.
//!
//! This is **not** a Stanford/UD F1. Authored v0 gold that was generated from
//! those kernels will score 1.0. That number is a freeze check, not linguistic
//! quality. `minimum_score` stays unset on NLP-005 manifests, so the release
//! gate remains UNEVALUATED (D-015).

use super::corpus::gold::{gold_files, load_gold_file, GoldDoc};
use super::gate::evaluate_release_gate;
use super::manifest::{load_manifest, GateStatus};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanScore {
    pub token_exact: f64,
    pub sentence_exact: f64,
    pub gazetteer_exact: f64,
    pub date_exact: f64,
    pub quantity_exact: f64,
}

impl SpanScore {
    /// Mean of the five exact-match layers. Empty layer (no gold items and no
    /// live items) counts as 1.0.
    pub fn mean(self) -> f64 {
        (self.token_exact
            + self.sentence_exact
            + self.gazetteer_exact
            + self.date_exact
            + self.quantity_exact)
            / 5.0
    }
}

fn exact_ratio(matched: usize, gold: usize, live: usize) -> f64 {
    if gold == 0 && live == 0 {
        return 1.0;
    }
    let denom = gold.max(live);
    if denom == 0 {
        return 1.0;
    }
    matched as f64 / denom as f64
}

pub fn score_doc(doc: &GoldDoc) -> SpanScore {
    use qualia_core_db::nlp::gazetteer::Gazetteer;
    use qualia_core_db::nlp::normalize::{normalize_dates_and_numbers, Normalized};
    use qualia_core_db::nlp::tokenize::{split_sentences, tokenize, TokenKind};

    fn kind_name(kind: TokenKind) -> &'static str {
        match kind {
            TokenKind::Word => "Word",
            TokenKind::Number => "Number",
            TokenKind::Punct => "Punct",
            TokenKind::Other => "Other",
        }
    }

    let tokens = tokenize(&doc.source);
    let token_matched = tokens
        .iter()
        .zip(&doc.tokens)
        .filter(|(live, gold)| {
            live.span.start_utf8 == gold.start
                && live.span.end_utf8 == gold.end
                && live.text == gold.surface
                && kind_name(live.kind) == gold.kind
        })
        .count();
    let sentences = split_sentences(&doc.source);
    let sent_matched = sentences
        .iter()
        .zip(&doc.sentences)
        .filter(|(live, gold)| {
            live.span.start_utf8 == gold.start && live.span.end_utf8 == gold.end
        })
        .count();
    let hits = Gazetteer::default().find(&doc.source);
    let gaz_matched = hits
        .iter()
        .zip(&doc.gazetteer)
        .filter(|(live, gold)| {
            live.span.start_utf8 == gold.start
                && live.span.end_utf8 == gold.end
                && live.iri == gold.iri
        })
        .count();
    let norms = normalize_dates_and_numbers(&doc.source);
    let live_dates: Vec<_> = norms
        .iter()
        .filter_map(|n| match n {
            Normalized::DateIso { span, .. } => Some(*span),
            _ => None,
        })
        .collect();
    let date_matched = live_dates
        .iter()
        .zip(&doc.dates)
        .filter(|(live, gold)| live.start_utf8 == gold.start && live.end_utf8 == gold.end)
        .count();
    let live_qty: Vec<_> = norms
        .iter()
        .filter_map(|n| match n {
            Normalized::Number {
                span,
                unit: Some(_),
                ..
            } => Some(*span),
            _ => None,
        })
        .collect();
    let qty_matched = live_qty
        .iter()
        .zip(&doc.quantities)
        .filter(|(live, gold)| live.start_utf8 == gold.start && live.end_utf8 == gold.end)
        .count();

    SpanScore {
        token_exact: exact_ratio(token_matched, doc.tokens.len(), tokens.len()),
        sentence_exact: exact_ratio(sent_matched, doc.sentences.len(), sentences.len()),
        gazetteer_exact: exact_ratio(gaz_matched, doc.gazetteer.len(), hits.len()),
        date_exact: exact_ratio(date_matched, doc.dates.len(), live_dates.len()),
        quantity_exact: exact_ratio(qty_matched, doc.quantities.len(), live_qty.len()),
    }
}

fn corpus_mean(corpus: &str) -> f64 {
    let mut total = 0.0;
    let mut n = 0.0;
    for path in gold_files(corpus) {
        let doc = load_gold_file(&path);
        total += score_doc(&doc).mean();
        n += 1.0;
    }
    total / n
}

#[test]
fn authored_v0_gold_is_an_exact_match_freeze_not_a_quality_gate() {
    let catchment = corpus_mean("qualia-catchment-notes-v0");
    let english = corpus_mean("qualia-english-notes-v0");
    assert!(
        (catchment - 1.0).abs() < 1e-12,
        "catchment gold must match live kernels, got {catchment}"
    );
    assert!(
        (english - 1.0).abs() < 1e-12,
        "english gold must match live kernels, got {english}"
    );

    let mut shifted = load_gold_file(&gold_files("qualia-english-notes-v0")[0]);
    if let Some(tok) = shifted.tokens.first_mut() {
        tok.end = tok.end.saturating_add(1);
    }
    let broken = score_doc(&shifted);
    assert!(
        broken.token_exact < 1.0,
        "a shifted gold span must pull token_exact below 1.0, got {}",
        broken.token_exact
    );
    assert!(broken.mean() < 1.0);

    for json in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../benchmarks/nlp/manifests/nlp-005-catchment-notes-v0.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../benchmarks/nlp/manifests/nlp-005-english-notes-v0.json"
        )),
    ] {
        let mut m = load_manifest(json).expect("nlp-005 manifest");
        m.achieved_score = Some(1.0);
        let out = evaluate_release_gate(&m);
        assert!(
            !out.succeeded,
            "1.0 freeze score must not pass a release gate while minimum_score is unset"
        );
        assert_eq!(out.effective_status, GateStatus::Unevaluated);
    }
}
