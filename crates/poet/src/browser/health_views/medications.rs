//! Medications — medication catalog + administration log + adherence (§2, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("active", "Active Medications"),
    ("log", "Administration Log"),
];

const MEDS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "Levothyroxine",
        "50mcg",
        "oral",
        "daily morning",
        "Dr. Chen",
        "active",
    ),
    (
        "Vitamin D3",
        "1000IU",
        "oral",
        "daily with food",
        "Dr. Chen",
        "active",
    ),
    (
        "Ferrous Sulfate",
        "325mg",
        "oral",
        "3x daily with food",
        "Dr. Chen",
        "active",
    ),
    (
        "Ibuprofen",
        "200mg",
        "oral",
        "as needed",
        "Self",
        "inactive",
    ),
];

const LOG: &[(&str, &str, &str, &str)] = &[
    (
        "Levothyroxine 50mcg",
        "2026-08-18 07:30",
        "taken",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Vitamin D3 1000IU",
        "2026-08-18 07:35",
        "taken",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Ferrous Sulfate 325mg",
        "2026-08-18 08:00",
        "taken",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Ferrous Sulfate 325mg",
        "2026-08-18 13:00",
        "skipped",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Ferrous Sulfate 325mg",
        "2026-08-18 19:00",
        "overdue",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Levothyroxine 50mcg",
        "2026-08-17 07:30",
        "taken",
        "did:qualia:timothy_charles_holborn",
    ),
];

pub fn build_medications_view(document: &Document) -> Element {
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

    content.append_child(&build_meds_tab(document)).unwrap();

    let log_panel = build_log_tab(document);
    let lp_el: HtmlElement = log_panel.clone().dyn_into().unwrap();
    lp_el.style().set_css_text("display: none;");
    content.append_child(&log_panel).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} medications require wellfare-core/medication.rs + HW-9 adherence engine.",
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
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-meds-tab", tab_id).unwrap();
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

fn build_meds_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-meds-panel", "active").unwrap();

    let adherence = document.create_element("div").unwrap();
    adherence.set_text_content(Some("Adherence rate (30d): 87% \u{2014} 52/60 doses taken"));
    let a_el: HtmlElement = adherence.clone().dyn_into().unwrap();
    a_el.style().set_css_text(
        "padding: 6px 8px; font-size: 10px; color: var(--text-primary); \
         font-family: var(--font-mono); background: var(--surface-panel); \
         border-radius: 4px; margin-bottom: 6px;",
    );
    panel.append_child(&adherence).unwrap();

    let table = make_table(
        document,
        &[
            "Medication",
            "Dose",
            "Route",
            "Schedule",
            "Prescribed By",
            "Status",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (name, dose, route, schedule, prescribed_by, status) in MEDS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, dose, route, schedule, prescribed_by, status]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 5 {
                let color = match **val {
                    "active" => "rgba(100, 200, 100, 0.8)",
                    "inactive" => "var(--text-muted)",
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

fn build_log_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-meds-panel", "log").unwrap();

    let table = make_table(
        document,
        &["Medication", "Timestamp", "Status", "Logged By"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (med, ts, status, logged_by) in LOG {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [med, ts, status, logged_by].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "taken" => "rgba(100, 200, 100, 0.8)",
                    "skipped" => "rgba(255, 165, 0, 0.8)",
                    "overdue" => "rgba(255, 100, 100, 0.8)",
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
