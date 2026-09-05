//! Faceted search panel, chip toggling, and live-canvas matching.

use super::catalog::{
    CONTAINER_TYPES, ENTITY_TYPES, EPISTEMIC_MODALITIES, HONESTY_LEVELS, ONTOLOGY_PREFIXES, STRATA,
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

pub(super) fn build_faceted_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("search-mode-panel");
    panel.set_attribute("data-mode", "faceted").unwrap();

    // Facet groups
    let facets_row = document.create_element("div").unwrap();
    let fr_el: HtmlElement = facets_row.clone().dyn_into().unwrap();
    fr_el
        .style()
        .set_css_text("display: grid; grid-template-columns: 1fr 1fr; gap: 12px;");

    // Ontology prefix facet
    let prefix_group = build_facet_group(
        document,
        "ontology-prefix",
        "Ontology Prefix",
        ONTOLOGY_PREFIXES,
        true,
    );
    facets_row.append_child(&prefix_group).unwrap();

    // Entity type facet
    let entity_group =
        build_facet_group(document, "entity-type", "Entity Type", ENTITY_TYPES, true);
    facets_row.append_child(&entity_group).unwrap();

    // Epistemic modality facet
    let epi_group = build_facet_group(
        document,
        "epistemic",
        "Epistemic Modality",
        EPISTEMIC_MODALITIES,
        false,
    );
    facets_row.append_child(&epi_group).unwrap();

    // Strata facet
    let strata_group = build_facet_group(document, "strata", "Strata", STRATA, true);
    facets_row.append_child(&strata_group).unwrap();

    // Honesty facet
    let honesty_group = build_facet_group(document, "honesty", "Honesty", HONESTY_LEVELS, true);
    facets_row.append_child(&honesty_group).unwrap();

    // Container type facet
    let container_group = build_facet_group(
        document,
        "container-type",
        "Container Type",
        CONTAINER_TYPES,
        true,
    );
    facets_row.append_child(&container_group).unwrap();

    panel.append_child(&facets_row).unwrap();

    // Search button + results
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");

    let search_btn = document.create_element("button").unwrap();
    search_btn.set_id("faceted-search-btn");
    search_btn.set_text_content(Some("\u{1F50D} Search"));
    let sb_el: HtmlElement = search_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 8px 20px; background: var(--accent-cyan); color: var(--bg-deep); \
         border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; font-weight: 700; cursor: pointer;",
    );
    actions.append_child(&search_btn).unwrap();

    let save_btn = document.create_element("button").unwrap();
    save_btn.set_id("faceted-save-btn");
    save_btn.set_text_content(Some("\u{1F4BE} Save Query"));
    let svb_el: HtmlElement = save_btn.clone().dyn_into().unwrap();
    svb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--surface-panel); color: var(--text-secondary); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
    );
    actions.append_child(&save_btn).unwrap();

    let count_label = document.create_element("span").unwrap();
    count_label.set_id("faceted-result-count");
    let cl_el: HtmlElement = count_label.clone().dyn_into().unwrap();
    cl_el.style().set_css_text("font-size: 10px; color: var(--text-muted); margin-left: auto; font-family: var(--font-mono);");
    count_label.set_text_content(Some(""));
    actions.append_child(&count_label).unwrap();

    panel.append_child(&actions).unwrap();

    // Results area
    let results = document.create_element("div").unwrap();
    results.set_id("faceted-results");
    let r_el: HtmlElement = results.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; min-height: 100px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary);",
    );
    results.set_text_content(Some("Select facets and click Search to see results."));
    panel.append_child(&results).unwrap();

    panel
}

