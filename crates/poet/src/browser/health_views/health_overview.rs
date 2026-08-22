//! Health Overview dashboard — summary of all health domains (§2.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const CARDS: &[(&str, &str, &str, &str)] = &[
    (
        "Conditions",
        "2 active, 1 allergy",
        "Hypothyroidism, Vitamin D deficiency",
        "var(--text-primary)",
    ),
    (
        "Medications",
        "3 active",
        "Levothyroxine 50mcg, Vitamin D 1000IU, Iron",
        "var(--text-primary)",
    ),
    (
        "Lab Results",
        "Latest: 2026-08-15",
        "Ferritin LOW, TSH normal, Vitamin D LOW",
        "rgba(255, 165, 0, 0.8)",
    ),
    (
        "Mental Wellbeing",
        "PHQ-9: 4 (minimal)",
        "GAD-7: 3 (minimal)",
        "rgba(100, 200, 100, 0.8)",
    ),
    (
        "Vitals",
        "HR 72, BP 118/76, Temp 36.6",
        "All within normal range",
        "rgba(100, 200, 100, 0.8)",
    ),
    (
        "Sleep",
        "7.2h avg, debt: 2.1h",
        "Chronic debt flag: no",
        "var(--text-primary)",
    ),
    (
        "Diet",
        "1,850 kcal today",
        "Protein 72g, Carbs 210g, Fat 55g",
        "var(--text-primary)",
    ),
    (
        "Hypotheses",
        "3 pending",
        "Top: Iron deficiency anaemia (0.72)",
        "rgba(0, 200, 255, 0.8)",
    ),
    (
        "Documents",
        "12 records",
        "3 QECP-verified, 9 pending",
        "var(--text-primary)",
    ),
    (
        "Welfare",
        "1 active stream",
        "Housing assistance: in progress",
        "var(--text-primary)",
    ),
];

pub fn build_health_overview_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         display: flex; justify-content: space-between; align-items: center;",
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some(
        "Health Overview \u{2014} did:qualia:timothy_charles_holborn",
    ));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 11px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    h_el.append_child(&title).unwrap();

    let refresh = document.create_element("button").unwrap();
    refresh.set_text_content(Some("Refresh"));
    let r_el: HtmlElement = refresh.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "padding: 2px 8px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 9px; font-family: var(--font-mono);",
    );
    h_el.append_child(&refresh).unwrap();
    wrapper.append_child(&header).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style()
        .set_css_text("display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px;");

    for (title, summary, detail, color) in CARDS {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(
            "border: 1px solid var(--border-subtle); border-radius: 6px; \
             padding: 8px; background: var(--surface-panel);",
        );

        let t = document.create_element("div").unwrap();
        t.set_text_content(Some(title));
        let t_el: HtmlElement = t.clone().dyn_into().unwrap();
        t_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--text-muted); \
             font-family: var(--font-mono); text-transform: uppercase; \
             margin-bottom: 4px;",
        );
        card.append_child(&t).unwrap();

        let s = document.create_element("div").unwrap();
        s.set_text_content(Some(summary));
        let s_el: HtmlElement = s.clone().dyn_into().unwrap();
        s_el.style().set_css_text(&format!(
            "font-size: 11px; font-weight: 600; color: {}; \
             font-family: var(--font-mono);",
            color,
        ));
        card.append_child(&s).unwrap();

        let d = document.create_element("div").unwrap();
        d.set_text_content(Some(detail));
        let d_el: HtmlElement = d.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); \
             font-family: var(--font-mono); margin-top: 2px;",
        );
        card.append_child(&d).unwrap();

        grid.append_child(&card).unwrap();
    }

    content.append_child(&grid).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} health overview requires wellfare-core engine + consent-gated telemetry.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
