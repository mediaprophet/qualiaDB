//! Documents — health document library with QECP status (§6, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("library", "Document Library"),
    ("qecp", "QECP-Verified"),
    ("pending", "Pending NLP"),
];

const DOCS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "Full Blood Count 2026-08-15",
        "PDF",
        "QECP-verified",
        "2026-08-15",
        "Pathology Lab",
        "Dr. Chen",
    ),
    (
        "Chest X-Ray Report 2026-06-20",
        "PDF",
        "QECP-verified",
        "2026-06-20",
        "Imaging Centre",
        "Dr. Park",
    ),
    (
        "Discharge Summary 2025-03-10",
        "PDF",
        "pending-nlp",
        "2025-03-10",
        "Hospital ER",
        "Dr. Lee",
    ),
    (
        "Referral Letter Endocrinology",
        "PDF",
        "pending-nlp",
        "2026-08-01",
        "Self",
        "Self",
    ),
    (
        "Iron Studies 2026-08-15",
        "PDF",
        "QECP-verified",
        "2026-08-15",
        "Pathology Lab",
        "Dr. Chen",
    ),
    (
        "Thyroid Ultrasound 2024-03-20",
        "DICOM",
        "pending-nlp",
        "2024-03-20",
        "Imaging Centre",
        "Dr. Chen",
    ),
    (
        "Vitamin D Results 2025-11-20",
        "PDF",
        "pending-nlp",
        "2025-11-20",
        "Pathology Lab",
        "Dr. Chen",
    ),
    (
        "Sleep Study Report 2025-06-15",
        "PDF",
        "pending-nlp",
        "2025-06-15",
        "Sleep Clinic",
        "Dr. Park",
    ),
];

pub fn build_documents_view(document: &Document) -> Element {
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
        .append_child(&build_docs_tab(document, "library"))
        .unwrap();

    for (_i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_docs_tab(document, tab_id);
        let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
        p_el.style().set_css_text("display: none;");
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} documents require DOC-20..DOC-21 + NLP extraction + QECP parsing.",
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
        tab.set_attribute("data-docs-tab", tab_id).unwrap();
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

fn build_docs_tab(document: &Document, filter: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-docs-panel", filter).unwrap();

    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style()
        .set_css_text("display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px;");

    for (name, fmt, qecp_status, date, source, author) in DOCS {
        let show = match filter {
            "library" => true,
            "qecp" => *qecp_status == "QECP-verified",
            "pending" => *qecp_status == "pending-nlp",
            _ => true,
        };
        if !show {
            continue;
        }

        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        let border = if *qecp_status == "QECP-verified" {
            "rgba(100, 200, 100, 0.3)"
        } else {
            "rgba(255, 165, 0, 0.3)"
        };
        c_el.style().set_css_text(&format!(
            "border: 1px solid {}; border-radius: 6px; padding: 8px; \
             background: var(--surface-panel);",
            border,
        ));

        let icon = match *fmt {
            "PDF" => "\u{1F4D1}",
            "DICOM" => "\u{1F4C7}",
            "DOCX" => "\u{1F4C2}",
            "Image" => "\u{1F5BC}",
            _ => "\u{1F4C4}",
        };

        let n = document.create_element("div").unwrap();
        n.set_text_content(Some(&format!("{} {}", icon, name)));
        let n_el: HtmlElement = n.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono); margin-bottom: 4px;",
        );
        card.append_child(&n).unwrap();

        let meta = document.create_element("div").unwrap();
        meta.set_text_content(Some(&format!("{}  |  {}  |  {}", fmt, date, source)));
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&meta).unwrap();

        let badge = document.create_element("div").unwrap();
        badge.set_text_content(Some(qecp_status));
        let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
        let badge_color = if *qecp_status == "QECP-verified" {
            "rgba(100, 200, 100, 0.8)"
        } else {
            "rgba(255, 165, 0, 0.8)"
        };
        b_el.style().set_css_text(&format!(
            "font-size: 8px; color: {}; font-family: var(--font-mono); \
             font-weight: 600; text-transform: uppercase; margin-top: 4px;",
            badge_color,
        ));
        card.append_child(&badge).unwrap();

        let author_div = document.create_element("div").unwrap();
        author_div.set_text_content(Some(author));
        let a_el: HtmlElement = author_div.clone().dyn_into().unwrap();
        a_el.style().set_css_text(
            "font-size: 8px; color: var(--accent-cyan); font-family: var(--font-mono); \
             margin-top: 2px;",
        );
        card.append_child(&author_div).unwrap();

        grid.append_child(&card).unwrap();
    }

    panel.append_child(&grid).unwrap();

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ Import Document"));
    let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 6px;",
    );
    panel.append_child(&btn).unwrap();

    panel
}