fn build_facet_group(
    document: &Document,
    facet_id: &str,
    label: &str,
    options: &[(&str, &str)],
    multi_select: bool,
) -> Element {
    let group = document.create_element("div").unwrap();
    let g_el: HtmlElement = group.clone().dyn_into().unwrap();
    g_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 4px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         padding: 8px; border: 1px solid var(--border-subtle);",
    );

    let lbl = document.create_element("div").unwrap();
    let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
    l_el.style().set_css_text("font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.3px;");
    lbl.set_text_content(Some(label));
    group.append_child(&lbl).unwrap();

    let options_el = document.create_element("div").unwrap();
    let o_el: HtmlElement = options_el.clone().dyn_into().unwrap();
    o_el.style().set_css_text(
        "display: flex; flex-wrap: wrap; gap: 4px; max-height: 120px; overflow-y: auto;",
    );

    for (key, display) in options {
        let chip = document.create_element("button").unwrap();
        chip.set_class_name("facet-chip");
        chip.set_attribute("data-facet", facet_id).unwrap();
        chip.set_attribute("data-value", key).unwrap();
        chip.set_attribute("data-multi", if multi_select { "true" } else { "false" })
            .unwrap();
        let c_el: HtmlElement = chip.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "padding: 3px 8px; font-size: 10px; font-family: var(--font-mono); \
             background: var(--surface-panel-elevated); border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); color: var(--text-muted); cursor: pointer; \
             transition: var(--trans-fast);",
        );
        chip.set_text_content(Some(display));
        options_el.append_child(&chip).unwrap();
    }
    group.append_child(&options_el).unwrap();
    group
}

pub(super) fn wire_faceted_search(document: &Document) {
    // Wire facet chip toggling
    let chips = document.query_selector_all(".facet-chip").unwrap();
    for i in 0..chips.length() {
        let chip = chips.get(i).unwrap();
        let chip_el: Element = chip.dyn_into().unwrap();
        let chip_clone = chip_el.clone();

        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let is_active = chip_clone.class_list().contains("active");
            let multi = chip_clone.get_attribute("data-multi").unwrap_or_default() == "true";
            let facet_id = chip_clone.get_attribute("data-facet").unwrap_or_default();

            if !multi {
                // Deselect all chips in the same facet group
                let doc = web_sys::window().unwrap().document().unwrap();
                let all = doc
                    .query_selector_all(&format!(".facet-chip[data-facet=\"{}\"]", facet_id))
                    .unwrap();
                for j in 0..all.length() {
                    let c = all.get(j).unwrap();
                    let c_el: Element = c.dyn_into().unwrap();
                    c_el.class_list().remove_1("active").unwrap();
                    let c_html: HtmlElement = c_el.clone().dyn_into().unwrap();
                    c_html
                        .style()
                        .set_property("background", "var(--surface-panel-elevated)")
                        .unwrap();
                    c_html
                        .style()
                        .set_property("color", "var(--text-muted)")
                        .unwrap();
                    c_html
                        .style()
                        .set_property("border-color", "var(--border-subtle)")
                        .unwrap();
                }
            }

            if is_active && multi {
                chip_clone.class_list().remove_1("active").unwrap();
                let html: HtmlElement = chip_clone.clone().dyn_into().unwrap();
                html.style()
                    .set_property("background", "var(--surface-panel-elevated)")
                    .unwrap();
                html.style()
                    .set_property("color", "var(--text-muted)")
                    .unwrap();
                html.style()
                    .set_property("border-color", "var(--border-subtle)")
                    .unwrap();
            } else {
                chip_clone.class_list().add_1("active").unwrap();
                let html: HtmlElement = chip_clone.clone().dyn_into().unwrap();
                html.style()
                    .set_property("background", "rgba(0,255,170,0.12)")
                    .unwrap();
                html.style()
                    .set_property("color", "var(--accent-cyan)")
                    .unwrap();
                html.style()
                    .set_property("border-color", "var(--accent-cyan)")
                    .unwrap();
            }
        }) as Box<dyn FnMut(MouseEvent)>);

        chip_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire search button
    let search_btn = match document.get_element_by_id("faceted-search-btn") {
        Some(b) => b,
        None => return,
    };
    let sb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        run_faceted_search(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    search_btn
        .add_event_listener_with_callback("click", sb_closure.as_ref().unchecked_ref())
        .unwrap();
    sb_closure.forget();

    // Wire save button
    let save_btn = match document.get_element_by_id("faceted-save-btn") {
        Some(b) => b,
        None => return,
    };
    let svb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        super::saved::save_current_query(&doc, "faceted");
    }) as Box<dyn FnMut(MouseEvent)>);
    save_btn
        .add_event_listener_with_callback("click", svb_closure.as_ref().unchecked_ref())
        .unwrap();
    svb_closure.forget();
}

