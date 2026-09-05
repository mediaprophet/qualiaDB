//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Hypermedia library browser.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Library / Lived Memory (§3.8b — P0 critical gap)
// ---------------------------------------------------------------------------

/// Hypermedia library browser — 8 sections, search, facet filters, stats.
pub fn build_library_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    // Stats bar
    let stats = document.create_element("div").unwrap();
    stats.set_class_name("vibe-toolbar");
    let stats_el: HtmlElement = stats.clone().dyn_into().unwrap();
    stats_el.style().set_css_text(
        "gap: 12px; font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
    );
    for (label, value) in &[
        ("Documents", "142"),
        ("Ontologies", "36"),
        ("Models", "8"),
        ("Graph facts", "12.4k"),
    ] {
        let stat = document.create_element("span").unwrap();
        stat.set_text_content(Some(&format!("{}: {}", label, value)));
        stats.append_child(&stat).unwrap();
    }
    wrapper.append_child(&stats).unwrap();

    // Section tabs
    let tabs = document.create_element("div").unwrap();
    tabs.set_class_name("vibe-toolbar");
    let tabs_el: HtmlElement = tabs.clone().dyn_into().unwrap();
    tabs_el.style().set_css_text("gap: 2px; flex-wrap: wrap;");
    for section in &[
        "All", "Secret", "Wellfair", "Personal", "Work", "Tools", "Software", "Commons",
    ] {
        let tab = document.create_element("button").unwrap();
        tab.set_class_name("vibe-run-btn");
        let tab_el: HtmlElement = tab.clone().dyn_into().unwrap();
        tab_el
            .style()
            .set_css_text("font-size: 10px; padding: 2px 8px;");
        tab.set_text_content(Some(section));
        tabs.append_child(&tab).unwrap();
    }
    wrapper.append_child(&tabs).unwrap();

    // Search bar
    let search = document.create_element("div").unwrap();
    search.set_class_name("vibe-toolbar");
    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_placeholder("Search library\u{2026}");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 4px 8px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono);").unwrap();
    search.append_child(&input).unwrap();
    let facet_btn = document.create_element("button").unwrap();
    facet_btn.set_class_name("vibe-run-btn");
    facet_btn.set_text_content(Some("Facets"));
    search.append_child(&facet_btn).unwrap();
    let ingest_btn = document.create_element("button").unwrap();
    ingest_btn.set_class_name("vibe-run-btn");
    ingest_btn.set_text_content(Some("+ Ingest"));
    search.append_child(&ingest_btn).unwrap();
    wrapper.append_child(&search).unwrap();

    // Entry list
    let list = document.create_element("div").unwrap();
    list.set_class_name("vibe-output");
    let entries = &[
        (
            "doc",
            "Research notes — Q42 embedding manifold",
            "wellfair",
            "present",
        ),
        (
            "ontology",
            "epistemics.n3 — epistemic modalities",
            "software",
            "live",
        ),
        (
            "pdf",
            "Framingham risk assessment — clinical",
            "wellfair",
            "partial",
        ),
        (
            "image",
            "Satellite imagery — catchment hull",
            "work",
            "present",
        ),
        ("model", "Llama-3.2-1B — quantised GGUF", "tools", "live"),
        (
            "text",
            "Agreement draft — fiduciary obligation",
            "personal",
            "present",
        ),
        (
            "ontology",
            "hypermedia.n3 — HCF container spec",
            "software",
            "live",
        ),
        (
            "text",
            "Legislation — Health Records Act 2024",
            "commons",
            "present",
        ),
    ];
    for (kind, title, section, honesty) in entries {
        let row = document.create_element("div").unwrap();
        row.set_class_name("vibe-out-line");
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el
            .style()
            .set_css_text("display: flex; align-items: center; gap: 8px;");

        let kind_badge = document.create_element("span").unwrap();
        kind_badge.set_class_name(&format!("honesty-badge honesty-{}", honesty));
        kind_badge.set_text_content(Some(kind));
        row.append_child(&kind_badge).unwrap();

        let title_el = document.create_element("span").unwrap();
        title_el.set_text_content(Some(title));
        row.append_child(&title_el).unwrap();

        let section_el = document.create_element("span").unwrap();
        let s_el: HtmlElement = section_el.clone().dyn_into().unwrap();
        s_el.style()
            .set_css_text("margin-left: auto; color: var(--text-muted); font-size: 9px;");
        section_el.set_text_content(Some(section));
        row.append_child(&section_el).unwrap();

        list.append_child(&row).unwrap();
    }
    wrapper.append_child(&list).unwrap();

    wrapper
}
