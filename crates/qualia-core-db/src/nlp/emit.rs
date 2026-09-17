//! Span-annotation plans. Host seals Quins via `text_span::annotation_quin`.

use super::gazetteer::Hit;
use super::hash::hash60;
use super::normalize::Normalized;
use super::span::DocSpan;

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationPlan {
    pub term_iri: String,
    pub start_utf8: u32,
    pub end_utf8: u32,
    pub content_hash: u64,
    pub source_hash: u64,
    pub surface: String,
    pub kind: &'static str,
}

pub fn emit_from_hits(source: &str, hits: &[Hit]) -> Vec<AnnotationPlan> {
    let source_hash = hash60(source.as_bytes());
    hits.iter()
        .filter_map(|h| {
            let slice = h.span.slice(source)?;
            Some(AnnotationPlan {
                term_iri: h.iri.to_string(),
                start_utf8: h.span.start_utf8,
                end_utf8: h.span.end_utf8,
                content_hash: hash60(slice.as_bytes()),
                source_hash,
                surface: slice.to_string(),
                kind: "gazetteer",
            })
        })
        .collect()
}

pub fn emit_from_normalized(source: &str, norms: &[Normalized]) -> Vec<AnnotationPlan> {
    let source_hash = hash60(source.as_bytes());
    norms
        .iter()
        .filter_map(|n| {
            let (span, iri, kind) = match n {
                Normalized::DateIso { span, .. } => {
                    (*span, "https://qualiadb.org/datatype/isoDate", "date")
                }
                Normalized::Number { span, unit, .. } => (
                    *span,
                    if unit.is_some() {
                        "https://qualiadb.org/datatype/quantity"
                    } else {
                        "https://qualiadb.org/datatype/number"
                    },
                    "number",
                ),
            };
            let slice = span.slice(source)?;
            Some(AnnotationPlan {
                term_iri: iri.to_string(),
                start_utf8: span.start_utf8,
                end_utf8: span.end_utf8,
                content_hash: hash60(slice.as_bytes()),
                source_hash,
                surface: slice.to_string(),
                kind,
            })
        })
        .collect()
}

pub fn span_of_plan(plan: &AnnotationPlan) -> DocSpan {
    DocSpan::new(plan.start_utf8, plan.end_utf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::gazetteer::Gazetteer;
    use crate::nlp::link::filter_known;
    use crate::nlp::normalize::normalize_dates_and_numbers;

    #[test]
    fn plans_cover_lived_snippet() {
        let src = "North Spring is the reference catchment. Timothy Charles Holborn recorded 12.5 mm of rain on 2026-08-15.";
        let hits = filter_known(Gazetteer::default().find(src));
        let mut plans = emit_from_hits(src, &hits);
        plans.extend(emit_from_normalized(src, &normalize_dates_and_numbers(src)));
        assert!(plans.iter().any(|p| p.term_iri.ends_with("NorthSpring")));
        assert!(plans
            .iter()
            .any(|p| p.term_iri.contains("timothy_charles_holborn")));
        assert!(plans.iter().any(|p| p.kind == "date"));
        assert!(plans.iter().any(|p| p.kind == "number"));
    }
}
