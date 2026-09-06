//! Fiction / non-fiction classification and detection microformats.

use super::assessments::next_reality_category;
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "epistemic:classify-reality" => Some(classify_reality(container)),
        "epistemic:query-classifications" => Some(query_classifications(document, container)),
        "epistemic:detect-deceptive-fiction" => Some(detect_deceptive_fiction(container)),
        "epistemic:detect-blended-content" => Some(detect_blended_content(container)),
        "epistemic:compare-classifications" => Some(compare_classifications(document, container)),
        "epistemic:trace-fiction-to-reality" => Some(trace_fiction_to_reality(container)),
        _ => None,
    }
}

fn classify_reality(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-fiction-classification");
    let next = next_reality_category(current.as_deref());
    container
        .set_attribute("data-fiction-classification", next)
        .map_err(|_| "Failed to classify reality category.".to_string())?;
    let _ = container.set_attribute("data-reality-category", next);
    let _ = container.set_attribute("data-fiction-confidence", "0.80");
    Ok(())
}

fn query_classifications(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-fiction-classification], [data-reality-category]")
        .map_err(|_| "Failed to query reality classifications.".to_string())?;
    container
        .set_attribute("data-fiction-classification-count", &list.length().to_string())
        .map_err(|_| "Failed to record classification count.".to_string())
}

fn detect_deceptive_fiction(container: &Element) -> Result<(), String> {
    let category = container
        .get_attribute("data-fiction-classification")
        .or_else(|| container.get_attribute("data-reality-category"));
    let deceptive = category
        .as_deref()
        .is_some_and(|c| c == "fiction" || c == "blend");
    let mode = container.get_attribute("data-epistemic-mode");
    let presented_as_fact = mode.as_deref().is_some_and(|m| m == "objective" || m == "intersubjective");
    let flagged = deceptive && presented_as_fact;
    container
        .set_attribute(
            "data-deceptive-fiction",
            if flagged { "detected" } else { "none" },
        )
        .map_err(|_| "Failed to record deceptive fiction detection.".to_string())
}

fn detect_blended_content(container: &Element) -> Result<(), String> {
    let category = container
        .get_attribute("data-fiction-classification")
        .or_else(|| container.get_attribute("data-reality-category"));
    let blended = category.as_deref().is_some_and(|c| c == "blend");
    container
        .set_attribute(
            "data-blended-content",
            if blended { "detected" } else { "none" },
        )
        .map_err(|_| "Failed to record blended content detection.".to_string())
}

fn compare_classifications(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-fiction-classification]")
        .map_err(|_| "Failed to query classifications for comparison.".to_string())?;
    let comparison = if list.length() >= 2 {
        "multi-agent classification comparison active"
    } else {
        "single classification on canvas"
    };
    container
        .set_attribute("data-fiction-comparison", comparison)
        .map_err(|_| "Failed to set classification comparison.".to_string())
}

fn trace_fiction_to_reality(container: &Element) -> Result<(), String> {
    let category = container
        .get_attribute("data-fiction-classification")
        .or_else(|| container.get_attribute("data-reality-category"))
        .unwrap_or_else(|| "nonfiction".to_string());
    let trace = match category.as_str() {
        "fiction" | "blend" => "allegorical:satirical:historical-fiction references mapped",
        _ => "non-fiction target; no fiction-to-reality trace required",
    };
    container
        .set_attribute("data-fiction-reality-trace", trace)
        .map_err(|_| "Failed to record fiction-to-reality trace.".to_string())
}
