use super::*;
use crate::nlp::budget::MAX_SOURCE_BYTES;
use crate::nlp::span::DocSpan;

const CATCHMENT: &str = "North Spring is the reference site. Timothy Charles Holborn recorded 12.5 mm of rain on 2026-08-15.";

fn stack_buffers<'a>(
    tokens: &'a mut [TokenView],
    sentences: &'a mut [SentenceView],
    hits: &'a mut [HitView],
    norms: &'a mut [NormView],
    plans: &'a mut [PlanView],
) -> DocumentViewBuffers<'a> {
    DocumentViewBuffers {
        tokens,
        sentences,
        hits,
        norms,
        plans,
    }
}

#[test]
fn annotation_contract_version_is_one() {
    assert_eq!(ANNOTATION_CONTRACT_VERSION, 1);
}

#[test]
fn empty_source_is_complete_zero_counts() {
    let mut tokens = [TokenView::EMPTY; 0];
    let mut sentences = [SentenceView::EMPTY; 0];
    let mut hits = [HitView::EMPTY; 0];
    let mut norms = [NormView::EMPTY; 0];
    let mut plans = [PlanView::EMPTY; 0];
    let mut buffers = stack_buffers(
        &mut tokens,
        &mut sentences,
        &mut hits,
        &mut norms,
        &mut plans,
    );
    let summary = analyze_document_into("", &mut buffers).expect("empty source is success");
    assert_eq!(summary.contract_version, ANNOTATION_CONTRACT_VERSION);
    assert_eq!(summary.state, AnalysisState::Complete);
    assert_eq!(summary.token_count, 0);
    assert_eq!(summary.sentence_count, 0);
    assert_eq!(summary.hit_count, 0);
    assert_eq!(summary.norm_count, 0);
    assert_eq!(summary.plan_count, 0);
}

#[test]
fn catchment_paragraph_fills_buffers_and_slices_source() {
    let mut tokens = [TokenView::EMPTY; 64];
    let mut sentences = [SentenceView::EMPTY; 8];
    let mut hits = [HitView::EMPTY; 16];
    let mut norms = [NormView::EMPTY; 16];
    let mut plans = [PlanView::EMPTY; 32];
    let mut buffers = stack_buffers(
        &mut tokens,
        &mut sentences,
        &mut hits,
        &mut norms,
        &mut plans,
    );
    let summary =
        analyze_document_into(CATCHMENT, &mut buffers).expect("catchment fits 64/8/16/16/32");
    assert_eq!(summary.contract_version, ANNOTATION_CONTRACT_VERSION);
    assert_eq!(summary.state, AnalysisState::Complete);
    assert!(summary.token_count >= 10);
    assert!(summary.sentence_count >= 2);
    assert!(summary.plan_count >= 4);
    assert_ne!(summary.source_hash, 0);

    for view in &tokens[..summary.token_count] {
        let slice = view.text(CATCHMENT).expect("token span");
        assert_eq!(&CATCHMENT[view.span.as_range()], slice);
    }
    for view in &sentences[..summary.sentence_count] {
        let slice = view.text(CATCHMENT).expect("sentence span");
        assert_eq!(&CATCHMENT[view.span.as_range()], slice);
    }
    for view in &hits[..summary.hit_count] {
        assert!(view.text(CATCHMENT).is_ok());
    }
    for view in &norms[..summary.norm_count] {
        assert!(view.text(CATCHMENT).is_ok());
    }
    for view in &plans[..summary.plan_count] {
        assert!(view.text(CATCHMENT).is_ok());
    }
    assert!(tokens[..summary.token_count]
        .iter()
        .any(|t| t.text(CATCHMENT) == Ok("North")));
    assert!(plans[..summary.plan_count].iter().any(|p| p.kind == "date"));
    assert!(plans[..summary.plan_count]
        .iter()
        .any(|p| p.kind == "number"));
}

#[test]
fn buffer_too_small_is_error_not_empty_success() {
    let mut tokens = [TokenView::EMPTY; 1];
    let mut sentences = [SentenceView::EMPTY; 32];
    let mut hits = [HitView::EMPTY; 32];
    let mut norms = [NormView::EMPTY; 32];
    let mut plans = [PlanView::EMPTY; 32];
    let mut buffers = stack_buffers(
        &mut tokens,
        &mut sentences,
        &mut hits,
        &mut norms,
        &mut plans,
    );
    match analyze_document_into(CATCHMENT, &mut buffers) {
        Err(NlpContractError::OutputBufferFull {
            needed,
            capacity,
            channel,
        }) => {
            assert_eq!(channel, BufferChannel::Tokens);
            assert_eq!(capacity, 1);
            assert!(needed > 1);
        }
        Ok(summary) => panic!("overflow must not succeed: {summary:?}"),
        Err(other) => panic!("expected OutputBufferFull, got {other:?}"),
    }
}

