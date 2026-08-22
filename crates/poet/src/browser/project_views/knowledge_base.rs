//! Knowledge Base — aggregation, graph, search (§8k.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("sources", "Indexed Sources"),
    ("graph", "Knowledge Graph"),
    ("search", "Search"),
    ("coverage", "Coverage"),
];

const INDEXED_SOURCES: &[(&str, &str, &str, &str, &str)] = &[
    ("Wiki Pages", "24 pages", "indexed", "2026-08-18", "Public"),
    (
        "Discussion Threads",
        "12 threads",
        "indexed",
        "2026-08-18",
        "Public",
    ),
    (
        "Decisions",
        "8 decisions",
        "indexed",
        "2026-08-18",
        "Public",
    ),
    (
        "Meeting Minutes",
        "2 meetings",
        "indexed",
        "2026-08-18",
        "Public",
    ),
    ("Deliverables", "5 items", "indexed", "2026-08-18", "Public"),
    (
        "Research Findings",
        "3 findings",
        "indexed",
        "2026-08-18",
        "Public",
    ),
    (
        "Innovation Log",
        "4 entries",
        "indexed",
        "2026-08-18",
        "Public",
    ),
    (
        "Contribution Ledger",
        "23 entries",
        "indexed",
        "2026-08-18",
        "Restricted",
    ),
    ("Agreements", "2 active", "indexed", "2026-08-18", "Public"),
    (
        "Governance Settings",
        "1 config",
        "indexed",
        "2026-08-18",
        "Public",
    ),
    (
        "Selfhood Records",
        "0 records",
        "excluded",
        "n/a",
        "Selfhood",
    ),
];

const GRAPH_ENTITIES: &[(&str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "Person",
        "contributed_to, authored, influenced",
    ),
    (
        "NLP Pipeline Architecture",
        "Work",
        "depends_on, implements, extended",
    ),
    (
        "Ontology Specification",
        "Work",
        "authored, influenced, cited_by",
    ),
    (
        "FST Morphology Engine",
        "Technology",
        "implements, depends_on",
    ),
    (
        "did:qualia:contributor_02",
        "Person",
        "contributed_to, reviewed",
    ),
    (
        "SHACL Shapes Library",
        "Work",
        "authored, implements, cited_by",
    ),
    ("Benchmark Dataset", "Dataset", "depends_on, validated_by"),
];

const COVERAGE: &[(&str, &str, &str, &str)] = &[
    ("Wiki", "24/24 pages", "100%", "full"),
    ("Discussion", "12/12 threads", "100%", "full"),
    ("Decisions", "8/8 decisions", "100%", "full"),
    ("Tasks", "18/20 tasks", "90%", "partial"),
    ("Meetings", "2/2 meetings", "100%", "full"),
    ("Deliverables", "5/5 items", "100%", "full"),
    ("Research", "3/3 findings", "100%", "full"),
    ("Innovation", "4/4 entries", "100%", "full"),
    ("Contributions", "23/23 entries", "100%", "full"),
    ("Agreements", "2/2 active", "100%", "full"),
    ("Credentials", "0/3 credentials", "0%", "gap"),
    ("IP Registry", "6/6 items", "100%", "full"),
];

