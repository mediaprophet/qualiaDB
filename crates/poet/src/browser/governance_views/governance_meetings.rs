//! Governance Meetings — meeting scheduler, minutes & resolutions (§8g).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::disputes::{build_tab_bar, make_table};

const TABS: &[(&str, &str)] = &[
    ("upcoming", "Upcoming"),
    ("minutes", "Minutes"),
    ("resolutions", "Resolutions"),
    ("settings", "Voting Settings"),
];

const MEETINGS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "MTG-003",
        "2026-08-22 14:00 UTC",
        "standing",
        "virtual",
        "3 of 5",
        "scheduled",
    ),
    (
        "MTG-004",
        "2026-08-29 14:00 UTC",
        "special",
        "virtual",
        "TBD",
        "scheduled",
    ),
    (
        "MTG-005",
        "2026-09-05 14:00 UTC",
        "standing",
        "hybrid",
        "TBD",
        "scheduled",
    ),
];

const MINUTES: &[(&str, &str, &str, &str)] = &[
    ("MTG-001", "2026-08-01", "3 of 3", "carried"),
    ("MTG-002", "2026-08-08", "2 of 3", "carried"),
];

const RESOLUTIONS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "RES-001",
        "Project scope approved",
        "did:qualia:timothy_charles_holborn",
        "3-0-0",
        "carried",
        "2026-08-01",
    ),
    (
        "RES-002",
        "Ontology approach approved",
        "did:qualia:reviewer_01",
        "2-1-0",
        "carried",
        "2026-08-08",
    ),
    (
        "RES-003",
        "SHACL revision requested",
        "did:qualia:reviewer_02",
        "1-1-1",
        "lost",
        "2026-08-08",
    ),
];

const VOTING_SETTINGS: &[(&str, &str, &str, &str)] = &[
    ("routine", "simple_majority", "24h", "recorded"),
    ("significant", "supermajority_2_3", "7d", "public"),
    ("constitutional", "unanimous", "30d", "public"),
    ("emergency", "simple_majority", "1h", "recorded"),
];

pub fn build_governance_meetings_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = build_tab_bar(document, "govmeet-tab", TABS);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content.append_child(&build_upcoming_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} governance meetings require COP-A3 governance engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_upcoming_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-govmeet-panel", "upcoming")
        .unwrap();

    let table = make_table(
        document,
        &["ID", "Date/Time", "Type", "Location", "Quorum", "Status"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, datetime, mtype, location, quorum, status) in MEETINGS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, datetime, mtype, location, quorum, status]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "standing" => "rgba(0, 200, 255, 0.8)",
                    "special" => "rgba(255, 165, 0, 0.8)",
                    "emergency" => "rgba(255, 100, 100, 0.8)",
                    "annual" => "rgba(100, 200, 100, 0.8)",
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
    add_btn.set_text_content(Some("+ Schedule Meeting"));
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
    panel.set_attribute("data-govmeet-panel", tab_id).unwrap();

    match tab_id {
        "minutes" => build_minutes_tab(document, &panel),
        "resolutions" => build_resolutions_tab(document, &panel),
        "settings" => build_settings_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_minutes_tab(document: &Document, panel: &Element) {
    let table = make_table(document, &["Meeting", "Date", "Attendance", "Outcome"]);
    let tbody = document.create_element("tbody").unwrap();
    for (id, date, attendance, outcome) in MINUTES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, date, attendance, outcome].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "carried" => "rgba(100, 200, 100, 0.8)",
                    "adjourned" => "rgba(255, 165, 0, 0.8)",
                    "cancelled" => "rgba(255, 100, 100, 0.8)",
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

fn build_resolutions_tab(document: &Document, panel: &Element) {
    let table = make_table(
        document,
        &["ID", "Text", "Moved By", "Vote (Y-N-A)", "Result", "Date"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, text, moved_by, vote, result, date) in RESOLUTIONS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, text, moved_by, vote, result, date].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "carried" => "rgba(100, 200, 100, 0.8)",
                    "lost" => "rgba(255, 100, 100, 0.8)",
                    "tied" => "rgba(255, 165, 0, 0.8)",
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

fn build_settings_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Voting systems per decision category. Transparency: public, recorded, anonymous, zk.",
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
        &["Category", "Voting System", "Notice Period", "Transparency"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (category, system, notice, transparency) in VOTING_SETTINGS {
        let tr = document.create_element("tr").unwrap();
        for val in [category, system, notice, transparency].iter() {
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
