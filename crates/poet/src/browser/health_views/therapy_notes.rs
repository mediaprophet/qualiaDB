//! Therapy Notes — classified therapy session notes (§2, P1, Sanctuary only).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const NOTES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "TN-001",
        "2026-08-10",
        "Dr. Williams",
        "50 min",
        "CBT session: worked on reframing sleep anxiety. Client receptive.",
    ),
    (
        "TN-002",
        "2026-07-27",
        "Dr. Williams",
        "50 min",
        "CBT session: introduced thought records. Client engaged well.",
    ),
    (
        "TN-003",
        "2026-07-13",
        "Dr. Williams",
        "50 min",
        "Initial assessment: GAD-7=5, PHQ-9=6. Formulated treatment plan.",
    ),
    (
        "TN-004",
        "2026-06-29",
        "Dr. Williams",
        "50 min",
        "Session focused on stress management techniques.",
    ),
];

pub fn build_therapy_notes_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let banner = document.create_element("div").unwrap();
    banner.set_text_content(Some(
        "\u{1F512} CLASSIFIED \u{2014} Sanctuary-only records. \
         Access requires explicit consent. Not visible in tiered views.",
    ));
    let b_el: HtmlElement = banner.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 8px; font-size: 9px; color: rgba(200, 150, 255, 0.8); \
         font-family: var(--font-mono); background: var(--surface-panel); \
         border-radius: 4px; border: 1px solid rgba(200, 150, 255, 0.3); \
         margin: 4px 8px;",
    );
    wrapper.append_child(&banner).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    let table = make_table(
        document,
        &["ID", "Date", "Therapist", "Duration", "Summary"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, date, therapist, duration, summary) in NOTES {
        let tr = document.create_element("tr").unwrap();
        for val in [id, date, therapist, duration, summary].iter() {
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
    content.append_child(&table).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} therapy notes are Sanctuary-classified. \
         Requires wellfare-core/mental_wellbeing.rs + consent-gated access.",
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
