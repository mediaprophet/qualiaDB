//! Wiki — structured document tree with provenance (§2.2.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("pages", "Pages"),
    ("categories", "Categories"),
    ("history", "Version History"),
];

const PAGES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Ontology Specification",
        "Design",
        "did:qualia:timothy_charles_holborn",
        "2026-08-03",
        "v3",
    ),
    (
        "NLP Pipeline Architecture",
        "Design",
        "did:qualia:timothy_charles_holborn",
        "2026-08-01",
        "v2",
    ),
    (
        "SHACL Shapes Library",
        "Design",
        "did:qualia:contributor_02",
        "2026-08-05",
        "v1",
    ),
    (
        "FST Morphology Engine",
        "Design",
        "did:qualia:timothy_charles_holborn",
        "2026-08-12",
        "v1",
    ),
    (
        "Contributor Guide",
        "Meta",
        "did:qualia:contributor_02",
        "2026-07-20",
        "v4",
    ),
    (
        "Governance Policy",
        "Meta",
        "did:qualia:timothy_charles_holborn",
        "2026-07-15",
        "v2",
    ),
    (
        "Meeting Minutes 2026-08-17",
        "Minutes",
        "did:qualia:contributor_02",
        "2026-08-17",
        "v1",
    ),
    (
        "Research Findings Q3",
        "Research",
        "did:qualia:contributor_03",
        "2026-08-10",
        "v1",
    ),
];

const CATEGORIES: &[(&str, &str)] = &[
    ("Design", "4 pages"),
    ("Meta", "2 pages"),
    ("Minutes", "1 page"),
    ("Research", "1 page"),
    ("Decisions", "0 pages"),
    ("Reports", "0 pages"),
];

const HISTORY: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Ontology Specification",
        "v3",
        "did:qualia:timothy_charles_holborn",
        "2026-08-03",
        "Added SHACL constraints section",
    ),
    (
        "Ontology Specification",
        "v2",
        "did:qualia:timothy_charles_holborn",
        "2026-07-28",
        "Refined N3 examples",
    ),
    (
        "Ontology Specification",
        "v1",
        "did:qualia:timothy_charles_holborn",
        "2026-07-15",
        "Initial draft",
    ),
    (
        "Contributor Guide",
        "v4",
        "did:qualia:contributor_02",
        "2026-07-20",
        "Added onboarding section",
    ),
    (
        "Contributor Guide",
        "v3",
        "did:qualia:contributor_02",
        "2026-07-10",
        "Updated compensation section",
    ),
];

pub fn build_wiki_view(document: &Document) -> Element {
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

    content.append_child(&build_pages_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} wiki requires COP-X1 extended artefact engine. \
         Pages are append-only with predecessor chain provenance.",
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
        tab.set_attribute("data-wiki-tab", tab_id).unwrap();
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

fn build_pages_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-wiki-panel", "pages").unwrap();

    let table = make_table(
        document,
        &["Title", "Category", "Author", "Last Edit", "Version"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (title, cat, author, date, ver) in PAGES {
        let tr = document.create_element("tr").unwrap();
        for val in [title, cat, author, date, ver].iter() {
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

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ New Page"));
    let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 6px;",
    );
    panel.append_child(&btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-wiki-panel", tab_id).unwrap();

    match tab_id {
        "categories" => build_categories_tab(document, &panel),
        "history" => build_history_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_categories_tab(document: &Document, panel: &Element) {
    let table = make_table(document, &["Category", "Pages"]);
    let tbody = document.create_element("tbody").unwrap();
    for (name, count) in CATEGORIES {
        let tr = document.create_element("tr").unwrap();
        for val in [name, count].iter() {
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

fn build_history_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Version history is append-only. Each version links to its predecessor. \
         Original is always visible. Corrections create new versions, not overwrites.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["Page", "Version", "Author", "Date", "Change"]);
    let tbody = document.create_element("tbody").unwrap();
    for (page, ver, author, date, change) in HISTORY {
        let tr = document.create_element("tr").unwrap();
        for val in [page, ver, author, date, change].iter() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wiki_pages_not_empty() {
        assert!(!PAGES.is_empty());
        for (title, cat, author, date, ver) in PAGES {
            assert!(!title.is_empty());
            assert!(!cat.is_empty());
            assert!(!author.is_empty());
            assert!(!date.is_empty());
            assert!(!ver.is_empty());
        }
    }

    #[test]
    fn test_wiki_tabs_complete() {
        assert_eq!(TABS.len(), 3);
        assert_eq!(TABS[0].0, "pages");
        assert_eq!(TABS[1].0, "categories");
        assert_eq!(TABS[2].0, "history");
    }
}
