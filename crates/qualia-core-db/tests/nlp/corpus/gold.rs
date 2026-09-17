//! Load NLP-005 gold JSON and check UTF-8 span provenance.
//!
//! Matching the live tokenizer / gazetteer / normalizer is a contract that
//! claimed hits are true of *current* kernels. It is not an F1 or Stanford gate.

use super::super::json_lite::{parse_json, Json};
use qualia_core_db::nlp::gazetteer::Gazetteer;
use qualia_core_db::nlp::normalize::{normalize_dates_and_numbers, Normalized};
use qualia_core_db::nlp::tokenize::{split_sentences, tokenize, TokenKind};

pub(crate) fn datasets_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/benchmark-datasets/nlp")
}

#[derive(Debug, Clone)]
pub(crate) struct GoldDoc {
    pub corpus_id: String,
    pub doc_id: String,
    pub split: String,
    pub language: String,
    pub domain: String,
    pub sensitivity: String,
    pub source: String,
    pub gazetteer: Vec<GazetteerAnn>,
    pub dates: Vec<DateAnn>,
    pub quantities: Vec<QuantityAnn>,
    pub tokens: Vec<TokenAnn>,
    pub sentences: Vec<SpanAnn>,
}

#[derive(Debug, Clone)]
pub(crate) struct SpanAnn {
    pub start: u32,
    pub end: u32,
    pub surface: String,
}

#[derive(Debug, Clone)]
pub(crate) struct GazetteerAnn {
    pub start: u32,
    pub end: u32,
    pub surface: String,
    pub iri: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DateAnn {
    pub start: u32,
    pub end: u32,
    pub surface: String,
    pub yyyy_mm_dd: String,
}

#[derive(Debug, Clone)]
pub(crate) struct QuantityAnn {
    pub start: u32,
    pub end: u32,
    pub surface: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TokenAnn {
    pub start: u32,
    pub end: u32,
    pub surface: String,
    pub kind: String,
}

pub(crate) fn load_gold_file(path: &std::path::Path) -> GoldDoc {
    let raw = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let v = parse_json(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    load_gold_value(&v)
}

fn load_gold_value(v: &Json) -> GoldDoc {
    GoldDoc {
        corpus_id: req_str(v, "corpus_id"),
        doc_id: req_str(v, "doc_id"),
        split: req_str(v, "split"),
        language: req_str(v, "language"),
        domain: req_str(v, "domain"),
        sensitivity: req_str(v, "sensitivity"),
        source: req_str(v, "source"),
        gazetteer: req_array(v, "gazetteer")
            .iter()
            .map(load_gazetteer)
            .collect(),
        dates: req_array(v, "dates").iter().map(load_date).collect(),
        quantities: req_array(v, "quantities")
            .iter()
            .map(load_quantity)
            .collect(),
        tokens: req_array(v, "tokens").iter().map(load_token).collect(),
        sentences: req_array(v, "sentences").iter().map(load_span).collect(),
    }
}

fn field<'a>(v: &'a Json, key: &str) -> &'a Json {
    v.field(key)
        .unwrap_or_else(|| panic!("missing field {key:?}"))
}

fn req_str(v: &Json, key: &str) -> String {
    field(v, key)
        .as_str()
        .unwrap_or_else(|| panic!("{key} must be a string"))
        .to_owned()
}

fn req_u32(v: &Json, key: &str) -> u32 {
    let n = field(v, key)
        .as_i64()
        .unwrap_or_else(|| panic!("{key} must be an integer"));
    u32::try_from(n).unwrap_or_else(|_| panic!("{key} out of u32 range"))
}

fn req_f64(v: &Json, key: &str) -> f64 {
    match field(v, key) {
        Json::Float(x) => *x,
        Json::Int(n) => *n as f64,
        other => panic!("{key} must be a number, got {other:?}"),
    }
}

fn req_array<'a>(v: &'a Json, key: &str) -> &'a [Json] {
    field(v, key)
        .as_array()
        .unwrap_or_else(|| panic!("{key} must be an array"))
}

