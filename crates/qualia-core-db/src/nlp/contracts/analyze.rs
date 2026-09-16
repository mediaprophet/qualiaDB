//! Year-one pipeline behind the caller-buffered contract.

use super::error::{AnalysisState, BufferChannel, NlpContractError};
use super::views::{
    slice_source, DocumentSummary, DocumentViewBuffers, HitView, NormView, PlanView, SentenceView,
    TokenView, NORM_INDEX_NONE, SENTENCE_ID_NONE,
};
use super::ANNOTATION_CONTRACT_VERSION;
use crate::nlp::budget::{reject_source, NlpBudgetError};
use crate::nlp::emit::AnnotationPlan;
use crate::nlp::gazetteer::{Gazetteer, Hit};
use crate::nlp::hash::hash60;
use crate::nlp::link;
use crate::nlp::normalize::{normalize_dates_and_numbers, Normalized};
use crate::nlp::span::DocSpan;
use crate::nlp::tokenize::{split_sentences, tokenize, Sentence, Token};
use crate::nlp::DocumentAnalysis;

/// Slot counts a caller must supply for [`analyze_document_into`] on `source`.
///
/// This runs the year-one pipeline once. Prefer calling it, allocating those
/// lengths, then [`analyze_document_into`] — or use [`analyze_document`], which
/// sizes from one pipeline run. Overflow is a typed error; buffers are not
/// written when any channel is short. After `OutputBufferFull`, caller slices
/// retain their prior contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentCapacities {
    pub tokens: usize,
    pub sentences: usize,
    pub hits: usize,
    pub norms: usize,
    pub plans: usize,
}

/// Caller-buffered year-one analysis. Source cap is [`crate::nlp::budget::MAX_SOURCE_BYTES`].
///
/// Required capacities: [`required_capacities`]. Retry: allocate at least those
/// lengths and call again; this function does not resize. On error, `buffers`
/// are left unchanged (all `require_cap` checks run before any write).
pub fn analyze_document_into(
    source: &str,
    buffers: &mut DocumentViewBuffers<'_>,
) -> Result<DocumentSummary, NlpContractError> {
    reject_source(source.len()).map_err(source_error)?;
    write_views(source, &year_one_layers(source), buffers)
}

/// Capacities for a successful [`analyze_document_into`] of `source`.
pub fn required_capacities(source: &str) -> Result<DocumentCapacities, NlpContractError> {
    reject_source(source.len()).map_err(source_error)?;
    Ok(capacities_from_layers(&year_one_layers(source)))
}

/// Cold `Vec` adapter. Same year-one pipeline as [`analyze_document_into`].
///
/// Signature stays `-> DocumentAnalysis` for existing hosts. The pipeline runs
/// **once**; buffers are sized from those layer lengths (no probe-by-error
/// loop). Hosts and HTTP adapters must still call
/// [`crate::nlp::budget::reject_source`] (or the 64 KiB Poet payload cap)
/// before this entry — this adapter does not change the public error type.
pub fn analyze_document(source: &str) -> DocumentAnalysis {
    let layers = year_one_layers(source);
    let caps = capacities_from_layers(&layers);
    let mut tokens = vec![TokenView::EMPTY; caps.tokens];
    let mut sentences = vec![SentenceView::EMPTY; caps.sentences];
    let mut hits = vec![HitView::EMPTY; caps.hits];
    let mut norms = vec![NormView::EMPTY; caps.norms];
    let mut plans = vec![PlanView::EMPTY; caps.plans];
    let mut buffers = DocumentViewBuffers {
        tokens: &mut tokens,
        sentences: &mut sentences,
        hits: &mut hits,
        norms: &mut norms,
        plans: &mut plans,
    };
    match write_views(source, &layers, &mut buffers) {
        Ok(summary) => document_analysis_from_views(source, &hits, &norms, &plans, summary),
        // Cold adapter cannot return Result. Span/capacity failures fall back
        // to the owned emit path rather than panic.
        Err(NlpContractError::SpanInvalid { .. } | NlpContractError::OutputBufferFull { .. }) => {
            document_analysis_from_layers(source, &layers)
        }
        Err(NlpContractError::SourceTooLarge { .. }) => {
            document_analysis_from_layers(source, &layers)
        }
    }
}

struct YearOneLayers<'a> {
    tokens: Vec<Token<'a>>,
    sentences: Vec<Sentence>,
    hits: Vec<Hit>,
    norms: Vec<Normalized>,
}

