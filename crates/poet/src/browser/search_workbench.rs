//! Search workbench — a modal overlay with three search modes:
//!
//! 1. **Faceted Search** — filter by ontology prefix, entity type,
//!    epistemic modality, strata, honesty, and container type. Shows
//!    result count and mock result list.
//! 2. **Visual Query Builder** — build SPARQL queries by adding
//!    triple patterns (subject, predicate, object) via UI controls.
//!    Generates a SPARQL SELECT query that can be inspected, edited,
//!    or saved.
//! 3. **Manual SPARQL** — write or edit a SPARQL query directly in a
//!    textarea. Supports loading saved queries, editing them, and
//!    saving new ones.
//!
//! Saved queries are persisted in localStorage as named objects with
//! metadata (name, mode, query text, facets, timestamp). Saved queries
//! can be used as container content sources — placing a "query-results"
//! container that displays the query and mock results.
//!
//! All search results are structural mocks — actual SPARQL execution
//! requires the QualiaDB daemon backend.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement, KeyboardEvent,
    MouseEvent,
};

// ---------------------------------------------------------------------------
// Ontology prefix catalog for faceted search and query builder
// ---------------------------------------------------------------------------

const ONTOLOGY_PREFIXES: &[(&str, &str)] = &[
    ("ont", "Ontology (ont:)"),
    ("hm", "Hypermedia (hm:)"),
    ("doc", "Document (doc:)"),
    ("prov", "Provenance (prov:)"),
    ("agency", "Agency (agency:)"),
    ("inv", "Investigation (inv:)"),
    ("set", "Settings (set:)"),
    ("soc", "Social (soc:)"),
    ("comm", "Communications (comm:)"),
    ("rights", "Rights (rights:)"),
    ("coop", "Cooperation (coop:)"),
    ("vibe", "VibeScript (vibe:)"),
    ("sanctuary", "Sanctuary (sanctuary:)"),
    ("epi", "Epistemics (epi:)"),
    ("selfhood", "Selfhood (selfhood:)"),
    ("care", "Care (care:)"),
    ("values", "Values (values:)"),
];

const ENTITY_TYPES: &[(&str, &str)] = &[
    ("term", "Term"),
    ("entity", "Named Entity"),
    ("claimedFact", "Claimed Fact"),
    ("statement", "Statement"),
    ("statistic", "Statistic"),
    ("citation", "Citation"),
    ("definition", "Definition"),
    ("quote", "Quote"),
];

const EPISTEMIC_MODALITIES: &[(&str, &str)] = &[
    ("objective", "Objective"),
    ("subjective", "Subjective"),
    ("intersubjective", "Intersubjective"),
    ("normative", "Normative"),
];

const STRATA: &[(&str, &str)] = &[
    ("environmental", "Environmental"),
    ("social", "Social"),
    ("legal", "Legal"),
    ("financial", "Financial"),
    ("technical", "Technical"),
];

const HONESTY_LEVELS: &[(&str, &str)] = &[
    ("live", "Live"),
    ("present", "Present"),
    ("partial", "Partial"),
    ("missing", "Missing"),
];

const CONTAINER_TYPES: &[(&str, &str)] = &[
    ("doc", "Document"),
    ("sheet", "Sheet"),
    ("code", "Code"),
    ("map", "Map"),
    ("ontology", "Ontology"),
    ("social", "Social"),
    ("graph", "Graph"),
    ("media", "Media"),
    ("3d", "3D"),
    ("webrtc", "WebRTC"),
    ("webview", "WebView"),
    ("vision", "Vision"),
    ("listen", "Listen"),
    ("triad", "Triad"),
    ("library", "Library"),
    ("latex", "LaTeX"),
    ("slide", "Slides"),
    ("finance", "Finance"),
    ("wallet", "Wallet"),
    ("rights", "Rights"),
    ("pulse", "Pulse"),
    ("aura", "Aura"),
];

// ---------------------------------------------------------------------------
// Common predicates for the visual query builder
// ---------------------------------------------------------------------------

const COMMON_PREDICATES: &[(&str, &str)] = &[
    ("rdf:type", "rdf:type"),
    ("rdfs:label", "rdfs:label"),
    ("rdfs:comment", "rdfs:comment"),
    ("ont:hasEntity", "ont:hasEntity"),
    ("ont:hasTerm", "ont:hasTerm"),
    ("doc:hasMarkup", "doc:hasMarkup"),
    ("doc:markupType", "doc:markupType"),
    ("doc:appendScope", "doc:appendScope"),
    ("prov:actor", "prov:actor"),
    ("prov:timestamp", "prov:timestamp"),
    ("prov:role", "prov:role"),
    ("prov:contributedTo", "prov:contributedTo"),
    ("prov:derivedFrom", "prov:derivedFrom"),
    ("agency:actor", "agency:actor"),
    ("agency:did", "agency:did"),
    ("inv:hasHypothesis", "inv:hasHypothesis"),
    ("inv:confidence", "inv:confidence"),
    ("epi:modality", "epi:modality"),
    ("selfhood:access", "selfhood:access"),
    ("set:capability", "set:capability"),
];

// ---------------------------------------------------------------------------
// Build the search workbench overlay
// ---------------------------------------------------------------------------

