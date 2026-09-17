//! Sentiment assessment, dimension scoring, and detection microformats.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "epistemic:assess-sentiment" => Some(assess_sentiment(container)),
        "epistemic:set-sentiment-dimension" => Some(set_sentiment_dimension(container)),
        "epistemic:link-sentiment-to-reality" => Some(link_sentiment_to_reality(container)),
        "epistemic:query-sentiment" => Some(query_sentiment(document, container)),
        "epistemic:analyse-sentiment-trends" => Some(analyse_sentiment_trends(container)),
        "epistemic:detect-sentiment-manipulation" => Some(detect_sentiment_manipulation(container)),
        "epistemic:detect-performed-sentiment" => Some(detect_performed_sentiment(container)),
        "epistemic:map-sentiment-network" => Some(map_sentiment_network(document, container)),
        "epistemic:compare-sentiment" => Some(compare_sentiment(document, container)),
        "epistemic:detect-sentiment-reality-mismatch" => {
            Some(detect_sentiment_reality_mismatch(container))
        }
        _ => None,
    }
}

pub(crate) fn next_sentiment_type(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("positive") => "negative",
        Some("negative") => "mixed",
        Some("mixed") => "neutral",
        _ => "positive",
    }
}

pub(crate) fn next_sentiment_dimension(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("valence") => "arousal",
        Some("arousal") => "dominance",
        Some("dominance") => "certainty",
        Some("certainty") => "moral",
        Some("moral") => "aesthetic",
        Some("aesthetic") => "irony",
        Some("irony") => "sarcasm",
        _ => "valence",
    }
}

fn assess_sentiment(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-sentiment-type");
    let next = next_sentiment_type(current.as_deref());
    container
        .set_attribute("data-sentiment-type", next)
        .map_err(|_| "Failed to set sentiment type.".to_string())?;
    let _ = container.set_attribute("data-sentiment-intensity", "moderate");
    let _ = container.set_attribute("data-sentiment-score", "0.65");
    let _ = container.set_attribute("data-sentiment-authenticity", "genuine");
    Ok(())
}

fn set_sentiment_dimension(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-sentiment-dimension-active");
    let dimension = next_sentiment_dimension(current.as_deref());
    container
        .set_attribute("data-sentiment-dimension-active", dimension)
        .map_err(|_| "Failed to set active sentiment dimension.".to_string())?;
    append_dimension_score(container, dimension, "0.70")
}

fn append_dimension_score(container: &Element, dimension: &str, score: &str) -> Result<(), String> {
    let current = container
        .get_attribute("data-sentiment-dimensions")
        .unwrap_or_default();
    let entry = format!("{dimension}:{score}");
    let updated = if current.is_empty() {
        entry
    } else if current.contains(dimension) {
        current
    } else {
        format!("{current};{entry}")
    };
    container
        .set_attribute("data-sentiment-dimensions", &updated)
        .map_err(|_| "Failed to update sentiment dimension scores.".to_string())
}

fn link_sentiment_to_reality(container: &Element) -> Result<(), String> {
    let category = container
        .get_attribute("data-reality-category")
        .or_else(|| container.get_attribute("data-fiction-classification"))
        .unwrap_or_else(|| "nonfiction".to_string());
    container
        .set_attribute("data-sentiment-reality-link", &category)
        .map_err(|_| "Failed to link sentiment to reality category.".to_string())
}

fn query_sentiment(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-sentiment-type]")
        .map_err(|_| "Failed to query sentiment records.".to_string())?;
    container
        .set_attribute("data-sentiment-count", &list.length().to_string())
        .map_err(|_| "Failed to record sentiment count.".to_string())
}

fn analyse_sentiment_trends(container: &Element) -> Result<(), String> {
    let score = container
        .get_attribute("data-sentiment-score")
        .unwrap_or_else(|| "0.50".to_string());
    let trend = format!("sentiment trajectory sampled; latest score={score}");
    container
        .set_attribute("data-sentiment-trends", &trend)
        .map_err(|_| "Failed to record sentiment trend analysis.".to_string())
}

fn detect_sentiment_manipulation(container: &Element) -> Result<(), String> {
    let authenticity = container
        .get_attribute("data-sentiment-authenticity")
        .unwrap_or_default();
    let flagged = authenticity == "manipulated" || authenticity == "simulated";
    container
        .set_attribute(
            "data-sentiment-manipulation",
            if flagged { "detected" } else { "none" },
        )
        .map_err(|_| "Failed to record sentiment manipulation detection.".to_string())
}

fn detect_performed_sentiment(container: &Element) -> Result<(), String> {
    let authenticity = container
        .get_attribute("data-sentiment-authenticity")
        .unwrap_or_default();
    let performed = authenticity == "performed" || authenticity == "simulated";
    container
        .set_attribute(
            "data-sentiment-performed",
            if performed { "detected" } else { "none" },
        )
        .map_err(|_| "Failed to record performed sentiment detection.".to_string())
}

fn map_sentiment_network(document: &Document, container: &Element) -> Result<(), String> {
    let agents = document
        .query_selector_all("[data-perspective-dids]")
        .map_err(|_| "Failed to query agents for sentiment network.".to_string())?;
    let map = format!(
        "sentiment influence graph: {} observer nodes linked",
        agents.length()
    );
    container
        .set_attribute("data-sentiment-network", &map)
        .map_err(|_| "Failed to record sentiment network map.".to_string())
}

fn compare_sentiment(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-sentiment-type]")
        .map_err(|_| "Failed to query sentiment records for comparison.".to_string())?;
    let comparison = if list.length() >= 2 {
        "multi-agent sentiment comparison active"
    } else {
        "single sentiment note on canvas"
    };
    container
        .set_attribute("data-sentiment-comparison", comparison)
        .map_err(|_| "Failed to set sentiment comparison.".to_string())
}

fn detect_sentiment_reality_mismatch(container: &Element) -> Result<(), String> {
    let intensity = container
        .get_attribute("data-sentiment-intensity")
        .unwrap_or_default();
    let category = container
        .get_attribute("data-reality-category")
        .or_else(|| container.get_attribute("data-fiction-classification"))
        .unwrap_or_else(|| "nonfiction".to_string());
    let high_intensity = intensity == "extreme" || intensity == "strong";
    let fictional = category == "fiction" || category == "simulation";
    let mismatch = high_intensity && fictional;
    container
        .set_attribute(
            "data-sentiment-reality-mismatch",
            if mismatch { "detected" } else { "none" },
        )
        .map_err(|_| "Failed to record sentiment-reality mismatch detection.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentiment_types_cycle() {
        assert_eq!(next_sentiment_type(None), "positive");
        assert_eq!(next_sentiment_type(Some("neutral")), "positive");
    }

    #[test]
    fn sentiment_dimensions_cycle() {
        assert_eq!(next_sentiment_dimension(None), "valence");
        assert_eq!(next_sentiment_dimension(Some("sarcasm")), "valence");
    }

    #[test]
    fn reality_link_uses_category() {
        assert_eq!(
            super::super::assessments::next_reality_category(Some("fiction")),
            "blend"
        );
    }
}