fn load_span(v: &Json) -> SpanAnn {
    SpanAnn {
        start: req_u32(v, "start"),
        end: req_u32(v, "end"),
        surface: req_str(v, "surface"),
    }
}

fn load_gazetteer(v: &Json) -> GazetteerAnn {
    GazetteerAnn {
        start: req_u32(v, "start"),
        end: req_u32(v, "end"),
        surface: req_str(v, "surface"),
        iri: req_str(v, "iri"),
    }
}

fn load_date(v: &Json) -> DateAnn {
    DateAnn {
        start: req_u32(v, "start"),
        end: req_u32(v, "end"),
        surface: req_str(v, "surface"),
        yyyy_mm_dd: req_str(v, "yyyy_mm_dd"),
    }
}

fn load_quantity(v: &Json) -> QuantityAnn {
    QuantityAnn {
        start: req_u32(v, "start"),
        end: req_u32(v, "end"),
        surface: req_str(v, "surface"),
        value: req_f64(v, "value"),
        unit: req_str(v, "unit"),
    }
}

fn load_token(v: &Json) -> TokenAnn {
    TokenAnn {
        start: req_u32(v, "start"),
        end: req_u32(v, "end"),
        surface: req_str(v, "surface"),
        kind: req_str(v, "kind"),
    }
}

pub(crate) fn gold_files(corpus: &str) -> Vec<std::path::PathBuf> {
    let dir = datasets_root().join(corpus).join("gold");
    let mut out: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.expect("dirent").path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no gold JSON in {}", dir.display());
    out
}

pub(crate) fn assert_utf8_span(source: &str, start: u32, end: u32, surface: &str, label: &str) {
    let s = start as usize;
    let e = end as usize;
    assert!(
        s < e && e <= source.len(),
        "{label}: span {start}..{end} not inside source len {}",
        source.len()
    );
    assert!(
        source.is_char_boundary(s) && source.is_char_boundary(e),
        "{label}: span {start}..{end} is not on a UTF-8 boundary"
    );
    let slice = &source[s..e];
    assert_eq!(
        slice, surface,
        "{label}: source[{start}..{end}] == {slice:?} != gold surface {surface:?}"
    );
}

fn kind_name(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Word => "Word",
        TokenKind::Number => "Number",
        TokenKind::Punct => "Punct",
        TokenKind::Other => "Other",
    }
}

pub(crate) fn assert_gold_spans(doc: &GoldDoc) {
    assert!(!doc.source.is_empty(), "{}: empty source", doc.doc_id);
    for h in &doc.gazetteer {
        assert_utf8_span(&doc.source, h.start, h.end, &h.surface, "gazetteer");
    }
    for d in &doc.dates {
        assert_utf8_span(&doc.source, d.start, d.end, &d.surface, "date");
        assert_eq!(d.surface, d.yyyy_mm_dd);
    }
    for q in &doc.quantities {
        assert_utf8_span(&doc.source, q.start, q.end, &q.surface, "quantity");
    }
    for t in &doc.tokens {
        assert_utf8_span(&doc.source, t.start, t.end, &t.surface, "token");
    }
    for s in &doc.sentences {
        assert_utf8_span(&doc.source, s.start, s.end, &s.surface, "sentence");
    }
}

