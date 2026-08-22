//! Conflict of Interest — COI register & management (§8h).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::disputes::{build_tab_bar, make_table};

const TABS: &[(&str, &str)] = &[
    ("register", "COI Register"),
    ("recusals", "Recusals"),
    ("standing", "Standing Declarations"),
];

const DECLARATIONS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "COI-001",
        "did:qualia:reviewer_01",
        "financial",
        "supplier_selection",
        "actual",
        "disclosure_only",
    ),
    (
        "COI-002",
        "did:qualia:contributor_02",
        "personal",
        "deliverable_review",
        "perceived",
        "recusal",
    ),
    (
        "COI-003",
        "did:qualia:reviewer_02",
        "institutional",
        "budget_approval",
        "potential",
        "managed_participation",
    ),
];

const RECUSALS: &[(&str, &str, &str, &str)] = &[
    (
        "COI-002",
        "did:qualia:contributor_02",
        "deliverable_review",
        "2026-08-14",
    ),
    (
        "COI-004",
        "did:qualia:reviewer_01",
        "budget_vote_q3",
        "2026-08-10",
    ),
];

const STANDING: &[(&str, &str, &str, &str)] = &[
    (
        "COI-001",
        "did:qualia:reviewer_01",
        "Employed by Org X (supplier to project) — applies to all supplier-selection decisions",
        "active",
    ),
    (
        "COI-005",
        "did:qualia:contributor_03",
        "Board member of Partner Org Y — applies to partnership decisions",
        "active",
    ),
];

pub fn build_conflict_of_interest_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = build_tab_bar(document, "coi-tab", TABS);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content.append_child(&build_register_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} COI management requires COP-A3 governance engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_register_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-coi-panel", "register").unwrap();

    let table = make_table(
        document,
        &[
            "ID",
            "Agent",
            "Nature",
            "Affected Area",
            "Severity",
            "Management",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, agent, nature, area, severity, management) in DECLARATIONS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, agent, nature, area, severity, management]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "actual" => "rgba(255, 100, 100, 0.8)",
                    "perceived" => "rgba(255, 165, 0, 0.8)",
                    "potential" => "rgba(0, 200, 255, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color,
                ));
            } else if i == 5 {
                let color = match **val {
                    "recusal" => "rgba(255, 100, 100, 0.8)",
                    "disclosure_only" => "rgba(0, 200, 255, 0.8)",
                    "managed_participation" => "rgba(255, 165, 0, 0.8)",
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
    add_btn.set_text_content(Some("+ Declare COI"));
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
    panel.set_attribute("data-coi-panel", tab_id).unwrap();

    match tab_id {
        "recusals" => build_recusals_tab(document, &panel),
        "standing" => build_standing_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_recusals_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Recused agents cannot vote, comment, or influence the affected decision.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["COI", "Agent", "Decision", "Date"]);
    let tbody = document.create_element("tbody").unwrap();
    for (id, agent, decision, date) in RECUSALS {
        let tr = document.create_element("tr").unwrap();
        for val in [id, agent, decision, date].iter() {
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

fn build_standing_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Standing declarations apply to all future decisions in a category. Annual review required.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["ID", "Agent", "Declaration", "Status"]);
    let tbody = document.create_element("tbody").unwrap();
    for (id, agent, declaration, status) in STANDING {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, agent, declaration, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "active" => "rgba(100, 200, 100, 0.8)",
                    "retired" => "var(--text-muted)",
                    "resolved" => "rgba(0, 200, 255, 0.8)",
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