/// Build the search workbench overlay (hidden by default).
/// Triggered via Ctrl+Shift+F or the command palette.
pub fn build_search_workbench(document: &Document) -> Element {
    let overlay = document.create_element("div").unwrap();
    overlay.set_id("search-workbench");
    let ov_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    ov_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.7); z-index: 10001; display: none; \
         align-items: flex-start; justify-content: center; padding-top: 60px;",
    );

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("search-workbench-panel");
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "width: 880px; max-height: 680px; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px); \
         box-shadow: 0 12px 48px rgba(0,0,0,0.5); overflow: hidden; \
         display: flex; flex-direction: column;",
    );

    // Header bar with title + close
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 10px 16px; border-bottom: 1px solid var(--border-subtle);",
    );

    let title = document.create_element("span").unwrap();
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 13px; font-weight: 700; color: var(--accent-cyan); \
         text-transform: uppercase; letter-spacing: 0.5px; font-family: var(--font-mono);",
    );
    title.set_text_content(Some("\u{1F50D} Search Workbench"));
    header.append_child(&title).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));
    let cb_el: HtmlElement = close_btn.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "background: transparent; border: none; color: var(--text-muted); \
         cursor: pointer; font-size: 16px; padding: 4px;",
    );
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    // Mode tabs
    let tabs = document.create_element("div").unwrap();
    tabs.set_class_name("search-mode-tabs");
    let tabs_el: HtmlElement = tabs.clone().dyn_into().unwrap();
    tabs_el
        .style()
        .set_css_text("display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle);");

    for (i, (mode_id, label, icon)) in [
        ("faceted", "Faceted Search", "\u{1F3AF}"),
        ("builder", "Query Builder", "\u{1F9F9}"),
        ("sparql", "Manual SPARQL", "\u{270F}\u{FE0F}"),
        ("saved", "Saved Queries", "\u{1F4BE}"),
    ]
    .iter()
    .enumerate()
    {
        let tab = document.create_element("button").unwrap();
        tab.set_class_name("search-mode-tab");
        tab.set_attribute("data-mode", mode_id).unwrap();
        if i == 0 {
            tab.class_list().add_1("active").unwrap();
        }
        let tab_el: HtmlElement = tab.clone().dyn_into().unwrap();
        tab_el.style().set_css_text(&format!(
            "padding: 8px 16px; background: transparent; border: none; \
             border-bottom: 2px solid {}; \
             color: {}; font-size: 11px; font-family: var(--font-mono); \
             cursor: pointer; display: flex; align-items: center; gap: 6px; \
             transition: var(--trans-fast);",
            if i == 0 {
                "var(--accent-cyan)"
            } else {
                "transparent"
            },
            if i == 0 {
                "var(--text-primary)"
            } else {
                "var(--text-muted)"
            },
        ));
        tab.set_text_content(Some(&format!("{} {}", icon, label)));
        tabs.append_child(&tab).unwrap();
    }
    panel.append_child(&tabs).unwrap();

    // Content area — holds the active mode panel
    let content = document.create_element("div").unwrap();
    content.set_id("search-workbench-content");
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style().set_css_text(
        "flex: 1; overflow-y: auto; padding: 16px; display: flex; flex-direction: column; gap: 12px;"
    );

    // Build all mode panels (only the active one is visible)
    content
        .append_child(&build_faceted_panel(document))
        .unwrap();
    content
        .append_child(&build_query_builder_panel(document))
        .unwrap();
    content.append_child(&build_sparql_panel(document)).unwrap();
    content
        .append_child(&build_saved_queries_panel(document))
        .unwrap();

    // Show only the faceted panel initially
    show_mode_panel(document, "faceted");

    panel.append_child(&content).unwrap();

    // Honesty footer
    let footer = document.create_element("div").unwrap();
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "padding: 6px 16px; border-top: 1px solid var(--border-subtle); \
         font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
    );
    footer.set_text_content(Some(
        "\u{1F4A1} Query construction and saving are live. \
         SPARQL execution requires the QualiaDB daemon backend \u{2014} results are structural mocks."
    ));
    panel.append_child(&footer).unwrap();

    overlay.append_child(&panel).unwrap();

    // Wire close button
    let ov_clone = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let ov: HtmlElement = ov_clone.clone().dyn_into().unwrap();
        ov.style().set_property("display", "none").unwrap();
    }) as Box<dyn FnMut(MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();

    // Wire mode tabs
    wire_mode_tabs(document);

    // Wire faceted search
    wire_faceted_search(document);

    // Wire query builder
    wire_query_builder(document);

    // Wire SPARQL editor
    wire_sparql_editor(document);

    // Wire saved queries
    wire_saved_queries(document);

    overlay
}

// ---------------------------------------------------------------------------
// Mode switching
// ---------------------------------------------------------------------------

fn show_mode_panel(document: &Document, mode: &str) {
    let panels = document.query_selector_all(".search-mode-panel").unwrap();
    for i in 0..panels.length() {
        let p = panels.get(i).unwrap();
        let p_el: Element = p.dyn_into().unwrap();
        let p_mode = p_el.get_attribute("data-mode").unwrap_or_default();
        let html_el: HtmlElement = p_el.clone().dyn_into().unwrap();
        if p_mode == mode {
            html_el.style().set_property("display", "").unwrap();
        } else {
            html_el.style().set_property("display", "none").unwrap();
        }
    }
}

fn wire_mode_tabs(document: &Document) {
    let tabs = document.query_selector_all(".search-mode-tab").unwrap();
    for i in 0..tabs.length() {
        let tab = tabs.get(i).unwrap();
        let tab_el: Element = tab.dyn_into().unwrap();
        let mode = tab_el.get_attribute("data-mode").unwrap_or_default();
        let tab_clone = tab_el.clone();

        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Update tab active states
            let all_tabs = doc.query_selector_all(".search-mode-tab").unwrap();
            for j in 0..all_tabs.length() {
                let t = all_tabs.get(j).unwrap();
                let t_el: Element = t.dyn_into().unwrap();
                t_el.class_list().remove_1("active").unwrap();
                let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
                t_html
                    .style()
                    .set_property("border-bottom", "2px solid transparent")
                    .unwrap();
                t_html
                    .style()
                    .set_property("color", "var(--text-muted)")
                    .unwrap();
            }
            tab_clone.class_list().add_1("active").unwrap();
            let tab_html: HtmlElement = tab_clone.clone().dyn_into().unwrap();
            tab_html
                .style()
                .set_property("border-bottom", "2px solid var(--accent-cyan)")
                .unwrap();
            tab_html
                .style()
                .set_property("color", "var(--text-primary)")
                .unwrap();

            show_mode_panel(&doc, &mode);
        }) as Box<dyn FnMut(MouseEvent)>);

        tab_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Faceted search panel
// ---------------------------------------------------------------------------

fn build_faceted_panel(document: &Document) -> Element {
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

fn wire_faceted_search(document: &Document) {
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
        save_current_query(&doc, "faceted");
    }) as Box<dyn FnMut(MouseEvent)>);
    save_btn
        .add_event_listener_with_callback("click", svb_closure.as_ref().unchecked_ref())
        .unwrap();
    svb_closure.forget();
}

fn get_active_facets(document: &Document) -> Vec<(String, Vec<String>)> {
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

    // Generate mock results based on active facets
    let mock_count = 3 + (facets.len() * 2) as u32;
    let mut results_html = String::new();

    for i in 0..mock_count.min(20) {
        let facet_desc: Vec<String> = facets
            .iter()
            .map(|(f, vals)| format!("{}: {}", f, vals.join(",")))
            .collect();

        results_html.push_str(&format!(
            "<div style=\"padding: 6px 8px; border-bottom: 1px solid var(--border-subtle); \
             display: flex; align-items: center; gap: 8px;\">\
            <span style=\"color: var(--accent-cyan); font-size: 10px;\">#{:03}</span>\
            <span style=\"color: var(--text-primary);\">result-{}-{:04}</span>\
            <span style=\"color: var(--text-muted); font-size: 9px; margin-left: auto;\">{}</span>\
            </div>",
            i + 1,
            facets.first().map(|(f, _)| f.as_str()).unwrap_or("all"),
            i + 1,
            facet_desc.join(" | "),
        ));
    }

    results_el.set_inner_html(&results_html);
    count_el.set_text_content(Some(&format!(
        "{} mock results (engine wiring pending)",
        mock_count.min(20)
    )));
}

