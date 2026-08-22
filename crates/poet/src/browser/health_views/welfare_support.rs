//! Welfare Support — assistance needs + welfare streams + government letters (§2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("needs", "Assistance Needs"),
    ("streams", "Welfare Streams"),
    ("letters", "Government Letters"),
];

const NEEDS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "ASN-001",
        "Housing assistance",
        "2026-07-01",
        "in_progress",
        "high",
    ),
    (
        "ASN-002",
        "Medical expense support",
        "2026-08-01",
        "submitted",
        "medium",
    ),
    ("ASN-003", "Food security", "2026-06-15", "resolved", "high"),
];

const STREAMS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "WS-001",
        "Housing Support Program",
        "2026-07-01",
        "active",
        "300 XEC/mo",
    ),
    (
        "WS-002",
        "Healthcare Subsidy",
        "2026-01-01",
        "active",
        "150 XEC/mo",
    ),
    (
        "WS-003",
        "Emergency Relief",
        "2026-03-15",
        "completed",
        "500 XEC one-off",
    ),
];

const LETTERS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "LTR-001",
        "Housing Application Letter",
        "2026-07-01",
        "Department of Housing",
        "sent",
    ),
    (
        "LTR-002",
        "Medical Hardship Declaration",
        "2026-08-05",
        "Health Services",
        "draft",
    ),
    (
        "LTR-003",
        "Income Statement Request",
        "2026-06-20",
        "Tax Office",
        "sent",
    ),
    (
        "LTR-004",
        "Disability Support Letter",
        "2026-05-10",
        "Social Services",
        "acknowledged",
    ),
];

pub fn build_welfare_support_view(document: &Document) -> Element {
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
        .append_child(&build_tab(
            document,
            "needs",
            &["ID", "Need", "Filed", "Status", "Priority"],
            NEEDS,
        ))
        .unwrap();

    for (i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let (headers, data) = match *tab_id {
            "streams" => (
                &["ID", "Stream", "Started", "Status", "Amount"][..],
                STREAMS,
            ),
            "letters" => (
                &["ID", "Letter", "Date", "Recipient", "Status"][..],
                LETTERS,
            ),
            _ => (&["ID", "Need", "Filed", "Status", "Priority"][..], NEEDS),
        };
        let panel = build_tab(document, tab_id, headers, data);
        let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
        p_el.style().set_css_text("display: none;");
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} welfare support requires wellfare-core/welfare_support.rs.",
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
        tab.set_attribute("data-welfare-tab", tab_id).unwrap();
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

fn build_tab(
    document: &Document,
    tab_id: &str,
    headers: &[&str],
    data: &[(&str, &str, &str, &str, &str)],
) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-welfare-panel", tab_id).unwrap();

    let table = make_table(document, headers);
    let tbody = document.create_element("tbody").unwrap();
    for (c1, c2, c3, c4, c5) in data {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [c1, c2, c3, c4, c5].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "active" | "resolved" | "acknowledged" | "completed" => {
                        "rgba(100, 200, 100, 0.8)"
                    }
                    "in_progress" | "submitted" | "sent" => "rgba(0, 200, 255, 0.8)",
                    "draft" | "pending" => "var(--text-muted)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color,
                ));
            } else if i == 4 && tab_id == "needs" {
                let color = match **val {
                    "high" => "rgba(255, 100, 100, 0.8)",
                    "medium" => "rgba(255, 165, 0, 0.8)",
                    "low" => "rgba(100, 200, 100, 0.8)",
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
