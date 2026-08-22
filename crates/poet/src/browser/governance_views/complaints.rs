//! Complaints — complaint registry & investigation (§8e.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::disputes::{build_tab_bar, make_table};

const TABS: &[(&str, &str)] = &[
    ("filed", "Filed Complaints"),
    ("investigations", "Investigations"),
    ("sanctions", "Sanctions"),
    ("appeals", "Appeals"),
];

const COMPLAINTS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "CMP-001",
        "conduct",
        "did:qualia:contributor_02",
        "did:qualia:reviewer_01",
        "filed",
        "2026-08-12",
    ),
    (
        "CMP-002",
        "harassment",
        "anonymous",
        "did:qualia:contributor_03",
        "investigating",
        "2026-08-14",
    ),
    (
        "CMP-003",
        "quality",
        "did:qualia:reviewer_02",
        "did:qualia:contributor_04",
        "filed",
        "2026-08-16",
    ),
];

const INVESTIGATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "CMP-002",
        "did:qualia:reviewer_01",
        "evidence_collection",
        "2026-08-15",
    ),
    (
        "CMP-001",
        "did:qualia:timothy_charles_holborn",
        "panel_review",
        "2026-08-14",
    ),
];

const SANCTIONS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "CMP-000",
        "did:qualia:contributor_05",
        "warning",
        "2026-08-01",
        "active",
    ),
    (
        "CMP-001",
        "did:qualia:contributor_02",
        "suspension",
        "2026-08-13",
        "active",
    ),
];

const APPEALS: &[(&str, &str, &str, &str, &str)] = &[(
    "CMP-000",
    "did:qualia:contributor_05",
    "filed",
    "2026-08-05",
    "pending",
)];

pub fn build_complaints_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = build_tab_bar(document, "complaints-tab", TABS);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content.append_child(&build_filed_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} complaint handling requires COP-A3 governance engine command. \
         Whistleblower protection enforced via governance policy.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_filed_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-complaints-panel", "filed")
        .unwrap();

    let table = make_table(
        document,
        &["ID", "Category", "Filed By", "Against", "Status", "Date"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, category, filed_by, against, status, date) in COMPLAINTS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, category, filed_by, against, status, date]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "filed" => "rgba(255, 100, 100, 0.8)",
                    "investigating" => "rgba(255, 165, 0, 0.8)",
                    "resolved" => "rgba(100, 200, 100, 0.8)",
                    "dismissed" => "var(--text-muted)",
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

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ File Complaint"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-complaints-panel", tab_id)
        .unwrap();

    match tab_id {
        "investigations" => build_investigations_tab(document, &panel),
        "sanctions" => build_sanctions_tab(document, &panel),
        "appeals" => build_appeals_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_investigations_tab(document: &Document, panel: &Element) {
    let table = make_table(document, &["Complaint", "Investigator", "Stage", "Started"]);
    let tbody = document.create_element("tbody").unwrap();
    for (id, investigator, stage, date) in INVESTIGATIONS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, investigator, stage, date].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "evidence_collection" => "rgba(255, 165, 0, 0.8)",
                    "panel_review" => "rgba(0, 200, 255, 0.8)",
                    "findings_drafted" => "rgba(100, 200, 100, 0.8)",
                    "closed" => "var(--text-muted)",
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

fn build_sanctions_tab(document: &Document, panel: &Element) {
    let table = make_table(
        document,
        &["Complaint", "Agent", "Sanction Type", "Date", "Status"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, agent, sanction, date, status) in SANCTIONS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, agent, sanction, date, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "active" => "rgba(255, 100, 100, 0.8)",
                    "expired" => "var(--text-muted)",
                    "overturned" => "rgba(100, 200, 100, 0.8)",
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

fn build_appeals_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Appeals are reviewed by a separate body (governance-configured). \
         Outcomes: upheld, overturned, modified.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(
        document,
        &["Complaint", "Appellant", "Status", "Date", "Outcome"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, appellant, status, date, outcome) in APPEALS {
        let tr = document.create_element("tr").unwrap();
        for val in [id, appellant, status, date, outcome].iter() {
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