// ---------------------------------------------------------------------------
// Visual query builder panel
// ---------------------------------------------------------------------------

fn build_query_builder_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("search-mode-panel");
    panel.set_attribute("data-mode", "builder").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("display: none; flex-direction: column; gap: 12px;");

    // Prefix declarations
    let prefix_section = document.create_element("div").unwrap();
    let ps_el: HtmlElement = prefix_section.clone().dyn_into().unwrap();
    ps_el
        .style()
        .set_css_text("display: flex; flex-direction: column; gap: 4px;");

    let ps_label = document.create_element("div").unwrap();
    ps_label.set_text_content(Some("PREFIX declarations:"));
    let psl_el: HtmlElement = ps_label.clone().dyn_into().unwrap();
    psl_el.style().set_css_text("font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;");
    prefix_section.append_child(&ps_label).unwrap();

    let prefix_text = document.create_element("textarea").unwrap();
    prefix_text.set_id("builder-prefixes");
    let pt_el: HtmlTextAreaElement = prefix_text.clone().dyn_into().unwrap();
    pt_el.set_value(
        "PREFIX ont: <http://qualia.org/ontology#>\n\
         PREFIX doc: <http://qualia.org/document#>\n\
         PREFIX prov: <http://qualia.org/provenance#>\n\
         PREFIX agency: <http://qualia.org/agency#>\n\
         PREFIX epi: <http://qualia.org/epistemics#>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>",
    );
    let pt_html: HtmlElement = prefix_text.clone().dyn_into().unwrap();
    pt_html.style().set_css_text(
        "width: 100%; box-sizing: border-box; height: 80px; \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-secondary); \
         resize: vertical;",
    );
    prefix_section.append_child(&prefix_text).unwrap();
    panel.append_child(&prefix_section).unwrap();

    // Triple patterns
    let patterns_label = document.create_element("div").unwrap();
    patterns_label.set_text_content(Some("Triple Patterns:"));
    let pl_el: HtmlElement = patterns_label.clone().dyn_into().unwrap();
    pl_el.style().set_css_text("font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;");
    panel.append_child(&patterns_label).unwrap();

    let patterns_container = document.create_element("div").unwrap();
    patterns_container.set_id("builder-patterns");
    let pc_el: HtmlElement = patterns_container.clone().dyn_into().unwrap();
    pc_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 6px; \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         padding: 8px; min-height: 60px; background: var(--surface-panel);",
    );

    // Add one default pattern row
    patterns_container
        .append_child(&build_pattern_row(document, 0))
        .unwrap();

    panel.append_child(&patterns_container).unwrap();

    // Add pattern button
    let add_row_btn = document.create_element("button").unwrap();
    add_row_btn.set_id("builder-add-row");
    add_row_btn.set_text_content(Some("+ Add Pattern"));
    let ar_el: HtmlElement = add_row_btn.clone().dyn_into().unwrap();
    ar_el.style().set_css_text(
        "padding: 6px 12px; background: var(--surface-panel); \
         border: 1px dashed var(--border-subtle); border-radius: var(--radius-xs); \
         color: var(--accent-emerald); font-family: var(--font-mono); font-size: 10px; \
         cursor: pointer; align-self: flex-start;",
    );
    panel.append_child(&add_row_btn).unwrap();

    // Generated query preview
    let preview_label = document.create_element("div").unwrap();
    preview_label.set_text_content(Some("Generated SPARQL:"));
    let pv_el: HtmlElement = preview_label.clone().dyn_into().unwrap();
    pv_el.style().set_css_text("font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;");
    panel.append_child(&preview_label).unwrap();

    let preview = document.create_element("textarea").unwrap();
    preview.set_id("builder-preview");
    preview.set_attribute("readonly", "true").unwrap();
    let pv_html: HtmlElement = preview.clone().dyn_into().unwrap();
    pv_html.style().set_css_text(
        "width: 100%; box-sizing: border-box; height: 120px; \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--accent-cyan); \
         resize: vertical;",
    );
    panel.append_child(&preview).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");

    let gen_btn = document.create_element("button").unwrap();
    gen_btn.set_id("builder-generate");
    gen_btn.set_text_content(Some("\u{2699} Generate Query"));
    let gb_el: HtmlElement = gen_btn.clone().dyn_into().unwrap();
    gb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--accent-cyan); color: var(--bg-deep); \
         border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; font-weight: 700; cursor: pointer;",
    );
    actions.append_child(&gen_btn).unwrap();

    let save_btn = document.create_element("button").unwrap();
    save_btn.set_id("builder-save");
    save_btn.set_text_content(Some("\u{1F4BE} Save Query"));
    let sb_el: HtmlElement = save_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--surface-panel); color: var(--text-secondary); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
    );
    actions.append_child(&save_btn).unwrap();

    let run_btn = document.create_element("button").unwrap();
    run_btn.set_id("builder-run");
    run_btn.set_text_content(Some("\u{25B6} Run (mock)"));
    let rb_el: HtmlElement = run_btn.clone().dyn_into().unwrap();
    rb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--surface-panel); color: var(--text-secondary); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
    );
    actions.append_child(&run_btn).unwrap();

    panel.append_child(&actions).unwrap();

    // Results
    let results = document.create_element("div").unwrap();
    results.set_id("builder-results");
    let r_el: HtmlElement = results.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; min-height: 80px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);",
    );
    results.set_text_content(Some(
        "Click \"Generate Query\" then \"Run (mock)\" to see results.",
    ));
    panel.append_child(&results).unwrap();

    panel
}

