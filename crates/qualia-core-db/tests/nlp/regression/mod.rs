//! Plumbing smoke for public NLP entry points.
//!
//! Calls `tokenize`, `normalize_dates_and_numbers`, and `analyze_document`.
//! Asserts spans stay inside the source. Does **not** freeze decimal-split
//! tokenisation as golden output (NLP-002 is repairing that concurrently).

use qualia_core_db::nlp::analyze_document;
use qualia_core_db::nlp::normalize::{normalize_dates_and_numbers, Normalized};
use qualia_core_db::nlp::tokenize::tokenize;

use super::gate::release_gate_succeeds;
use super::manifest::{load_manifest, BenchmarkManifest, SMOKE_MANIFEST_JSON};

fn join_corpus(manifest: &BenchmarkManifest) -> String {
    manifest.corpus.join(" ")
}

fn span_inside_source(source: &str, start: u32, end: u32) -> bool {
    let s = start as usize;
    let e = end as usize;
    s <= e && e <= source.len() && source.is_char_boundary(s) && source.is_char_boundary(e)
}

/// Plumbing check only: functions exist; every emitted span is a source slice.
pub fn plumbing_ok(source: &str) -> Result<(), String> {
    if source.is_empty() {
        return Err("smoke corpus must not be empty".into());
    }

    let tokens = tokenize(source);
    if tokens.is_empty() {
        return Err("tokenize returned no tokens".into());
    }
    for tok in &tokens {
        if !span_inside_source(source, tok.span.start_utf8, tok.span.end_utf8) {
            return Err(format!(
                "token span {}..{} outside source (len {})",
                tok.span.start_utf8,
                tok.span.end_utf8,
                source.len()
            ));
        }
        let slice = tok
            .span
            .slice(source)
            .ok_or_else(|| format!("token span is not a UTF-8 slice: {:?}", tok.span))?;
        if slice != tok.text {
            return Err(format!(
                "token text {:?} does not match source slice {:?}",
                tok.text, slice
            ));
        }
    }

    let norms = normalize_dates_and_numbers(source);
    for n in &norms {
        let span = match n {
            Normalized::DateIso { span, .. } | Normalized::Number { span, .. } => *span,
        };
        if !span_inside_source(source, span.start_utf8, span.end_utf8) {
            return Err(format!(
                "normalize span {}..{} outside source",
                span.start_utf8, span.end_utf8
            ));
        }
        if span.slice(source).is_none() {
            return Err("normalize span is not a UTF-8 slice".into());
        }
    }

    let analysis = analyze_document(source);
    if analysis.token_count != tokens.len() {
        return Err(format!(
            "analyze_document token_count {} != tokenize len {}",
            analysis.token_count,
            tokens.len()
        ));
    }
    if analysis.source_hash == 0 {
        return Err("analyze_document produced a zero source_hash".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::gate::evaluate_release_gate;
    use crate::nlp::manifest::GateStatus;
    use crate::nlp::receipt::EvalReceipt;

    #[test]
    fn smoke_plumbing_calls_public_apis() {
        let manifest = load_manifest(SMOKE_MANIFEST_JSON).expect("smoke manifest");
        assert_eq!(manifest.task, "tokenize");
        assert_eq!(manifest.language, "en");
        assert_eq!(manifest.resource_envelope.network, "forbidden");
        assert!(!manifest.resource_envelope.download_datasets);
        assert!(!manifest.resource_envelope.download_models);

        let source = join_corpus(&manifest);
        assert!(source.contains("North Spring is the reference site."));
        plumbing_ok(&source).expect("plumbing");

        // Plumbing may be green while the release gate stays UNEVALUATED.
        assert!(!release_gate_succeeds(&manifest));
        let gate = evaluate_release_gate(&manifest);
        assert_eq!(gate.effective_status, GateStatus::Unevaluated);
    }

    #[test]
    fn smoke_receipt_records_plumbing_not_quality() {
        let manifest = load_manifest(SMOKE_MANIFEST_JSON).unwrap();
        let source = join_corpus(&manifest);
        let mut passed = 0u32;
        let mut failed = 0u32;
        match plumbing_ok(&source) {
            Ok(()) => passed += 1,
            Err(_) => failed += 1,
        }
        if !release_gate_succeeds(&manifest) {
            passed += 1;
        } else {
            failed += 1;
        }
        let receipt = EvalReceipt::scaffold(
            passed,
            failed,
            vec!["benchmarks/nlp/manifests/nlp-004-smoke-tokenize-en.json".into()],
        );
        assert_eq!(receipt.failed, 0);
        assert_eq!(receipt.passed, 2);
        assert_eq!(receipt.source_revision, "uncommitted");
        let json = receipt.to_json();
        let md = receipt.to_markdown();
        assert!(json.contains("\"suite_id\": \"nlp-004\""));
        assert!(md.contains("uncommitted"));
    }
}
