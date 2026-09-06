//! Corpus building, tagging, and extraction for Poet research.

use super::util::{append_csv_attr, append_nested, count_document, count_within, next_confidence};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "research:add-corpus-item" => Some(add_corpus_item(document, container)),
        "research:import-literature" => Some(import_literature(document, container)),
        "research:set-corpus-confidence" => Some(cycle_corpus_confidence(container)),
        "research:tag-corpus-item" => Some(tag_corpus_item(container)),
        "research:query-corpus" => Some(query_corpus(document, container)),
        "research:deduplicate-corpus" => Some(deduplicate_corpus(container)),
        "research:extract-from-corpus" => Some(extract_from_corpus(container)),
        "research:annotate-corpus-item" => Some(annotate_corpus_item(container)),
        _ => None,
    }
}

fn add_corpus_item(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-corpus-item",
        "source:primary_literature",
        &[("data-corpus-kind", "literature")],
    )?;
    append_csv_attr(container, "data-corpus-items", "source:primary_literature")
}

fn import_literature(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-corpus-item",
        "source:imported_literature",
        &[
            ("data-corpus-kind", "literature"),
            ("data-corpus-origin", "local_file"),
        ],
    )?;
    append_csv_attr(container, "data-corpus-items", "source:imported_literature")
}

fn cycle_corpus_confidence(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-corpus-confidence");
    let next = next_confidence(current.as_deref());
    container
        .set_attribute("data-corpus-confidence", next)
        .map_err(|_| "Failed to set corpus confidence.".to_string())
}

fn tag_corpus_item(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-corpus-tags", "tag:peer_reviewed")
}

fn query_corpus(document: &Document, container: &Element) -> Result<(), String> {
    let local = count_within(container, "[data-corpus-item]")?;
    let global = count_document(document, "[data-corpus-item]")?;
    container
        .set_attribute(
            "data-corpus-query",
            &format!("local={local};canvas={global}"),
        )
        .map_err(|_| "Failed to record corpus query.".to_string())
}

fn deduplicate_corpus(container: &Element) -> Result<(), String> {
    let count = count_within(container, "[data-corpus-item]")?;
    let duplicates = if count > 1 { count - 1 } else { 0 };
    container
        .set_attribute("data-corpus-duplicates", &duplicates.to_string())
        .map_err(|_| "Failed to record corpus duplicates.".to_string())
}

fn extract_from_corpus(container: &Element) -> Result<(), String> {
    append_csv_attr(
        container,
        "data-corpus-extractions",
        "extract:entities_relations_topics",
    )
}

fn annotate_corpus_item(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-corpus-annotated", "semantic_markup_applied")
        .map_err(|_| "Failed to annotate corpus item.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_actions_route_safely() {
        assert!(run(
            &web_sys::Document::from(wasm_bindgen::JsValue::NULL),
            &web_sys::Element::from(wasm_bindgen::JsValue::NULL),
            "research:import-dataset",
        )
        .is_none());
        assert!(run(
            &web_sys::Document::from(wasm_bindgen::JsValue::NULL),
            &web_sys::Element::from(wasm_bindgen::JsValue::NULL),
            "research:import-web",
        )
        .is_none());
    }
}