fn build_pattern_row(document: &Document, idx: usize) -> Element {
    let row = document.create_element("div").unwrap();
    row.set_class_name("builder-pattern-row");
    row.set_attribute("data-row-idx", &idx.to_string()).unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 6px; align-items: center; flex-wrap: wrap;");

    // Subject
    let subj = document.create_element("input").unwrap();
    subj.set_class_name("pattern-subject");
    subj.set_attribute("type", "text").unwrap();
    subj.set_attribute("placeholder", "?subject").unwrap();
    subj.set_attribute("value", &format!("?s{}", idx)).unwrap();
    let s_el: HtmlElement = subj.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "width: 100px; padding: 4px 6px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );
    row.append_child(&subj).unwrap();

    // Predicate dropdown
    let pred = document.create_element("select").unwrap();
    pred.set_class_name("pattern-predicate");
    let p_el: HtmlElement = pred.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "padding: 4px 6px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );
    for (key, display) in COMMON_PREDICATES {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", key).unwrap();
        opt.set_text_content(Some(display));
        pred.append_child(&opt).unwrap();
    }
    // Add a "custom" option
    let custom_opt = document.create_element("option").unwrap();
    custom_opt.set_attribute("value", "custom").unwrap();
    custom_opt.set_text_content(Some("custom\u{2026}"));
    pred.append_child(&custom_opt).unwrap();
    row.append_child(&pred).unwrap();

    // Object
    let obj = document.create_element("input").unwrap();
    obj.set_class_name("pattern-object");
    obj.set_attribute("type", "text").unwrap();
    obj.set_attribute("placeholder", "?object").unwrap();
    obj.set_attribute("value", &format!("?o{}", idx)).unwrap();
    let o_el: HtmlElement = obj.clone().dyn_into().unwrap();
    o_el.style().set_css_text(
        "width: 100px; padding: 4px 6px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );
    row.append_child(&obj).unwrap();

    // Remove button
    let remove_btn = document.create_element("button").unwrap();
    remove_btn.set_class_name("pattern-remove");
    remove_btn.set_text_content(Some("\u{2715}"));
    let rm_el: HtmlElement = remove_btn.clone().dyn_into().unwrap();
    rm_el.style().set_css_text(
        "padding: 4px 8px; background: transparent; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); color: var(--accent-red); cursor: pointer; font-size: 10px;"
    );
    row.append_child(&remove_btn).unwrap();

    // Wire remove button
    let row_clone = row.clone();
    let rm_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        row_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    remove_btn
        .add_event_listener_with_callback("click", rm_closure.as_ref().unchecked_ref())
        .unwrap();
    rm_closure.forget();

    row
}

fn wire_query_builder(document: &Document) {
    // Wire add row button
    if let Some(add_btn) = document.get_element_by_id("builder-add-row") {
        let patterns_container = document.get_element_by_id("builder-patterns");
        let add_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            if let Some(container) = patterns_container.as_ref() {
                let count = container
                    .query_selector_all(".builder-pattern-row")
                    .unwrap()
                    .length();
                let row = build_pattern_row(&doc, count as usize);
                container.append_child(&row).unwrap();
            }
        }) as Box<dyn FnMut(MouseEvent)>);
        add_btn
            .add_event_listener_with_callback("click", add_closure.as_ref().unchecked_ref())
            .unwrap();
        add_closure.forget();
    }

    // Wire generate button
    if let Some(gen_btn) = document.get_element_by_id("builder-generate") {
        let gen_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            generate_builder_query(&doc);
        }) as Box<dyn FnMut(MouseEvent)>);
        gen_btn
            .add_event_listener_with_callback("click", gen_closure.as_ref().unchecked_ref())
            .unwrap();
        gen_closure.forget();
    }

    // Wire save button
    if let Some(save_btn) = document.get_element_by_id("builder-save") {
        let svb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            save_current_query(&doc, "builder");
        }) as Box<dyn FnMut(MouseEvent)>);
        save_btn
            .add_event_listener_with_callback("click", svb_closure.as_ref().unchecked_ref())
            .unwrap();
        svb_closure.forget();
    }

    // Wire run button
    if let Some(run_btn) = document.get_element_by_id("builder-run") {
        let rb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            run_mock_query(&doc, "builder-results");
        }) as Box<dyn FnMut(MouseEvent)>);
        run_btn
            .add_event_listener_with_callback("click", rb_closure.as_ref().unchecked_ref())
            .unwrap();
        rb_closure.forget();
    }
}

fn generate_builder_query(document: &Document) {
    // Get prefixes
    let prefixes = match document.get_element_by_id("builder-prefixes") {
        Some(p) => {
            let ta: HtmlTextAreaElement = p.dyn_into().unwrap();
            ta.value()
        }
        None => return,
    };

    // Get patterns
    let patterns = match document.get_element_by_id("builder-patterns") {
        Some(p) => p,
        None => return,
    };

    let rows = patterns.query_selector_all(".builder-pattern-row").unwrap();
    let mut pattern_lines = Vec::new();
    let mut vars = Vec::new();

    for i in 0..rows.length() {
        let row = rows.get(i).unwrap();
        let row_el: Element = row.dyn_into().unwrap();

        let subj = row_el.query_selector(".pattern-subject").unwrap().unwrap();
        let subj_input: HtmlInputElement = subj.dyn_into().unwrap();
        let s = subj_input.value();

        let pred = row_el
            .query_selector(".pattern-predicate")
            .unwrap()
            .unwrap();
        let pred_select: web_sys::HtmlSelectElement = pred.dyn_into().unwrap();
        let p = pred_select.value();

        let obj = row_el.query_selector(".pattern-object").unwrap().unwrap();
        let obj_input: HtmlInputElement = obj.dyn_into().unwrap();
        let o = obj_input.value();

        pattern_lines.push(format!("  {} {} {} .", s, p, o));

        // Collect variables for SELECT
        if s.starts_with('?') && !vars.contains(&s) {
            vars.push(s.clone());
        }
        if o.starts_with('?') && !vars.contains(&o) {
            vars.push(o.clone());
        }
    }

    if pattern_lines.is_empty() {
        pattern_lines.push("  ?s ?p ?o .".to_string());
        vars = vec!["?s".to_string(), "?p".to_string(), "?o".to_string()];
    }

    let select_vars = if vars.is_empty() {
        "*".to_string()
    } else {
        vars.join(" ")
    };

    let query = format!(
        "{}\nSELECT {} WHERE {{\n{}\n}}\nLIMIT 100",
        prefixes,
        select_vars,
        pattern_lines.join("\n")
    );

    // Set preview
    if let Some(preview) = document.get_element_by_id("builder-preview") {
        let ta: HtmlTextAreaElement = preview.dyn_into().unwrap();
        ta.set_value(&query);
    }
}

// ---------------------------------------------------------------------------
// Manual SPARQL panel
// ---------------------------------------------------------------------------