fn year_one_layers(source: &str) -> YearOneLayers<'_> {
    YearOneLayers {
        tokens: tokenize(source),
        sentences: split_sentences(source),
        hits: link::filter_known(Gazetteer::default().find(source)),
        norms: normalize_dates_and_numbers(source),
    }
}

fn capacities_from_layers(layers: &YearOneLayers<'_>) -> DocumentCapacities {
    DocumentCapacities {
        tokens: layers.tokens.len(),
        sentences: layers.sentences.len(),
        hits: layers.hits.len(),
        norms: layers.norms.len(),
        plans: layers
            .hits
            .len()
            .checked_add(layers.norms.len())
            .unwrap_or(0),
    }
}

fn source_error(err: NlpBudgetError) -> NlpContractError {
    match err {
        NlpBudgetError::SourceTooLarge { bytes, max } => {
            NlpContractError::SourceTooLarge { bytes, max }
        }
        other => unreachable!("reject_source only yields SourceTooLarge, got {other:?}"),
    }
}

fn require_cap(
    needed: usize,
    capacity: usize,
    channel: BufferChannel,
) -> Result<(), NlpContractError> {
    if needed > capacity {
        Err(NlpContractError::OutputBufferFull {
            needed,
            capacity,
            channel,
        })
    } else {
        Ok(())
    }
}

fn write_views(
    source: &str,
    layers: &YearOneLayers<'_>,
    buffers: &mut DocumentViewBuffers<'_>,
) -> Result<DocumentSummary, NlpContractError> {
    let plan_needed = layers.hits.len().checked_add(layers.norms.len()).ok_or(
        NlpContractError::OutputBufferFull {
            needed: buffers.plans.len().saturating_add(1),
            capacity: buffers.plans.len(),
            channel: BufferChannel::Plans,
        },
    )?;
    require_cap(
        layers.tokens.len(),
        buffers.tokens.len(),
        BufferChannel::Tokens,
    )?;
    require_cap(
        layers.sentences.len(),
        buffers.sentences.len(),
        BufferChannel::Sentences,
    )?;
    require_cap(layers.hits.len(), buffers.hits.len(), BufferChannel::Hits)?;
    require_cap(
        layers.norms.len(),
        buffers.norms.len(),
        BufferChannel::Norms,
    )?;
    require_cap(plan_needed, buffers.plans.len(), BufferChannel::Plans)?;

    for token in &layers.tokens {
        let slice = slice_source(token.span, source)?;
        if slice != token.text {
            return Err(NlpContractError::SpanInvalid {
                start: token.span.start_utf8,
                end: token.span.end_utf8,
            });
        }
    }
    for sentence in &layers.sentences {
        slice_source(sentence.span, source)?;
    }
    for hit in &layers.hits {
        slice_source(hit.span, source)?;
    }
    for norm in &layers.norms {
        slice_source(norm_span(norm), source)?;
    }

    let source_hash = hash60(source.as_bytes());
    for (i, token) in layers.tokens.iter().enumerate() {
        buffers.tokens[i] = TokenView {
            span: token.span,
            kind: token.kind,
            sentence_id: sentence_id_for(token.span, &layers.sentences),
            norm_index: norm_index_for(token.span, &layers.norms),
        };
    }
    for (i, sentence) in layers.sentences.iter().enumerate() {
        buffers.sentences[i] = SentenceView {
            span: sentence.span,
        };
    }
    for (i, hit) in layers.hits.iter().enumerate() {
        buffers.hits[i] = HitView {
            span: hit.span,
            iri: hit.iri,
            surface: hit.surface,
        };
    }
    for (i, norm) in layers.norms.iter().enumerate() {
        buffers.norms[i] = norm_view(norm);
    }
    let mut plan_i = 0usize;
    for hit in &layers.hits {
        buffers.plans[plan_i] = plan_from_hit(source, hit, source_hash)?;
        plan_i += 1;
    }
    for norm in &layers.norms {
        buffers.plans[plan_i] = plan_from_norm(source, norm, source_hash)?;
        plan_i += 1;
    }

    Ok(DocumentSummary {
        contract_version: ANNOTATION_CONTRACT_VERSION,
        source_hash,
        token_count: layers.tokens.len(),
        sentence_count: layers.sentences.len(),
        hit_count: layers.hits.len(),
        norm_count: layers.norms.len(),
        plan_count: plan_i,
        state: AnalysisState::Complete,
    })
}