#[test]
fn source_over_default_cap_is_error() {
    let oversized = "a".repeat(MAX_SOURCE_BYTES + 1);
    let mut tokens = [TokenView::EMPTY; 4];
    let mut sentences = [SentenceView::EMPTY; 4];
    let mut hits = [HitView::EMPTY; 4];
    let mut norms = [NormView::EMPTY; 4];
    let mut plans = [PlanView::EMPTY; 4];
    let mut buffers = stack_buffers(
        &mut tokens,
        &mut sentences,
        &mut hits,
        &mut norms,
        &mut plans,
    );
    match analyze_document_into(&oversized, &mut buffers) {
        Err(NlpContractError::SourceTooLarge { bytes, max }) => {
            assert_eq!(bytes, MAX_SOURCE_BYTES + 1);
            assert_eq!(max, MAX_SOURCE_BYTES);
        }
        Ok(_) => panic!("oversized source must not succeed"),
        Err(other) => panic!("expected SourceTooLarge, got {other:?}"),
    }
}

#[test]
fn invalid_span_is_error() {
    let src = "North Spring";
    let bad = DocSpan::new(0, 100);
    match slice_source(bad, src) {
        Err(NlpContractError::SpanInvalid { start, end }) => {
            assert_eq!(start, 0);
            assert_eq!(end, 100);
        }
        Ok(slice) => panic!("invalid span must not slice: {slice:?}"),
        Err(other) => panic!("expected SpanInvalid, got {other:?}"),
    }
}

#[test]
fn cold_adapter_matches_buffered_catchment() {
    let analysis = analyze_document(CATCHMENT);
    let caps = required_capacities(CATCHMENT).expect("caps");
    let mut tokens = vec![TokenView::EMPTY; caps.tokens];
    let mut sentences = vec![SentenceView::EMPTY; caps.sentences];
    let mut hits = vec![HitView::EMPTY; caps.hits];
    let mut norms = vec![NormView::EMPTY; caps.norms];
    let mut plans = vec![PlanView::EMPTY; caps.plans];
    let mut buffers = stack_buffers(
        &mut tokens,
        &mut sentences,
        &mut hits,
        &mut norms,
        &mut plans,
    );
    let summary = analyze_document_into(CATCHMENT, &mut buffers).expect("buffered");
    assert_eq!(analysis.token_count, summary.token_count);
    assert_eq!(analysis.sentence_count, summary.sentence_count);
    assert_eq!(analysis.hits.len(), summary.hit_count);
    assert_eq!(analysis.norms.len(), summary.norm_count);
    assert_eq!(analysis.plans.len(), summary.plan_count);
    assert_eq!(analysis.source_hash, summary.source_hash);
    for (owned, view) in analysis.hits.iter().zip(&hits[..summary.hit_count]) {
        assert_eq!(owned.span, view.span);
        assert_eq!(owned.iri, view.iri);
    }
}

#[test]
fn sentences_buffer_too_small_is_error() {
    let caps = required_capacities(CATCHMENT).expect("caps");
    let mut tokens = vec![TokenView::EMPTY; caps.tokens];
    let mut sentences = [SentenceView::EMPTY; 0];
    let mut hits = vec![HitView::EMPTY; caps.hits];
    let mut norms = vec![NormView::EMPTY; caps.norms];
    let mut plans = vec![PlanView::EMPTY; caps.plans];
    let mut buffers = stack_buffers(
        &mut tokens,
        &mut sentences,
        &mut hits,
        &mut norms,
        &mut plans,
    );
    match analyze_document_into(CATCHMENT, &mut buffers) {
        Err(NlpContractError::OutputBufferFull { channel, .. }) => {
            assert_eq!(channel, BufferChannel::Sentences);
        }
        other => panic!("expected sentences overflow, got {other:?}"),
    }
}

#[test]
fn overflow_leaves_caller_buffer_untouched() {
    let sentinel = TokenView {
        span: DocSpan::new(9, 10),
        kind: crate::nlp::tokenize::TokenKind::Word,
        sentence_id: 7,
        norm_index: 7,
    };
    let mut tokens = [sentinel; 1];
    let mut sentences = [SentenceView::EMPTY; 32];
    let mut hits = [HitView::EMPTY; 32];
    let mut norms = [NormView::EMPTY; 32];
    let mut plans = [PlanView::EMPTY; 32];
    let mut buffers = stack_buffers(
        &mut tokens,
        &mut sentences,
        &mut hits,
        &mut norms,
        &mut plans,
    );
    assert!(analyze_document_into(CATCHMENT, &mut buffers).is_err());
    assert_eq!(tokens[0], sentinel);
}

#[test]
fn v1_emits_only_complete() {
    let summary = analyze_document_into(
        "",
        &mut stack_buffers(
            &mut [],
            &mut [],
            &mut [],
            &mut [],
            &mut [],
        ),
    )
    .expect("empty");
    assert_eq!(summary.state, AnalysisState::Complete);
}
