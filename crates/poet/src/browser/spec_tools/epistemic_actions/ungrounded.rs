//! Ungrounded generation instance recording and diagnosis microformats.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "epistemic:create-ug-instance" => Some(create_ug_instance(container)),
        "epistemic:set-ug-cause" => Some(set_ug_cause(container)),
        "epistemic:set-ug-consequence" => Some(set_ug_consequence(container)),
        "epistemic:set-ug-detection" => Some(set_ug_detection(container)),
        "epistemic:set-ug-mitigation" => Some(set_ug_mitigation(container)),
        "epistemic:set-ug-calibration" => Some(set_ug_calibration(container)),
        "epistemic:query-ug-instances" => Some(query_ug_instances(document, container)),
        "epistemic:query-ug-causes" => Some(query_ug_causes(container)),
        "epistemic:query-ug-consequences" => Some(query_ug_consequences(container)),
        "epistemic:compare-ug-instances" => Some(compare_ug_instances(document, container)),
        "epistemic:query-cause-consequence-matrix" => Some(query_cause_consequence_matrix(container)),
        _ => None,
    }
}

pub(crate) fn next_ug_cause(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("training-data-gap") => "context-window-overflow",
        Some("context-window-overflow") => "attention-misallocation",
        Some("attention-misallocation") => "sampling-artefact",
        Some("sampling-artefact") => "retrieval-failure",
        Some("retrieval-failure") => "alignment-tax",
        Some("alignment-tax") => "sycophancy",
        _ => "training-data-gap",
    }
}

pub(crate) fn next_ug_consequence(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("enumerated-false-fact") => "enumerated-false-citation",
        Some("enumerated-false-citation") => "enumerated-false-entity",
        Some("enumerated-false-entity") => "context-drift",
        Some("context-drift") => "confident-ungrounded-output",
        Some("confident-ungrounded-output") => "plausible-ungrounded-output",
        _ => "enumerated-false-fact",
    }
}

fn create_ug_instance(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-ug-instance", "active")
        .map_err(|_| "Failed to create ungrounded generation instance.".to_string())?;
    let _ = container.set_attribute("data-ug-model", "model:unspecified");
    let _ = container.set_attribute("data-ug-sampling", "temperature:0.7;top_p:0.9");
    let _ = container.set_attribute("data-ug-retrieval-used", "false");
    Ok(())
}

fn set_ug_cause(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-ug-cause-primary");
    let cause = next_ug_cause(current.as_deref());
    container
        .set_attribute("data-ug-cause-primary", cause)
        .map_err(|_| "Failed to set ungrounded generation cause.".to_string())?;
    append_csv(container, "data-ug-causes", cause)
}

fn set_ug_consequence(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-ug-consequence-primary");
    let consequence = next_ug_consequence(current.as_deref());
    container
        .set_attribute("data-ug-consequence-primary", consequence)
        .map_err(|_| "Failed to set ungrounded generation consequence.".to_string())?;
    append_csv(container, "data-ug-consequences", consequence)
}

fn set_ug_detection(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-ug-detection", "grounding-verifier;detected_by:did:qualia:observer")
        .map_err(|_| "Failed to record ungrounded generation detection.".to_string())
}

fn set_ug_mitigation(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-ug-mitigation", "retrieval-improvement;temperature-reduction")
        .map_err(|_| "Failed to record ungrounded generation mitigation.".to_string())?;
    let _ = container.set_attribute("data-ug-mitigation-applied", "recommended");
    Ok(())
}

fn set_ug_calibration(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-ug-calibration", "0.35")
        .map_err(|_| "Failed to set confidence calibration gap.".to_string())
}

fn query_ug_instances(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-ug-instance]")
        .map_err(|_| "Failed to query ungrounded generation instances.".to_string())?;
    container
        .set_attribute("data-ug-instance-count", &list.length().to_string())
        .map_err(|_| "Failed to record ungrounded instance count.".to_string())
}

fn query_ug_causes(container: &Element) -> Result<(), String> {
    let causes = container
        .get_attribute("data-ug-causes")
        .unwrap_or_else(|| "none recorded".to_string());
    container
        .set_attribute("data-ug-causes-summary", &causes)
        .map_err(|_| "Failed to summarize ungrounded causes.".to_string())
}

fn query_ug_consequences(container: &Element) -> Result<(), String> {
    let consequences = container
        .get_attribute("data-ug-consequences")
        .unwrap_or_else(|| "none recorded".to_string());
    container
        .set_attribute("data-ug-consequences-summary", &consequences)
        .map_err(|_| "Failed to summarize ungrounded consequences.".to_string())
}

fn compare_ug_instances(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-ug-instance]")
        .map_err(|_| "Failed to query ungrounded instances for comparison.".to_string())?;
    let comparison = if list.length() >= 2 {
        "multi-instance ungrounded output comparison active"
    } else {
        "single ungrounded instance on canvas"
    };
    container
        .set_attribute("data-ug-comparison", comparison)
        .map_err(|_| "Failed to set ungrounded instance comparison.".to_string())
}

fn query_cause_consequence_matrix(container: &Element) -> Result<(), String> {
    let cause = container
        .get_attribute("data-ug-cause-primary")
        .unwrap_or_else(|| "training-data-gap".to_string());
    let consequence = container
        .get_attribute("data-ug-consequence-primary")
        .unwrap_or_else(|| "enumerated-false-fact".to_string());
    let matrix = format!(
        "{{\"cause\":\"{cause}\",\"consequence\":\"{consequence}\",\"mitigation\":\"retrieval-improvement\"}}"
    );
    container
        .set_attribute("data-ug-cause-consequence-matrix", &matrix)
        .map_err(|_| "Failed to record cause-consequence matrix.".to_string())
}

fn append_csv(container: &Element, attr: &str, item: &str) -> Result<(), String> {
    let current = container.get_attribute(attr).unwrap_or_default();
    let updated = if current.is_empty() {
        item.to_string()
    } else if current.contains(item) {
        current
    } else {
        format!("{current},{item}")
    };
    container
        .set_attribute(attr, &updated)
        .map_err(|_| format!("Failed to update {attr}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ug_causes_cycle() {
        assert_eq!(next_ug_cause(None), "training-data-gap");
        assert_eq!(next_ug_cause(Some("sycophancy")), "training-data-gap");
    }

    #[test]
    fn ug_consequences_cycle() {
        assert_eq!(next_ug_consequence(None), "enumerated-false-fact");
        assert_eq!(
            next_ug_consequence(Some("plausible-ungrounded-output")),
            "enumerated-false-fact"
        );
    }
}