pub(super) fn get_active_facets(document: &Document) -> Vec<(String, Vec<String>)> {
    let chips = document.query_selector_all(".facet-chip.active").unwrap();
    let mut facet_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for i in 0..chips.length() {
        let chip = chips.get(i).unwrap();
        let c_el: Element = chip.dyn_into().unwrap();
        let facet = c_el.get_attribute("data-facet").unwrap_or_default();
        let value = c_el.get_attribute("data-value").unwrap_or_default();
        facet_map.entry(facet).or_default().push(value);
    }
    facet_map.into_iter().collect()
}

fn run_faceted_search(document: &Document) {
    let facets = get_active_facets(document);
    let results_el = match document.get_element_by_id("faceted-results") {
        Some(r) => r,
        None => return,
    };
    let count_el = match document.get_element_by_id("faceted-result-count") {
        Some(c) => c,
        None => return,
    };

    results_el.set_inner_html("");

    if facets.is_empty() {
        results_el.set_text_content(Some("Select at least one facet to search."));
        count_el.set_text_content(Some(""));
        return;
    }

    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    let mut matched = Vec::new();
    for index in 0..containers.length() {
        let Some(node) = containers.get(index) else {
            continue;
        };
        let Ok(container) = node.dyn_into::<Element>() else {
            continue;
        };
        if container_matches_facets(&container, &facets) {
            matched.push(container);
        }
    }

    if matched.is_empty() {
        results_el.set_text_content(Some(
            "No live canvas containers match every selected facet.",
        ));
    } else {
        for (index, container) in matched.iter().enumerate() {
            let row = document.create_element("div").unwrap();
            row.set_attribute(
                "style",
                "padding: 6px 8px; border-bottom: 1px solid var(--border-subtle); display: flex; align-items: center; gap: 8px;",
            )
            .unwrap();
            let title = container
                .query_selector(".container-title")
                .ok()
                .flatten()
                .and_then(|title| title.text_content())
                .unwrap_or_else(|| "Untitled".into());
            let container_type = container
                .get_attribute("data-container-type")
                .unwrap_or_else(|| "unknown".into());
            row.set_text_content(Some(&format!(
                "#{:03}  {}  ·  {}",
                index + 1,
                title,
                container_type
            )));
            results_el.append_child(&row).unwrap();
        }
    }
    count_el.set_text_content(Some(&format!("{} live canvas result(s)", matched.len())));
}

fn container_matches_facets(container: &Element, facets: &[(String, Vec<String>)]) -> bool {
    facets.iter().all(|(facet, values)| {
        let actual = match facet.as_str() {
            "container-type" => container
                .get_attribute("data-container-type")
                .unwrap_or_default(),
            "epistemic" => container
                .get_attribute("data-epistemic")
                .unwrap_or_default(),
            "strata" => container.get_attribute("data-strata").unwrap_or_default(),
            "honesty" => container
                .query_selector(".honesty-badge")
                .ok()
                .flatten()
                .and_then(|badge| badge.text_content())
                .unwrap_or_default(),
            "ontology-prefix" => format!(
                "{} {}",
                container
                    .get_attribute("data-semantic-type")
                    .unwrap_or_default(),
                container
                    .get_attribute("data-semantic-uri")
                    .unwrap_or_default()
            ),
            "entity-type" => container
                .get_attribute("data-semantic-type")
                .unwrap_or_default(),
            _ => String::new(),
        }
        .to_ascii_lowercase();
        values.iter().any(|value| {
            let value = value.to_ascii_lowercase();
            actual == value
                || actual.starts_with(&format!("{value}:"))
                || actual.contains(&format!("/{value}"))
                || actual.contains(&format!("#{value}"))
        })
    })
}
