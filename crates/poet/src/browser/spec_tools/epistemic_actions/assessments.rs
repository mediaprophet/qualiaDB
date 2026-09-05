//! Epistemic assessment mutations on Poet container surfaces.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "epistemic:create-assessment" => Some(create_assessment(container)),
        "epistemic:set-epistemic-mode" => Some(cycle_mode(container)),
        "epistemic:set-reality-category" => Some(cycle_reality_category(container)),
        "epistemic:mark-disputed" => Some(toggle_disputed(container)),
        "epistemic:query-assessments" => Some(query_assessments(document, container)),
        "epistemic:compare-assessments" => Some(compare_assessments(document, container)),
        "epistemic:assess-recursive" => Some(assess_recursive(container)),
        "epistemic:export-assessment" => Some(export_assessment(container)),
        "epistemic:link-to-investigation" => Some(link_to_investigation(container)),
        "epistemic:link-to-research" => Some(link_to_research(container)),
        _ => None,
    }
}

pub(crate) fn next_epistemic_mode(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("objective") => "intersubjective",
        Some("intersubjective") => "subjective",
        Some("subjective") => "normative",
        Some("normative") => "contested",
        Some("contested") => "hypothetical",
        _ => "objective",
    }
}

pub(crate) fn next_reality_category(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("nonfiction") => "fiction",
        Some("fiction") => "blend",
        Some("blend") => "simulation",
        _ => "nonfiction",
    }
}

fn create_assessment(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-epistemic-assessment", "active")
        .map_err(|_| "Failed to create epistemic assessment on surface.".to_string())?;
    let _ = container.set_attribute("data-epistemic-mode", "objective");
    let _ = container.set_attribute("data-reality-category", "nonfiction");
    Ok(())
}

fn cycle_mode(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-epistemic-mode");
    let next = next_epistemic_mode(current.as_deref());
    container
        .set_attribute("data-epistemic-mode", next)
        .map_err(|_| "Failed to update epistemic mode.".to_string())?;
    let _ = container.set_attribute("data-epistemic-assessment", "active");
    Ok(())
}

fn cycle_reality_category(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-reality-category");
    let next = next_reality_category(current.as_deref());
    container
        .set_attribute("data-reality-category", next)
        .map_err(|_| "Failed to update reality category.".to_string())?;
    let _ = container.set_attribute("data-epistemic-assessment", "active");
    Ok(())
}

fn toggle_disputed(container: &Element) -> Result<(), String> {
    let is_disputed = container
        .get_attribute("data-epistemic-disputed")
        .is_some_and(|v| v == "true");
    if is_disputed {
        let _ = container.remove_attribute("data-epistemic-disputed");
        let _ = container.remove_attribute("data-dispute-reason");
    } else {
        container
            .set_attribute("data-epistemic-disputed", "true")
            .map_err(|_| "Failed to mark claim as disputed.".to_string())?;
        let _ = container.set_attribute("data-dispute-reason", "observer disagreement noted");
    }
    Ok(())
}

fn query_assessments(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-epistemic-assessment]")
        .map_err(|_| "Failed to query epistemic assessments.".to_string())?;
    let count = list.length();
    container
        .set_attribute("data-assessment-count", &count.to_string())
        .map_err(|_| "Failed to record assessment count.".to_string())?;
    Ok(())
}

fn compare_assessments(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all(".canvas-container-node[data-epistemic-assessment]")
        .map_err(|_| "Failed to query comparative assessments.".to_string())?;
    let comparison = if list.length() >= 2 {
        "multi-perspective comparison active"
    } else {
        "single assessment active on canvas"
    };
    container
        .set_attribute("data-epistemic-comparison", comparison)
        .map_err(|_| "Failed to set comparison attribute.".to_string())?;
    Ok(())
}

fn assess_recursive(container: &Element) -> Result<(), String> {
    let current_depth: u32 = container
        .get_attribute("data-epistemic-depth")
        .and_then(|d| d.parse().ok())
        .unwrap_or(0);
    let next_depth = current_depth.saturating_add(1);
    container
        .set_attribute("data-epistemic-depth", &next_depth.to_string())
        .map_err(|_| "Failed to set recursive assessment depth.".to_string())?;
    let _ = container.set_attribute("data-epistemic-meta-assessed", "true");
    Ok(())
}

fn export_assessment(container: &Element) -> Result<(), String> {
    let mode = container
        .get_attribute("data-epistemic-mode")
        .unwrap_or_else(|| "unspecified".to_string());
    let reality = container
        .get_attribute("data-reality-category")
        .unwrap_or_else(|| "nonfiction".to_string());
    let disputed = container
        .get_attribute("data-epistemic-disputed")
        .unwrap_or_else(|| "false".to_string());
    let payload = format!(
        "{{\"mode\":\"{}\",\"reality\":\"{}\",\"disputed\":{}}}",
        mode, reality, disputed
    );
    container
        .set_attribute("data-epistemic-export", &payload)
        .map_err(|_| "Failed to export assessment payload.".to_string())?;
    Ok(())
}

fn link_to_investigation(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-linked-case", "case:active")
        .map_err(|_| "Failed to link assessment to investigation.".to_string())
}

fn link_to_research(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-linked-research", "research:active")
        .map_err(|_| "Failed to link assessment to research finding.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_transitions_cycle_completely() {
        assert_eq!(next_epistemic_mode(None), "objective");
        assert_eq!(next_epistemic_mode(Some("objective")), "intersubjective");
        assert_eq!(next_epistemic_mode(Some("intersubjective")), "subjective");
        assert_eq!(next_epistemic_mode(Some("subjective")), "normative");
        assert_eq!(next_epistemic_mode(Some("normative")), "contested");
        assert_eq!(next_epistemic_mode(Some("contested")), "hypothetical");
        assert_eq!(next_epistemic_mode(Some("hypothetical")), "objective");
    }

    #[test]
    fn reality_categories_cycle_cleanly() {
        assert_eq!(next_reality_category(None), "nonfiction");
        assert_eq!(next_reality_category(Some("nonfiction")), "fiction");
        assert_eq!(next_reality_category(Some("fiction")), "blend");
        assert_eq!(next_reality_category(Some("blend")), "simulation");
        assert_eq!(next_reality_category(Some("simulation")), "nonfiction");
    }
}