pub fn build_knowledge_base_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = build_tab_bar(document);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content.append_child(&build_sources_tab(document)).unwrap();

    for (i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_tab_panel(document, tab_id);
        if i > 0 {
            let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
            p_el.style().set_css_text("display: none;");
        }
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} knowledge base requires COP-X3 knowledge aggregation engine. \
         Selfhood records excluded from indexing.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_tab_bar(document: &Document) -> Element {
    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-kb-tab", tab_id).unwrap();
        tab.set_text_content(Some(tab_label));
        let t_el: HtmlElement = tab.clone().dyn_into().unwrap();
        t_el.style().set_css_text(&format!(
            "padding: 4px 10px; border: none; border-bottom: 2px solid {}; \
             background: transparent; color: {}; font-size: 10px; \
             font-family: var(--font-mono); cursor: pointer; white-space: nowrap;",
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
        tab_bar.append_child(&tab).unwrap();
    }
    tab_bar
}

fn build_sources_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-kb-panel", "sources").unwrap();

    let table = make_table(
        document,
        &["Source", "Count", "Status", "Last Indexed", "Sensitivity"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (name, count, status, last, sensitivity) in INDEXED_SOURCES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, count, status, last, sensitivity].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "indexed" => "rgba(100, 200, 100, 0.8)",
                    "excluded" => "rgba(255, 100, 100, 0.8)",
                    "pending" => "rgba(255, 165, 0, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color,
                ));
            } else if i == 4 {
                let color = match **val {
                    "Public" => "rgba(100, 200, 100, 0.8)",
                    "Restricted" => "rgba(255, 165, 0, 0.8)",
                    "Classified" => "rgba(255, 100, 100, 0.8)",
                    "Selfhood" => "rgba(255, 0, 0, 0.9)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px; \
                     font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-kb-panel", tab_id).unwrap();

    match tab_id {
        "graph" => build_graph_tab(document, &panel),
        "search" => build_search_tab(document, &panel),
        "coverage" => build_coverage_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_graph_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Knowledge graph: entities (people, organisations, concepts, works, technologies) \
         with typed relationships. Queryable via SPARQL.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["Entity", "Type", "Relationships"]);
    let tbody = document.create_element("tbody").unwrap();
    for (entity, etype, rels) in GRAPH_ENTITIES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [entity, etype, rels].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                let color = match **val {
                    "Person" => "rgba(0, 200, 255, 0.8)",
                    "Work" => "rgba(100, 200, 100, 0.8)",
                    "Technology" => "rgba(255, 165, 0, 0.8)",
                    "Dataset" => "rgba(200, 100, 255, 0.8)",
                    "Organisation" => "rgba(255, 100, 100, 0.8)",
                    "Concept" => "var(--text-primary)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px; \
                     font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();
}

fn build_search_tab(document: &Document, panel: &Element) {
    let search_box = document.create_element("input").unwrap();
    search_box.set_attribute("type", "text").unwrap();
    search_box
        .set_attribute("placeholder", "Search knowledge base...")
        .unwrap();
    let s_el: HtmlElement = search_box.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "width: 100%; padding: 6px 10px; border: 1px solid var(--border-medium); \
         border-radius: 4px; background: var(--surface-panel); color: var(--text-primary); \
         font-size: 11px; font-family: var(--font-mono); margin-bottom: 8px; \
         box-sizing: border-box;",
    );
    panel.append_child(&search_box).unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Full-text search + structured query. Faceted by type, tag, category, author, date range, sensitivity class.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let results = &[
        (
            "Ontology Specification",
            "Wiki Page",
            "did:qualia:timothy_charles_holborn",
            "2026-08-03",
        ),
        (
            "NLP Pipeline Architecture",
            "Deliverable",
            "did:qualia:timothy_charles_holborn",
            "2026-08-01",
        ),
        (
            "Ontology approach approved",
            "Decision",
            "did:qualia:reviewer_01",
            "2026-08-08",
        ),
        (
            "SHACL Shapes Library",
            "Wiki Page",
            "did:qualia:contributor_02",
            "2026-08-05",
        ),
        (
            "FST Morphology Engine",
            "Innovation",
            "did:qualia:timothy_charles_holborn",
            "2026-08-12",
        ),
    ];

    let table = make_table(document, &["Title", "Type", "Author", "Date"]);
    let tbody = document.create_element("tbody").unwrap();
    for (title, rtype, author, date) in results {
        let tr = document.create_element("tr").unwrap();
        for val in [title, rtype, author, date].iter() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                 color: var(--text-primary); font-size: 10px; \
                 font-family: var(--font-mono);",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();
}

fn build_coverage_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Coverage shows which project surfaces are indexed. Gaps indicate surfaces with no indexed records.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["Surface", "Coverage", "Percentage", "Status"]);
    let tbody = document.create_element("tbody").unwrap();
    for (surface, coverage, pct, status) in COVERAGE {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [surface, coverage, pct, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "full" => "rgba(100, 200, 100, 0.8)",
                    "partial" => "rgba(255, 165, 0, 0.8)",
                    "gap" => "rgba(255, 100, 100, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px; \
                     font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();
}

fn make_table(document: &Document, headers: &[&str]) -> Element {
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 10px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 4px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
