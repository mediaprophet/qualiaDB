//! Provenance tracing and verification for Poet research.

use super::util::count_within;
use web_sys::{Document, Element};

pub(super) fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "research:trace-provenance" => Some(trace_provenance(container)),
        "research:verify-provenance" => Some(verify_provenance(container)),
        "research:detect-provenance-break" => Some(detect_provenance_break(container)),
        "research:compare-provenance" => Some(compare_provenance(container)),
        "research:export-provenance-report" => Some(export_provenance_report(container)),
        _ => None,
    }
}

fn trace_provenance(container: &Element) -> Result<(), String> {
    let corpus = count_within(container, "[data-corpus-item]")?;
    let inferences = count_within(container, "[data-research-inference]")?;
    let findings = count_within(container, "[data-research-finding]")?;
    container
        .set_attribute(
            "data-provenance-trace",
            &format!("corpus={corpus};inferences={inferences};findings={findings}"),
        )
        .map_err(|_| "Failed to trace provenance.".to_string())
}

fn verify_provenance(container: &Element) -> Result<(), String> {
    let corpus = count_within(container, "[data-corpus-item]")?;
    let verdict = if corpus > 0 {
        "provenance:complete_with_sources"
    } else {
        "provenance:incomplete_missing_sources"
    };
    container
        .set_attribute("data-provenance-verified", verdict)
        .map_err(|_| "Failed to verify provenance.".to_string())
}

fn detect_provenance_break(container: &Element) -> Result<(), String> {
    let inferences = count_within(container, "[data-research-inference]")?;
    let corpus = count_within(container, "[data-corpus-item]")?;
    let breaks = if inferences > corpus {
        "break:undocumented_derivation"
    } else {
        "break:none_detected"
    };
    container
        .set_attribute("data-provenance-breaks", breaks)
        .map_err(|_| "Failed to detect provenance breaks.".to_string())
}

fn compare_provenance(container: &Element) -> Result<(), String> {
    let items = count_within(container, "[data-corpus-item]")?;
    let comparison = if items >= 2 {
        "provenance_compare:shared_sources_detected"
    } else {
        "provenance_compare:insufficient_items"
    };
    container
        .set_attribute("data-provenance-comparison", comparison)
        .map_err(|_| "Failed to compare provenance.".to_string())
}

fn export_provenance_report(container: &Element) -> Result<(), String> {
    let trace = container
        .get_attribute("data-provenance-trace")
        .unwrap_or_else(|| "untraced".to_string());
    let verified = container
        .get_attribute("data-provenance-verified")
        .unwrap_or_else(|| "unverified".to_string());
    let report = format!("{{\"trace\":\"{trace}\",\"verified\":\"{verified}\"}}");
    container
        .set_attribute("data-provenance-export", &report)
        .map_err(|_| "Failed to export provenance report.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_actions_route_safely() {
        assert!(run(
            &web_sys::Document::from(wasm_bindgen::JsValue::NULL),
            &web_sys::Element::from(wasm_bindgen::JsValue::NULL),
            "research:unknown",
        )
        .is_none());
    }
}