pub(crate) fn assert_matches_live_kernels(doc: &GoldDoc) {
    let tokens = tokenize(&doc.source);
    assert_eq!(tokens.len(), doc.tokens.len(), "{} token count", doc.doc_id);
    for (live, gold) in tokens.iter().zip(&doc.tokens) {
        assert_eq!(live.span.start_utf8, gold.start, "{} token start", doc.doc_id);
        assert_eq!(live.span.end_utf8, gold.end, "{} token end", doc.doc_id);
        assert_eq!(live.text, gold.surface.as_str(), "{} token surface", doc.doc_id);
        assert_eq!(kind_name(live.kind), gold.kind.as_str(), "{} token kind", doc.doc_id);
    }

    let sentences = split_sentences(&doc.source);
    assert_eq!(
        sentences.len(),
        doc.sentences.len(),
        "{} sentence count",
        doc.doc_id
    );
    for (live, gold) in sentences.iter().zip(&doc.sentences) {
        assert_eq!(live.span.start_utf8, gold.start, "{} sentence start", doc.doc_id);
        assert_eq!(live.span.end_utf8, gold.end, "{} sentence end", doc.doc_id);
        assert_eq!(
            &doc.source[live.span.as_range()],
            gold.surface.as_str(),
            "{} sentence surface",
            doc.doc_id
        );
    }

    let hits = Gazetteer::default().find(&doc.source);
    assert_eq!(hits.len(), doc.gazetteer.len(), "{} gazetteer count", doc.doc_id);
    for (live, gold) in hits.iter().zip(&doc.gazetteer) {
        assert_eq!(live.span.start_utf8, gold.start);
        assert_eq!(live.span.end_utf8, gold.end);
        assert_eq!(live.iri, gold.iri.as_str());
        assert_eq!(&doc.source[live.span.as_range()], gold.surface.as_str());
    }

    let norms = normalize_dates_and_numbers(&doc.source);
    let live_dates: Vec<_> = norms
        .iter()
        .filter_map(|n| match n {
            Normalized::DateIso { span, yyyy_mm_dd } => Some((
                span.start_utf8,
                span.end_utf8,
                std::str::from_utf8(yyyy_mm_dd).unwrap().to_owned(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(live_dates.len(), doc.dates.len(), "{} date count", doc.doc_id);
    for (live, gold) in live_dates.iter().zip(&doc.dates) {
        assert_eq!(live.0, gold.start);
        assert_eq!(live.1, gold.end);
        assert_eq!(live.2, gold.yyyy_mm_dd);
    }

    let live_qty: Vec<_> = norms
        .iter()
        .filter_map(|n| match n {
            Normalized::Number {
                span,
                value,
                unit: Some(unit),
            } => Some((span.start_utf8, span.end_utf8, *value, *unit)),
            _ => None,
        })
        .collect();
    assert_eq!(
        live_qty.len(),
        doc.quantities.len(),
        "{} quantity count",
        doc.doc_id
    );
    for (live, gold) in live_qty.iter().zip(&doc.quantities) {
        assert_eq!(live.0, gold.start);
        assert_eq!(live.1, gold.end);
        assert!((live.2 - gold.value).abs() < 1e-9, "{} value", doc.doc_id);
        assert_eq!(live.3, gold.unit.as_str());
    }
}

fn check_corpus(corpus: &str) -> Vec<GoldDoc> {
    let mut docs = Vec::new();
    for path in gold_files(corpus) {
        let doc = load_gold_file(&path);
        assert_eq!(doc.corpus_id, corpus);
        assert_eq!(doc.language, "en");
        match corpus {
            "qualia-catchment-notes-v0" => {
                assert_eq!(doc.domain, "catchment-measurement-notes");
            }
            "qualia-english-notes-v0" => {
                assert_eq!(doc.domain, "english-general-notes");
            }
            other => panic!("unexpected corpus {other}"),
        }
        assert_gold_spans(&doc);
        assert_matches_live_kernels(&doc);
        let txt = datasets_root()
            .join(corpus)
            .join("documents")
            .join(format!("{}.txt", doc.doc_id));
        let disk = std::fs::read_to_string(&txt).unwrap_or_else(|e| panic!("read {}: {e}", txt.display()));
        assert_eq!(disk, doc.source, "{} source != documents txt", doc.doc_id);
        docs.push(doc);
    }
    docs
}

#[test]
fn catchment_gold_spans_match_source_and_kernels() {
    let docs = check_corpus("qualia-catchment-notes-v0");
    let fixture = docs
        .iter()
        .find(|d| d.doc_id == "north-spring-fixture-001")
        .expect("fixture");
    assert_eq!(fixture.split, "train");
    assert_eq!(fixture.sensitivity, "Restricted");
    assert_eq!(fixture.sentences.len(), 2);
    assert!(fixture
        .gazetteer
        .iter()
        .any(|h| h.surface == "North Spring"
            && h.iri == "https://qualiadb.org/catchment/NorthSpring"));
    assert!(fixture.quantities.iter().any(|q| q.unit == "mm" && (q.value - 12.5).abs() < 1e-9));
    assert!(fixture.dates.iter().any(|d| d.yyyy_mm_dd == "2026-08-15"));

    for doc in &docs {
        if doc.doc_id == "north-spring-fixture-001" {
            continue;
        }
        assert!(
            doc.gazetteer.is_empty(),
            "{} independent note must not gold a made-up site hit",
            doc.doc_id
        );
        assert!(
            !doc.source.contains("North Spring"),
            "{} must be an independent site",
            doc.doc_id
        );
        assert!(
            !doc.source.contains("12.5 mm"),
            "{} must use a different quantity/unit than the fixture",
            doc.doc_id
        );
    }

    let invalid = docs
        .iter()
        .find(|d| d.doc_id == "west-pond-invalid-date-001")
        .expect("invalid calendar date");
    assert!(invalid.source.contains("2026-02-31"));
    assert!(
        invalid.dates.is_empty(),
        "invalid Gregorian date must not be gold as ISO"
    );
}

#[test]
fn english_gold_spans_match_source_and_kernels() {
    let docs = check_corpus("qualia-english-notes-v0");
    for doc in &docs {
        assert_eq!(doc.sensitivity, "Public");
        assert!(doc.gazetteer.is_empty());
    }

    let decimal = docs
        .iter()
        .find(|d| d.doc_id == "notes-decimal-en-001")
        .expect("decimal");
    assert_eq!(decimal.sentences.len(), 3);
    assert_eq!(decimal.sentences[0].surface, "Recorded 12.5 mm.");
    assert!(decimal.source.contains("12.5 mm."));

    let abbrev = docs
        .iter()
        .find(|d| d.doc_id == "notes-abbrev-en-001")
        .expect("abbrev");
    assert_eq!(abbrev.sentences.len(), 2);
    assert_eq!(abbrev.sentences[0].surface, "Dr.");
    assert_eq!(abbrev.sentences[1].surface, "Smith arrived later.");

    let eg = docs
        .iter()
        .find(|d| d.doc_id == "notes-eg-en-001")
        .expect("e.g.");
    assert_eq!(eg.sentences.len(), 3);
    assert_eq!(eg.sentences[0].surface, "See e.");
    assert_eq!(eg.sentences[1].surface, "g.");

    let punct = docs
        .iter()
        .find(|d| d.doc_id == "notes-punct-en-001")
        .expect("ascii punct");
    assert_eq!(punct.sentences.len(), 2);
    assert_eq!(punct.sentences[0].surface, "Really?");
    assert_eq!(punct.sentences[1].surface, "Yes!");

    let newline = docs
        .iter()
        .find(|d| d.doc_id == "notes-newline-en-001")
        .expect("newline");
    assert_eq!(newline.sentences.len(), 2);
    assert_eq!(newline.sentences[0].surface, "Line one");

    let fullwidth_q = docs
        .iter()
        .find(|d| d.doc_id == "notes-fullwidth-q-en-001")
        .expect("fullwidth q");
    assert_eq!(fullwidth_q.sentences.len(), 1);
    assert_eq!(fullwidth_q.sentences[0].surface, "好吗？");

    let unicode = docs
        .iter()
        .find(|d| d.doc_id == "notes-unicode-en-001")
        .expect("unicode");
    assert_eq!(unicode.sentences[1].surface, "你好。");
    assert_eq!(unicode.sentences[2].surface, "世界！");
    assert!(
        unicode
            .sentences
            .iter()
            .any(|s| s.surface == "Wait… later."),
        "U+2026 must not split the Wait sentence"
    );
}