fn build_sparql_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("search-mode-panel");
    panel.set_attribute("data-mode", "sparql").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("display: none; flex-direction: column; gap: 12px;");

    // Query name input (for saving)
    let name_row = document.create_element("div").unwrap();
    let nr_el: HtmlElement = name_row.clone().dyn_into().unwrap();
    nr_el
        .style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");

    let name_label = document.create_element("span").unwrap();
    name_label.set_text_content(Some("Query name:"));
    let nl_el: HtmlElement = name_label.clone().dyn_into().unwrap();
    nl_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    name_row.append_child(&name_label).unwrap();

    let name_input = document.create_element("input").unwrap();
    name_input.set_id("sparql-query-name");
    name_input.set_attribute("type", "text").unwrap();
    name_input.set_attribute("placeholder", "my-query").unwrap();
    let ni_el: HtmlElement = name_input.clone().dyn_into().unwrap();
    ni_el.style().set_css_text(
        "flex: 1; padding: 6px 10px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-primary);",
    );
    name_row.append_child(&name_input).unwrap();
    panel.append_child(&name_row).unwrap();

    // SPARQL editor
    let editor = document.create_element("textarea").unwrap();
    editor.set_id("sparql-editor");
    let ed_el: HtmlTextAreaElement = editor.clone().dyn_into().unwrap();
    ed_el.set_value(
        "PREFIX ont: <http://qualia.org/ontology#>\n\
         PREFIX doc: <http://qualia.org/document#>\n\
         PREFIX prov: <http://qualia.org/provenance#>\n\n\
         SELECT ?subject ?label ?type WHERE {\n\
         \x20\x20?subject rdf:type ?type .\n\
         \x20\x20?subject rdfs:label ?label .\n\
         \x20\x20FILTER(LANG(?label) = \"en\")\n\
         }\n\
         LIMIT 50",
    );
    let ed_html: HtmlElement = editor.clone().dyn_into().unwrap();
    ed_html.style().set_css_text(
        "flex: 1; width: 100%; box-sizing: border-box; min-height: 200px; \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 12px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--accent-cyan); \
         line-height: 1.5; resize: vertical;",
    );
    panel.append_child(&editor).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");

    let run_btn = document.create_element("button").unwrap();
    run_btn.set_id("sparql-run");
    run_btn.set_text_content(Some("\u{25B6} Run Query"));
    let rb_el: HtmlElement = run_btn.clone().dyn_into().unwrap();
    rb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--accent-cyan); color: var(--bg-deep); \
         border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; font-weight: 700; cursor: pointer;",
    );
    actions.append_child(&run_btn).unwrap();

    let save_btn = document.create_element("button").unwrap();
    save_btn.set_id("sparql-save");
    save_btn.set_text_content(Some("\u{1F4BE} Save Query"));
    let sb_el: HtmlElement = save_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--surface-panel); color: var(--text-secondary); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
    );
    actions.append_child(&save_btn).unwrap();

    let place_btn = document.create_element("button").unwrap();
    place_btn.set_id("sparql-place-container");
    place_btn.set_text_content(Some("\u{1F4CB} Use as Container Source"));
    let pb_el: HtmlElement = place_btn.clone().dyn_into().unwrap();
    pb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--surface-panel); color: var(--accent-violet); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
    );
    actions.append_child(&place_btn).unwrap();

    panel.append_child(&actions).unwrap();

    // Results
    let results = document.create_element("div").unwrap();
    results.set_id("sparql-results");
    let r_el: HtmlElement = results.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; min-height: 80px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);",
    );
    results.set_text_content(Some("Click \"Run Query\" to execute (mock results)."));
    panel.append_child(&results).unwrap();

    panel
}

fn wire_sparql_editor(document: &Document) {
    // Wire run button
    if let Some(run_btn) = document.get_element_by_id("sparql-run") {
        let rb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            run_mock_query(&doc, "sparql-results");
        }) as Box<dyn FnMut(MouseEvent)>);
        run_btn
            .add_event_listener_with_callback("click", rb_closure.as_ref().unchecked_ref())
            .unwrap();
        rb_closure.forget();
    }

    // Wire save button
    if let Some(save_btn) = document.get_element_by_id("sparql-save") {
        let svb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            save_current_query(&doc, "sparql");
        }) as Box<dyn FnMut(MouseEvent)>);
        save_btn
            .add_event_listener_with_callback("click", svb_closure.as_ref().unchecked_ref())
            .unwrap();
        svb_closure.forget();
    }

    // Wire place container button
    if let Some(place_btn) = document.get_element_by_id("sparql-place-container") {
        let pb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            place_query_container(&doc);
        }) as Box<dyn FnMut(MouseEvent)>);
        place_btn
            .add_event_listener_with_callback("click", pb_closure.as_ref().unchecked_ref())
            .unwrap();
        pb_closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Saved queries panel
// ---------------------------------------------------------------------------

fn build_saved_queries_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("search-mode-panel");
    panel.set_attribute("data-mode", "saved").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("display: none; flex-direction: column; gap: 8px;");

    let label = document.create_element("div").unwrap();
    label.set_text_content(Some("Saved Queries (persisted in localStorage):"));
    let l_el: HtmlElement = label.clone().dyn_into().unwrap();
    l_el.style().set_css_text("font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;");
    panel.append_child(&label).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_id("saved-queries-list");
    let l_el: HtmlElement = list.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; min-height: 200px; \
         display: flex; flex-direction: column; gap: 4px;",
    );
    panel.append_child(&list).unwrap();

    let refresh_btn = document.create_element("button").unwrap();
    refresh_btn.set_id("saved-refresh");
    refresh_btn.set_text_content(Some("\u{1F504} Refresh"));
    let r_el: HtmlElement = refresh_btn.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "padding: 6px 12px; background: var(--surface-panel); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         color: var(--text-secondary); font-family: var(--font-mono); font-size: 10px; \
         cursor: pointer; align-self: flex-start;",
    );
    panel.append_child(&refresh_btn).unwrap();

    panel
}

fn wire_saved_queries(document: &Document) {
    if let Some(refresh_btn) = document.get_element_by_id("saved-refresh") {
        let r_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            render_saved_queries(&doc);
        }) as Box<dyn FnMut(MouseEvent)>);
        refresh_btn
            .add_event_listener_with_callback("click", r_closure.as_ref().unchecked_ref())
            .unwrap();
        r_closure.forget();
    }
}

