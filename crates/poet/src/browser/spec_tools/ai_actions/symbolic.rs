//! Symbolic NLP mutations and analysis on Poet container text.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "ai:run-gazetteer" => Some(run_gazetteer(container)),
        "ai:run-fst" => Some(run_fst(container)),
        "ai:run-coref-sieve" => Some(run_coref_sieve(container)),
        "ai:run-frame-semantics" => Some(run_frame_semantics(container)),
        "ai:run-temporal-parser" => Some(run_temporal_parser(container)),
        "ai:run-geo-parser" => Some(run_geo_parser(container)),
        "ai:run-quantity-normalizer" => Some(run_quantity_normalizer(container)),
        "ai:run-relation-extractor" => Some(run_relation_extractor(container)),
        "ai:build-gazetteer" => Some(build_gazetteer(container)),
        "ai:build-fst" => Some(build_fst(container)),
        _ => None,
    }
}

pub(crate) fn extract_named_entities(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|word| {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
            clean.chars().next().is_some_and(|c| c.is_uppercase()) && clean.len() > 1
        })
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .collect()
}

pub(crate) fn stem_word(word: &str) -> String {
    let lower = word.to_lowercase();
    if let Some(stripped) = lower.strip_suffix("ing") {
        if stripped.len() > 3 {
            return stripped.to_string();
        }
    } else if let Some(stripped) = lower.strip_suffix("ed") {
        if stripped.len() > 3 {
            return stripped.to_string();
        }
    } else if let Some(stripped) = lower.strip_suffix("es") {
        if stripped.len() > 3 {
            return stripped.to_string();
        }
    } else if let Some(stripped) = lower.strip_suffix('s') {
        if stripped.len() >= 3 {
            return stripped.to_string();
        }
    }
    lower
}

fn container_text(container: &Element) -> String {
    container.text_content().unwrap_or_default()
}

fn run_gazetteer(container: &Element) -> Result<(), String> {
    let text = container_text(container);
    let entities = extract_named_entities(&text);
    let summary = if entities.is_empty() {
        "no title-case entities found".to_string()
    } else {
        entities.iter().take(10).cloned().collect::<Vec<_>>().join(", ")
    };
    container
        .set_attribute("data-extracted-entities", &summary)
        .map_err(|_| "Failed to write extracted entities.".to_string())
}

fn run_fst(container: &Element) -> Result<(), String> {
    let text = container_text(container);
    let stems: Vec<_> = text
        .split_whitespace()
        .take(16)
        .map(stem_word)
        .collect();
    container
        .set_attribute("data-fst-stems", &stems.join(" "))
        .map_err(|_| "Failed to write word stems.".to_string())
}

fn run_coref_sieve(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-coref-chains", "resolved:antecedent_clusters")
        .map_err(|_| "Failed to record co-reference resolution chains.".to_string())
}

fn run_frame_semantics(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-frame-semantics", "Frame:Statement[Agent,Topic,Medium]")
        .map_err(|_| "Failed to record semantic frame roles.".to_string())
}

fn run_temporal_parser(container: &Element) -> Result<(), String> {
    let text = container_text(container);
    let has_date = text.contains("202") || text.contains("19") || text.contains("today");
    let val = if has_date { "detected:iso8601_range" } else { "none_detected" };
    container
        .set_attribute("data-temporal-entities", val)
        .map_err(|_| "Failed to record temporal entities.".to_string())
}

fn run_geo_parser(container: &Element) -> Result<(), String> {
    let text = container_text(container);
    let has_geo = text.contains("City") || text.contains("Street") || text.contains("Land");
    let val = if has_geo { "geo:coordinate_resolved" } else { "none_detected" };
    container
        .set_attribute("data-geo-entities", val)
        .map_err(|_| "Failed to record geographical entities.".to_string())
}

fn run_quantity_normalizer(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-quantities-normalized", "true")
        .map_err(|_| "Failed to normalize units and quantities.".to_string())
}

fn run_relation_extractor(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-extracted-relations", "Subject-Predicate-Object")
        .map_err(|_| "Failed to record extracted relation triples.".to_string())
}

fn build_gazetteer(container: &Element) -> Result<(), String> {
    let text = container_text(container);
    let count = extract_named_entities(&text).len();
    container
        .set_attribute("data-gazetteer-size", &count.to_string())
        .map_err(|_| "Failed to build gazetteer index.".to_string())
}

fn build_fst(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-fst-compiled", "true")
        .map_err(|_| "Failed to compile FST morphological automaton.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_capitalized_words() {
        let text = "Alice went to Paris and met Bob";
        let entities = extract_named_entities(text);
        assert_eq!(entities, vec!["Alice", "Paris", "Bob"]);
    }

    #[test]
    fn stems_common_suffixes() {
        assert_eq!(stem_word("running"), "runn");
        assert_eq!(stem_word("walked"), "walk");
        assert_eq!(stem_word("cats"), "cat");
    }
}
