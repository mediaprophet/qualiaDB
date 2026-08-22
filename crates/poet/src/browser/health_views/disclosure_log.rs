//! Disclosure Log — disclosure event log with leak tracing (§2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const DISCLOSURES: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "DCL-001",
        "2026-08-15",
        "Dr. Chen (did:qualia:dr_chen)",
        "Lab Results: Iron Studies",
        "self",
        "direct",
    ),
    (
        "DCL-002",
        "2026-08-01",
        "Social Services",
        "Welfare Summary",
        "delegate: health_proxy_01",
        "tiered",
    ),
    (
        "DCL-003",
        "2026-07-20",
        "Housing Department",
        "Income Statement",
        "self",
        "direct",
    ),
    (
        "DCL-004",
        "2026-07-15",
        "Dr. Park (did:qualia:dr_park)",
        "Chest X-Ray Report",
        "self",
        "direct",
    ),
    (
        "DCL-005",
        "2026-06-01",
        "Research Study Group B",
        "Anonymised vitals aggregate",
        "self",
        "anonymised",
    ),
    (
        "DCL-006",
        "2026-08-10",
        "Insurance Provider",
        "Medical History Summary",
        "delegate: legal_proxy_01",
        "tiered",
    ),
];

pub fn build_disclosure_log_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    let table = make_table(
        document,
        &["ID", "Date", "Recipient", "Disclosed", "Acting", "Type"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, date, recipient, disclosed, acting, dtype) in DISCLOSURES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, date, recipient, disclosed, acting, dtype]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 5 {
                let color = match **val {
                    "direct" => "rgba(0, 200, 255, 0.8)",
                    "tiered" => "rgba(255, 165, 0, 0.8)",
                    "anonymised" => "rgba(100, 200, 100, 0.8)",
                    "onward-share" => "rgba(255, 100, 100, 0.8)",
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
    content.append_child(&table).unwrap();
    wrapper.append_child(&content).unwrap();

    let leak_section = document.create_element("div").unwrap();
    leak_section.set_text_content(Some(
        "Leak trace: 0 detected. Onward-share tracking: 0 flagged. \
         Tracing fingerprint: 0x4a8b2c1d9e3f5a7b (per-disclosure watermark).",
    ));
    let l_el: HtmlElement = leak_section.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "padding: 4px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); border-top: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&leak_section).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} disclosure log requires webizen-desktop/commands/wellfair/disclosure.rs.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
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