fn render_saved_queries(document: &Document) {
    let list = match document.get_element_by_id("saved-queries-list") {
        Some(l) => l,
        None => return,
    };
    list.set_inner_html("");

    let queries = load_saved_queries();

    if queries.is_empty() {
        let empty = document.create_element("div").unwrap();
        let e_el: HtmlElement = empty.clone().dyn_into().unwrap();
        e_el.style().set_css_text(
            "font-size: 11px; color: var(--text-muted); padding: 20px; text-align: center;",
        );
        empty.set_text_content(Some(
            "No saved queries yet. Use \u{1F4BE} Save Query in any mode to save a query.",
        ));
        list.append_child(&empty).unwrap();
        return;
    }

    for q in &queries {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(
            "display: flex; align-items: center; gap: 8px; padding: 8px 10px; \
             background: var(--surface-panel); border-radius: var(--radius-xs); \
             border-left: 3px solid var(--accent-cyan);",
        );

        // Name + metadata
        let info = document.create_element("div").unwrap();
        let inf_el: HtmlElement = info.clone().dyn_into().unwrap();
        inf_el
            .style()
            .set_css_text("flex: 1; display: flex; flex-direction: column; gap: 2px;");

        let name = document.create_element("span").unwrap();
        let n_el: HtmlElement = name.clone().dyn_into().unwrap();
        n_el.style().set_css_text("font-size: 11px; font-weight: 600; color: var(--text-primary); font-family: var(--font-mono);");
        name.set_text_content(Some(&q.name));
        info.append_child(&name).unwrap();

        let meta = document.create_element("span").unwrap();
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        meta.set_text_content(Some(&format!(
            "[{}] {} chars \u{2014} {}",
            q.mode,
            q.query.len(),
            q.timestamp
        )));
        info.append_child(&meta).unwrap();

        item.append_child(&info).unwrap();

        // Load button
        let load_btn = document.create_element("button").unwrap();
        load_btn.set_text_content(Some("Load"));
        let lb_el: HtmlElement = load_btn.clone().dyn_into().unwrap();
        lb_el.style().set_css_text(
            "padding: 4px 10px; background: var(--surface-panel-elevated); \
             border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
             color: var(--accent-cyan); font-family: var(--font-mono); font-size: 9px; cursor: pointer;"
        );
        let q_clone = q.clone();
        let load_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            load_query_into_editor(&doc, &q_clone);
        }) as Box<dyn FnMut(MouseEvent)>);
        load_btn
            .add_event_listener_with_callback("click", load_closure.as_ref().unchecked_ref())
            .unwrap();
        load_closure.forget();
        item.append_child(&load_btn).unwrap();

        // Place container button
        let place_btn = document.create_element("button").unwrap();
        place_btn.set_text_content(Some("Place"));
        let pb_el: HtmlElement = place_btn.clone().dyn_into().unwrap();
        pb_el.style().set_css_text(
            "padding: 4px 10px; background: var(--surface-panel-elevated); \
             border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
             color: var(--accent-violet); font-family: var(--font-mono); font-size: 9px; cursor: pointer;"
        );
        let q_clone2 = q.clone();
        let place_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            place_named_query_container(&doc, &q_clone2);
        }) as Box<dyn FnMut(MouseEvent)>);
        place_btn
            .add_event_listener_with_callback("click", place_closure.as_ref().unchecked_ref())
            .unwrap();
        place_closure.forget();
        item.append_child(&place_btn).unwrap();

        // Delete button
        let del_btn = document.create_element("button").unwrap();
        del_btn.set_text_content(Some("\u{2715}"));
        let db_el: HtmlElement = del_btn.clone().dyn_into().unwrap();
        db_el.style().set_css_text(
            "padding: 4px 8px; background: transparent; \
             border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
             color: var(--accent-red); font-family: var(--font-mono); font-size: 9px; cursor: pointer;"
        );
        let q_id = q.id.clone();
        let del_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            delete_saved_query(&doc, &q_id);
        }) as Box<dyn FnMut(MouseEvent)>);
        del_btn
            .add_event_listener_with_callback("click", del_closure.as_ref().unchecked_ref())
            .unwrap();
        del_closure.forget();
        item.append_child(&del_btn).unwrap();

        list.append_child(&item).unwrap();
    }
}

// ---------------------------------------------------------------------------
// Saved query persistence
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct SavedQuery {
    id: String,
    name: String,
    mode: String,
    query: String,
    timestamp: String,
}

fn load_saved_queries() -> Vec<SavedQuery> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return Vec::new(),
    };
    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return Vec::new(),
    };

    let json = match storage.get_item("qualia-ui:saved-queries") {
        Ok(Some(s)) => s,
        _ => return Vec::new(),
    };

    parse_saved_queries(&json)
}

fn parse_saved_queries(json: &str) -> Vec<SavedQuery> {
    let mut queries = Vec::new();
    // Simple JSON array parser: each entry is {"id":"...","name":"...","mode":"...","query":"...","timestamp":"..."}
    let parts: Vec<&str> = json.split("},{").collect();
    for (i, part) in parts.iter().enumerate() {
        let json_str = if i == 0 {
            if part.starts_with('[') {
                &part[1..]
            } else {
                part
            }
        } else if i == parts.len() - 1 {
            if part.ends_with(']') {
                &part[..part.len() - 1]
            } else {
                part
            }
        } else {
            part
        };

        let id = extract_json_str(json_str, "id").unwrap_or_default();
        let name = extract_json_str(json_str, "name").unwrap_or_default();
        let mode = extract_json_str(json_str, "mode").unwrap_or_default();
        let query = extract_json_str(json_str, "query").unwrap_or_default();
        let timestamp = extract_json_str(json_str, "timestamp").unwrap_or_default();

        if !id.is_empty() {
            queries.push(SavedQuery {
                id,
                name,
                mode,
                query,
                timestamp,
            });
        }
    }
    queries
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    // Find the closing quote (handle escaped quotes)
    let mut end = 0;
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            chars.next(); // skip escaped char
            end += 2;
            continue;
        }
        if c == '"' {
            break;
        }
        end += c.len_utf8();
    }
    let raw = &rest[..end];
    // Unescape
    Some(
        raw.replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\\\", "\\"),
    )
}

fn save_query_to_storage(query: &SavedQuery) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return,
    };

    let mut existing = load_saved_queries();
    existing.push(query.clone());

    let json = existing.iter().map(|q| {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"mode\":\"{}\",\"query\":\"{}\",\"timestamp\":\"{}\"}}",
            q.id.replace("\"", "\\\""),
            q.name.replace("\"", "\\\""),
            q.mode.replace("\"", "\\\""),
            q.query.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n"),
            q.timestamp.replace("\"", "\\\""),
        )
    }).collect::<Vec<_>>().join(",");

    let _ = storage.set_item("qualia-ui:saved-queries", &format!("[{}]", json));
}

