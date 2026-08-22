//! Clinical Reports — pathology/imaging/discharge/referral reports (§2, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("all", "All Reports"),
    ("draft", "Drafts"),
    ("confirmed", "Clinician Confirmed"),
    ("disputed", "Disputed"),
];

const REPORTS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "RPT-001",
        "Pathology: Full Blood Count",
        "2026-08-15",
        "pathology",
        "ClinicianConfirmed",
        "Dr. Chen",
    ),
    (
        "RPT-002",
        "Imaging: Chest X-Ray",
        "2026-06-20",
        "imaging",
        "ClinicianConfirmed",
        "Dr. Park",
    ),
    (
        "RPT-003",
        "Discharge Summary: ER Visit",
        "2025-03-10",
        "discharge",
        "Submitted",
        "Dr. Lee",
    ),
    (
        "RPT-004",
        "Referral: Endocrinology",
        "2026-08-01",
        "referral",
        "Draft",
        "Self",
    ),
    (
        "RPT-005",
        "Pathology: Iron Studies",
        "2026-08-15",
        "pathology",
        "Disputed",
        "Dr. Chen",
    ),
    (
        "RPT-006",
        "Imaging: Thyroid Ultrasound",
        "2024-03-20",
        "imaging",
        "Superseded",
        "Dr. Chen",
    ),
];

pub fn build_clinical_reports_view(document: &Document) -> Element {
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

    content
        .append_child(&build_reports_tab(document, "all"))
        .unwrap();

    for (_i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_reports_tab(document, tab_id);
        let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
        p_el.style().set_css_text("display: none;");
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} clinical reports require wellfare-core/clinical.rs + QECP verification.",
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
        tab.set_attribute("data-clinical-tab", tab_id).unwrap();
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

fn build_reports_tab(document: &Document, filter: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-clinical-panel", filter).unwrap();

    let table = make_table(
        document,
        &["ID", "Title", "Date", "Type", "Claim Status", "Author"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, title, date, rtype, status, author) in REPORTS {
        let show = match filter {
            "all" => true,
            "draft" => *status == "Draft",
            "confirmed" => *status == "ClinicianConfirmed",
            "disputed" => *status == "Disputed",
            _ => true,
        };
        if !show {
            continue;
        }
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, title, date, rtype, status, author].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "ClinicianConfirmed" => "rgba(100, 200, 100, 0.8)",
                    "Submitted" => "rgba(0, 200, 255, 0.8)",
                    "Draft" => "var(--text-muted)",
                    "Disputed" => "rgba(255, 100, 100, 0.8)",
                    "Superseded" => "rgba(150, 150, 150, 0.6)",
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

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ Import Report"));
    let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 6px;",
    );
    panel.append_child(&btn).unwrap();

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
