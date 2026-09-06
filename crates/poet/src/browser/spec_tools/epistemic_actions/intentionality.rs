//! Intentionality assessment and mistake classification microformats.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "epistemic:assess-intentionality" => Some(assess_intentionality(container)),
        "epistemic:classify-mistake" => Some(classify_mistake(container)),
        "epistemic:query-intentionality" => Some(query_intentionality(document, container)),
        "epistemic:query-mistakes" => Some(query_mistakes(document, container)),
        "epistemic:detect-mistake-patterns" => Some(detect_mistake_patterns(container)),
        "epistemic:compare-intentionality" => Some(compare_intentionality(document, container)),
        _ => None,
    }
}

pub(crate) fn next_intentionality_type(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("deliberate") => "intentional",
        Some("intentional") => "negligent",
        Some("negligent") => "reckless",
        Some("reckless") => "accidental",
        Some("accidental") => "mistaken",
        Some("mistaken") => "systematic",
        Some("systematic") => "emergent",
        Some("emergent") => "coerced",
        Some("coerced") => "habitual",
        _ => "deliberate",
    }
}

pub(crate) fn next_mistake_type(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("false-belief") => "skill-deficit",
        Some("skill-deficit") => "attention-failure",
        Some("attention-failure") => "perception-error",
        Some("perception-error") => "judgement-error",
        Some("judgement-error") => "system-design",
        Some("system-design") => "deliberate-deviation",
        Some("deliberate-deviation") => "ungrounded-generation",
        Some("ungrounded-generation") => "simulation-artefact",
        _ => "false-belief",
    }
}

fn assess_intentionality(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-intentionality-type");
    let next = next_intentionality_type(current.as_deref());
    container
        .set_attribute("data-intentionality-type", next)
        .map_err(|_| "Failed to set intentionality type.".to_string())?;
    let _ = container.set_attribute("data-intentionality-confidence", "0.75");
    Ok(())
}

fn classify_mistake(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-mistake-type");
    let next = next_mistake_type(current.as_deref());
    container
        .set_attribute("data-mistake-type", next)
        .map_err(|_| "Failed to classify mistake type.".to_string())?;
    let _ = container.set_attribute("data-mistake-severity", "moderate");
    let _ = container.set_attribute("data-mistake-correctable", "true");
    Ok(())
}

fn query_intentionality(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-intentionality-type]")
        .map_err(|_| "Failed to query intentionality assessments.".to_string())?;
    container
        .set_attribute("data-intentionality-count", &list.length().to_string())
        .map_err(|_| "Failed to record intentionality count.".to_string())
}

fn query_mistakes(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-mistake-type]")
        .map_err(|_| "Failed to query mistake classifications.".to_string())?;
    container
        .set_attribute("data-mistake-count", &list.length().to_string())
        .map_err(|_| "Failed to record mistake count.".to_string())
}

fn detect_mistake_patterns(container: &Element) -> Result<(), String> {
    let mistake = container
        .get_attribute("data-mistake-type")
        .unwrap_or_else(|| "none".to_string());
    let pattern = if mistake == "none" {
        "no recurring mistake pattern detected"
    } else {
        "recurring pattern flagged for review"
    };
    container
        .set_attribute("data-mistake-patterns", pattern)
        .map_err(|_| "Failed to record mistake pattern detection.".to_string())
}

fn compare_intentionality(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-intentionality-type]")
        .map_err(|_| "Failed to query intentionality notes for comparison.".to_string())?;
    let comparison = if list.length() >= 2 {
        "multi-agent intentionality comparison active"
    } else {
        "single intentionality note on canvas"
    };
    container
        .set_attribute("data-intentionality-comparison", comparison)
        .map_err(|_| "Failed to set intentionality comparison.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intentionality_types_cycle() {
        assert_eq!(next_intentionality_type(None), "deliberate");
        assert_eq!(next_intentionality_type(Some("habitual")), "deliberate");
    }

    #[test]
    fn mistake_types_cycle() {
        assert_eq!(next_mistake_type(None), "false-belief");
        assert_eq!(
            next_mistake_type(Some("simulation-artefact")),
            "false-belief"
        );
    }
}