fn delete_saved_query_from_storage(id: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return,
    };

    let mut existing = load_saved_queries();
    existing.retain(|q| q.id != id);

    let json = existing.iter().map(|q| {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"mode\":\"{}\",\"query\":\"{}\",\"timestamp\":\"{}\"}}",
            q.id.replace("\"", "\\\""),
            q.name.replace("\"", "\\\""),
            q.mode.replace("\"", "\\\""),
            q.query.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n"),
            q.timestamp.replace("\"", "\\\""),
        )
    }).collect::<Vec<_>>().join(",");

    let _ = storage.set_item("qualia-ui:saved-queries", &format!("[{}]", json));
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

fn save_current_query(document: &Document, mode: &str) {
    // Get query text based on mode
    let (query_text, name) = match mode {
        "faceted" => {
            let facets = get_active_facets(document);
            let facet_desc: Vec<String> = facets
                .iter()
                .map(|(f, v)| format!("{}={}", f, v.join(",")))
                .collect();
            let q = format!(
                "# Faceted search\n# Facets: {}\nSELECT * WHERE {{ ?s ?p ?o . }} LIMIT 100",
                facet_desc.join("; ")
            );
            let name = format!("faceted-{}", js_sys::Date::now() as u64 % 10000);
            (q, name)
        }
        "builder" => {
            let preview = document
                .get_element_by_id("builder-preview")
                .and_then(|p| p.dyn_into::<HtmlTextAreaElement>().ok())
                .map(|ta| ta.value())
                .unwrap_or_default();
            let name = format!("builder-{}", js_sys::Date::now() as u64 % 10000);
            (preview, name)
        }
        "sparql" => {
            let editor = document
                .get_element_by_id("sparql-editor")
                .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
                .map(|ta| ta.value())
                .unwrap_or_default();
            let name = document
                .get_element_by_id("sparql-query-name")
                .and_then(|n| n.dyn_into::<HtmlInputElement>().ok())
                .map(|i| i.value())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("sparql-{}", js_sys::Date::now() as u64 % 10000));
            (editor, name)
        }
        _ => return,
    };

    if query_text.trim().is_empty() {
        show_search_notification(document, "Query is empty \u{2014} nothing to save.");
        return;
    }

    let now = js_sys::Date::new_0();
    let timestamp = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        now.get_full_year(),
        now.get_month() + 1,
        now.get_date(),
        now.get_hours(),
        now.get_minutes(),
        now.get_seconds()
    );

    let saved = SavedQuery {
        id: format!("q-{}", js_sys::Date::now() as u64),
        name,
        mode: mode.to_string(),
        query: query_text,
        timestamp,
    };

    save_query_to_storage(&saved);
    show_search_notification(
        document,
        &format!("Saved query \u{201C}{}\u{201D}", saved.name),
    );
    render_saved_queries(document);
}

fn load_query_into_editor(document: &Document, query: &SavedQuery) {
    // Load into the SPARQL editor
    if let Some(editor) = document.get_element_by_id("sparql-editor") {
        let ta: HtmlTextAreaElement = editor.dyn_into().unwrap();
        ta.set_value(&query.query);
    }
    if let Some(name_input) = document.get_element_by_id("sparql-query-name") {
        let input: HtmlInputElement = name_input.dyn_into().unwrap();
        input.set_value(&query.name);
    }

    // Switch to SPARQL mode
    show_mode_panel(document, "sparql");
    // Update tab active states
    let tabs = document.query_selector_all(".search-mode-tab").unwrap();
    for i in 0..tabs.length() {
        let t = tabs.get(i).unwrap();
        let t_el: Element = t.dyn_into().unwrap();
        let t_mode = t_el.get_attribute("data-mode").unwrap_or_default();
        if t_mode == "sparql" {
            t_el.class_list().add_1("active").unwrap();
            let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
            t_html
                .style()
                .set_property("border-bottom", "2px solid var(--accent-cyan)")
                .unwrap();
            t_html
                .style()
                .set_property("color", "var(--text-primary)")
                .unwrap();
        } else {
            t_el.class_list().remove_1("active").unwrap();
            let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
            t_html
                .style()
                .set_property("border-bottom", "2px solid transparent")
                .unwrap();
            t_html
                .style()
                .set_property("color", "var(--text-muted)")
                .unwrap();
        }
    }

    show_search_notification(
        document,
        &format!("Loaded \u{201C}{}\u{201D} into SPARQL editor", query.name),
    );
}

fn delete_saved_query(document: &Document, id: &str) {
    delete_saved_query_from_storage(id);
    render_saved_queries(document);
    show_search_notification(document, "Query deleted");
}

fn run_mock_query(document: &Document, results_id: &str) {
    let results = match document.get_element_by_id(results_id) {
        Some(r) => r,
        None => return,
    };
    results.set_inner_html("");

    let query_text = document
        .get_element_by_id("sparql-editor")
        .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|ta| ta.value())
        .unwrap_or_default();

    if crate::browser::native_daemon::is_daemon_connected() {
        results.set_inner_html("<div style=\"padding: 8px; color: var(--accent-cyan); font-size: 11px;\">\u{25CB} Executing query on native daemon\u{2026}</div>");
        let results_id_owned = results_id.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let res = crate::browser::native_daemon::daemon_query(&query_text).await;
            if let Some(window) = web_sys::window() {
                if let Some(doc) = window.document() {
                    if let Some(target) = doc.get_element_by_id(&results_id_owned) {
                        match res {
                            Ok(output) => {
                                let mut html = String::new();
                                html.push_str("<div style=\"padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); font-size: 9px; color: var(--accent-emerald); margin-bottom: 4px;\">\u{25CF} Live Results from Native Daemon</div>");
                                html.push_str(&format!("<pre style=\"margin: 0; padding: 6px; font-family: var(--font-mono); font-size: 11px; color: var(--text-primary); white-space: pre-wrap; max-height: 240px; overflow-y: auto;\">{}</pre>", output.replace('<', "&lt;").replace('>', "&gt;")));
                                target.set_inner_html(&html);
                            }
                            Err(err) => {
                                target.set_inner_html(&format!("<div style=\"padding: 8px; color: var(--accent-amber); font-size: 11px;\">\u{26A0} Daemon Query Error: {}</div>", err));
                            }
                        }
                    }
                }
            }
        });
        return;
    }

    // Generate mock results when offline
    let mut html = String::new();
    html.push_str("<div style=\"padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); font-size: 9px; color: var(--text-muted); margin-bottom: 4px;\">Mock results \u{2014} Standalone WASM mode (QualiaDB daemon offline)</div>");

    for i in 0..10 {
        html.push_str(&format!(
            "<div style=\"padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); display: flex; gap: 8px;\">\
            <span style=\"color: var(--accent-cyan); font-size: 10px;\">#{:02}</span>\
            <span style=\"color: var(--text-primary);\">?subject = &lt;qualia:entity/{:04}&gt;</span>\
            <span style=\"color: var(--text-muted); font-size: 9px; margin-left: auto;\">?label = \"Entity {:04}\"</span>\
            </div>",
            i + 1, i + 1, i + 1,
        ));
    }

    results.set_inner_html(&html);
}