fn sentence_id_for(span: DocSpan, sentences: &[Sentence]) -> u32 {
    for (i, sentence) in sentences.iter().enumerate() {
        if span.start_utf8 >= sentence.span.start_utf8 && span.end_utf8 <= sentence.span.end_utf8 {
            return i as u32;
        }
    }
    SENTENCE_ID_NONE
}

fn norm_index_for(span: DocSpan, norms: &[Normalized]) -> u32 {
    for (i, norm) in norms.iter().enumerate() {
        let nspan = norm_span(norm);
        if span.start_utf8 < nspan.end_utf8 && nspan.start_utf8 < span.end_utf8 {
            return i as u32;
        }
    }
    NORM_INDEX_NONE
}

fn norm_span(norm: &Normalized) -> DocSpan {
    match *norm {
        Normalized::DateIso { span, .. } | Normalized::Number { span, .. } => span,
    }
}

fn norm_view(norm: &Normalized) -> NormView {
    match *norm {
        Normalized::DateIso { span, yyyy_mm_dd } => NormView::DateIso { span, yyyy_mm_dd },
        Normalized::Number { span, value, unit } => NormView::Number { span, value, unit },
    }
}

fn plan_from_hit(source: &str, hit: &Hit, source_hash: u64) -> Result<PlanView, NlpContractError> {
    let slice = slice_source(hit.span, source)?;
    Ok(PlanView {
        span: hit.span,
        content_hash: hash60(slice.as_bytes()),
        source_hash,
        term_iri: hit.iri,
        kind: "gazetteer",
    })
}

fn plan_from_norm(
    source: &str,
    norm: &Normalized,
    source_hash: u64,
) -> Result<PlanView, NlpContractError> {
    let (span, term_iri, kind) = match *norm {
        Normalized::DateIso { span, .. } => (span, "https://qualiadb.org/datatype/isoDate", "date"),
        Normalized::Number { span, unit, .. } => (
            span,
            if unit.is_some() {
                "https://qualiadb.org/datatype/quantity"
            } else {
                "https://qualiadb.org/datatype/number"
            },
            "number",
        ),
    };
    let slice = slice_source(span, source)?;
    Ok(PlanView {
        span,
        content_hash: hash60(slice.as_bytes()),
        source_hash,
        term_iri,
        kind,
    })
}

fn document_analysis_from_views(
    source: &str,
    hits: &[HitView],
    norms: &[NormView],
    plans: &[PlanView],
    summary: DocumentSummary,
) -> DocumentAnalysis {
    DocumentAnalysis {
        source_hash: summary.source_hash,
        token_count: summary.token_count,
        sentence_count: summary.sentence_count,
        hits: hits[..summary.hit_count]
            .iter()
            .map(|hit| Hit {
                span: hit.span,
                iri: hit.iri,
                surface: hit.surface,
            })
            .collect(),
        norms: norms[..summary.norm_count]
            .iter()
            .map(normalized_from_view)
            .collect(),
        plans: plans[..summary.plan_count]
            .iter()
            .map(|plan| plan_to_owned(source, *plan))
            .collect(),
    }
}

fn normalized_from_view(view: &NormView) -> Normalized {
    match *view {
        NormView::DateIso { span, yyyy_mm_dd } => Normalized::DateIso { span, yyyy_mm_dd },
        NormView::Number { span, value, unit } => Normalized::Number { span, value, unit },
    }
}

fn plan_to_owned(source: &str, plan: PlanView) -> AnnotationPlan {
    let surface = plan.span.slice(source).unwrap_or("").to_string();
    AnnotationPlan {
        term_iri: plan.term_iri.to_string(),
        start_utf8: plan.span.start_utf8,
        end_utf8: plan.span.end_utf8,
        content_hash: plan.content_hash,
        source_hash: plan.source_hash,
        surface,
        kind: plan.kind,
    }
}

fn document_analysis_from_layers(source: &str, layers: &YearOneLayers<'_>) -> DocumentAnalysis {
    let mut plans = crate::nlp::emit::emit_from_hits(source, &layers.hits);
    plans.extend(crate::nlp::emit::emit_from_normalized(source, &layers.norms));
    DocumentAnalysis {
        source_hash: hash60(source.as_bytes()),
        token_count: layers.tokens.len(),
        sentence_count: layers.sentences.len(),
        hits: layers.hits.clone(),
        norms: layers.norms.clone(),
        plans,
    }
}
