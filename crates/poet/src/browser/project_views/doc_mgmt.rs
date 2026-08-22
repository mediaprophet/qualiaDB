//! Document Management — centralized document registry (§2.2.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("registry", "Registry"),
    ("versions", "Version Chains"),
    ("access", "Access Control"),
];

const DOCS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "Ontology Spec v3",
        "spec",
        "CC-BY-SA",
        "public",
        "v3",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "NLP Architecture",
        "spec",
        "CC-BY-SA",
        "public",
        "v2",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Contributor Agreement",
        "contract",
        "restricted",
        "members",
        "v1",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "SHACL Shapes",
        "spec",
        "CC-BY",
        "public",
        "v1",
        "did:qualia:contributor_02",
    ),
    (
        "Funding Proposal",
        "report",
        "restricted",
        "governance",
        "v1",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Legal Review",
        "legal",
        "restricted",
        "governance",
        "v1",
        "did:qualia:contributor_02",
    ),
    (
        "Research Findings",
        "report",
        "CC-BY",
        "public",
        "v1",
        "did:qualia:contributor_03",
    ),
];

const VERSIONS: &[(&str, &str, &str, &str)] = &[
    (
        "Ontology Spec",
        "v3",
        "2026-08-03",
        "Added SHACL constraints",
    ),
    ("Ontology Spec", "v2", "2026-07-28", "Refined N3 examples"),
    ("Ontology Spec", "v1", "2026-07-15", "Initial draft"),
    (
        "NLP Architecture",
        "v2",
        "2026-08-01",
        "Added FST engine section",
    ),
    ("NLP Architecture", "v1", "2026-07-20", "Initial draft"),
    (
        "Contributor Agreement",
        "v1",
        "2026-07-01",
        "Initial version",
    ),
];

const ACCESS: &[(&str, &str, &str)] = &[
    ("Ontology Spec v3", "public", "all"),
    ("Contributor Agreement", "restricted", "members only"),
    ("Funding Proposal", "restricted", "governance only"),
    ("Legal Review", "restricted", "governance only"),
    ("SHACL Shapes", "public", "all"),
    ("Research Findings", "public", "all"),
];

pub fn build_doc_mgmt_view(document: &Document) -> Element {
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

    content.append_child(&build_registry_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} document management requires COP-X5 ArtifactAttachment + COP-R3 licensing engine.",
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
        tab.set_attribute("data-docmgr-tab", tab_id).unwrap();
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

fn build_registry_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-docmgr-panel", "registry")
        .unwrap();

    let table = make_table(
        document,
        &[
            "Title",
            "Kind",
            "License",
            "Sensitivity",
            "Version",
            "Author",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (title, kind, license, sensitivity, ver, author) in DOCS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [title, kind, license, sensitivity, ver, author]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "public" => "rgba(100, 200, 100, 0.8)",
                    "restricted" => "rgba(255, 165, 0, 0.8)",
                    "confidential" => "rgba(255, 100, 100, 0.8)",
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
    panel.set_attribute("data-docmgr-panel", tab_id).unwrap();

    match tab_id {
        "versions" => {
            let table = make_table(document, &["Document", "Version", "Date", "Change"]);
            let tbody = document.create_element("tbody").unwrap();
            for (doc, ver, date, change) in VERSIONS {
                let tr = document.create_element("tr").unwrap();
                for val in [doc, ver, date, change].iter() {
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
        "access" => {
            let table = make_table(document, &["Document", "Class", "Access"]);
            let tbody = document.create_element("tbody").unwrap();
            for (doc, class, access) in ACCESS {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [doc, class, access].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 1 {
                        let color = match **val {
                            "public" => "rgba(100, 200, 100, 0.8)",
                            "restricted" => "rgba(255, 165, 0, 0.8)",
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
        _ => {}
    }

    panel
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