fn place_query_container(document: &Document) {
    // Get the current SPARQL query
    let query = document
        .get_element_by_id("sparql-editor")
        .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|ta| ta.value())
        .unwrap_or_default();
    let name = document
        .get_element_by_id("sparql-query-name")
        .and_then(|n| n.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Query Results".to_string());

    place_query_container_on_canvas(document, &name, &query);
}

fn place_named_query_container(document: &Document, query: &SavedQuery) {
    place_query_container_on_canvas(document, &query.name, &query.query);
    // Close the workbench
    if let Some(wb) = document.get_element_by_id("search-workbench") {
        let wb_el: HtmlElement = wb.dyn_into().unwrap();
        wb_el.style().set_property("display", "none").unwrap();
    }
}

fn place_query_container_on_canvas(document: &Document, name: &str, query: &str) {
    use crate::tool_chest::core::registry::SeedContainer;

    let existing = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    let count = existing.length() as f32;
    let x = 80.0 + (count % 5.0) * 40.0;
    let y = 60.0 + (count % 5.0) * 40.0;

    let container = SeedContainer {
        container_type: "graph".into(),
        title: format!("\u{1F50D} {}", name),
        x,
        y,
        width: 480.0,
        height: 360.0,
        z: 100.0 + count,
        honesty: "present".into(),
        ..Default::default()
    };

    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        let el = super::containers::build_container(document, &container);

        // Inject the query into the container body as a data attribute
        el.set_attribute("data-query", query).unwrap();
        el.set_attribute("data-query-name", name).unwrap();

        if let Some(content) = canvas.query_selector(".canvas-content-layer").unwrap() {
            content.append_child(&el).unwrap();
        } else {
            canvas.append_child(&el).unwrap();
        }

        // Re-wire interactions
        super::interactions::wire_container_selection(document);
        super::interactions::wire_container_dragging(document);
        super::interactions::wire_container_resize(document);
        super::interactions::wire_container_deletion(document);
        super::interactions::wire_port_dragging(document);

        super::history::push_current_frame("place query container");
    }

    show_search_notification(
        document,
        &format!(
            "Placed \u{201C}{}\u{201D} as graph container on canvas",
            name
        ),
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn show_search_notification(document: &Document, message: &str) {
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; \
         background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 10002; max-width: 320px;",
    );
    notif.set_text_content(Some(&format!("\u{1F50D} {}", message)));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 2500);
    timeout.forget();
}

/// Toggle the search workbench visibility.
pub fn toggle_search_workbench(document: &Document) {
    if let Some(wb) = document.get_element_by_id("search-workbench") {
        let wb_el: HtmlElement = wb.dyn_into().unwrap();
        let display = wb_el
            .style()
            .get_property_value("display")
            .unwrap_or_default();
        if display == "none" {
            wb_el.style().set_property("display", "flex").unwrap();
            // Refresh saved queries
            render_saved_queries(document);
        } else {
            wb_el.style().set_property("display", "none").unwrap();
        }
    }
}

/// Open the search workbench to a specific mode.
pub fn open_to_mode(document: &Document, mode: &str) {
    if let Some(wb) = document.get_element_by_id("search-workbench") {
        let wb_el: HtmlElement = wb.dyn_into().unwrap();
        wb_el.style().set_property("display", "flex").unwrap();
    }
    // Update tab active states
    let tabs = document.query_selector_all(".search-mode-tab").unwrap();
    for i in 0..tabs.length() {
        let t = tabs.get(i).unwrap();
        let t_el: Element = t.dyn_into().unwrap();
        let t_mode = t_el.get_attribute("data-mode").unwrap_or_default();
        if t_mode == mode {
            t_el.class_list().add_1("active").unwrap();
            let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
            t_html
                .style()
                .set_property("border-bottom", "2px solid var(--accent-cyan)")
                .unwrap();
            t_html
                .style()
                .set_property("color", "var(--text-primary)")
                .unwrap();
        } else {
            t_el.class_list().remove_1("active").unwrap();
            let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
            t_html
                .style()
                .set_property("border-bottom", "2px solid transparent")
                .unwrap();
            t_html
                .style()
                .set_property("color", "var(--text-muted)")
                .unwrap();
        }
    }
    show_mode_panel(document, mode);
    if mode == "saved" {
        render_saved_queries(document);
    }
}

/// Wire Ctrl+Shift+F to toggle the search workbench.
pub fn wire_search_workbench_shortcut(document: &Document) {
    let closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        if e.key() == "F" && e.shift_key() && (e.ctrl_key() || e.meta_key()) {
            e.prevent_default();
            let doc = web_sys::window().unwrap().document().unwrap();
            toggle_search_workbench(&doc);
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);

    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_str_simple() {
        let json = r#"{"id":"q-123","name":"test"}"#;
        assert_eq!(extract_json_str(json, "id"), Some("q-123".to_string()));
        assert_eq!(extract_json_str(json, "name"), Some("test".to_string()));
    }

    #[test]
    fn test_extract_json_str_escaped() {
        let json = r#"{"query":"SELECT ?s WHERE { ?s ?p ?o }"}"#;
        assert_eq!(
            extract_json_str(json, "query"),
            Some("SELECT ?s WHERE { ?s ?p ?o }".to_string())
        );
    }

    #[test]
    fn test_extract_json_str_newlines() {
        let json = r#"{"query":"SELECT\n?s\nWHERE"}"#;
        let result = extract_json_str(json, "query").unwrap();
        assert!(result.contains("SELECT"));
        assert!(result.contains("?s"));
    }

    #[test]
    fn test_parse_saved_queries_empty() {
        assert!(parse_saved_queries("[]").is_empty());
    }

    #[test]
    fn test_parse_saved_queries_single() {
        let json = r#"[{"id":"q1","name":"test","mode":"sparql","query":"SELECT * WHERE {}","timestamp":"2026-01-01"}]"#;
        let queries = parse_saved_queries(json);
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].id, "q1");
        assert_eq!(queries[0].name, "test");
    }
}
