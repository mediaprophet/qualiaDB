//! Work Items Kanban — 7-column board with mock cards.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const COLUMNS: &[(&str, &str)] = &[
    ("proposed", "Proposed"),
    ("todo", "Todo"),
    ("in_progress", "In Progress"),
    ("blocked", "Blocked"),
    ("in_review", "In Review"),
    ("done", "Done"),
    ("cancelled", "Cancelled"),
];

const MOCK_CARDS: &[(&str, &str, &str, &str, &str)] = &[
    // (title, column, type_icon, priority, assignee)
    (
        "Draft project charter",
        "proposed",
        "\u{1F4DD}",
        "P1",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Define MVP scope",
        "proposed",
        "\u{1F3AF}",
        "P0",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Set up repository",
        "todo",
        "\u{1F4BB}",
        "P0",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Write ontology spec",
        "todo",
        "\u{1F4D6}",
        "P1",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Implement NLP pipeline",
        "in_progress",
        "\u{1F9EC}",
        "P0",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "SHACL validation pass",
        "in_review",
        "\u{2705}",
        "P1",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Initial release",
        "done",
        "\u{1F389}",
        "P0",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Awaiting daemon API",
        "blocked",
        "\u{23F3}",
        "P0",
        "did:qualia:timothy_charles_holborn",
    ),
];

pub fn build_kanban_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    // Filter bar
    let filter_bar = document.create_element("div").unwrap();
    let fb_el: HtmlElement = filter_bar.clone().dyn_into().unwrap();
    fb_el.style().set_css_text(
        "display: flex; gap: 8px; padding: 4px 8px; font-size: 10px; \
         color: var(--text-muted); font-family: var(--font-mono);",
    );
    for (label, _) in &[
        ("All", ""),
        ("My items", ""),
        ("By phase", ""),
        ("By priority", ""),
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 8px; border: 1px solid var(--border-subtle); \
             background: transparent; color: var(--text-secondary); \
             border-radius: 3px; cursor: pointer; font-size: 10px;",
        );
        filter_bar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&filter_bar).unwrap();

    // Board
    let board = document.create_element("div").unwrap();
    let board_el: HtmlElement = board.clone().dyn_into().unwrap();
    board_el.style().set_css_text(
        "display: flex; gap: 4px; flex: 1; overflow-x: auto; \
         padding: 0 4px;",
    );

    for (col_id, col_label) in COLUMNS {
        let col = document.create_element("div").unwrap();
        col.set_class_name(&format!("kanban-col kanban-{}", col_id));
        col.set_attribute("data-column", col_id).unwrap();
        let c_el: HtmlElement = col.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "display: flex; flex-direction: column; gap: 4px; \
             min-width: 140px; flex: 1; padding: 4px; \
             background: var(--surface-panel); border-radius: 4px; \
             border: 1px solid var(--border-subtle);",
        );

        // Column header
        let header = document.create_element("div").unwrap();
        let h_el: HtmlElement = header.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; text-transform: uppercase; \
             letter-spacing: 0.5px; color: var(--text-muted); \
             padding: 2px 4px; border-bottom: 1px solid var(--border-subtle);",
        );
        let count = MOCK_CARDS
            .iter()
            .filter(|(_, c, _, _, _)| c == col_id)
            .count();
        h_el.set_text_content(Some(&format!("{} ({})", col_label, count)));
        col.append_child(&header).unwrap();

        // Cards
        for (title, card_col, icon, priority, assignee) in MOCK_CARDS {
            if card_id(card_col) != *col_id {
                continue;
            }
            let card = document.create_element("div").unwrap();
            card.set_class_name("kanban-card");
            card.set_attribute("data-card-title", title).unwrap();
            let card_el: HtmlElement = card.clone().dyn_into().unwrap();
            card_el.style().set_css_text(
                "padding: 6px 8px; background: var(--surface-panel-elevated); \
                 border: 1px solid var(--border-medium); border-radius: 4px; \
                 cursor: grab; font-size: 11px; display: flex; \
                 flex-direction: column; gap: 3px;",
            );

            let card_top = document.create_element("div").unwrap();
            let ct_el: HtmlElement = card_top.clone().dyn_into().unwrap();
            ct_el
                .style()
                .set_css_text("display: flex; align-items: center; gap: 4px;");

            let icon_span = document.create_element("span").unwrap();
            icon_span.set_text_content(Some(icon));
            let is_el: HtmlElement = icon_span.clone().dyn_into().unwrap();
            is_el.style().set_css_text("font-size: 12px;");
            card_top.append_child(&icon_span).unwrap();

            let title_span = document.create_element("span").unwrap();
            title_span.set_text_content(Some(title));
            let ts_el: HtmlElement = title_span.clone().dyn_into().unwrap();
            ts_el
                .style()
                .set_css_text("flex: 1; color: var(--text-primary);");
            card_top.append_child(&title_span).unwrap();
            card.append_child(&card_top).unwrap();

            let card_meta = document.create_element("div").unwrap();
            let cm_el: HtmlElement = card_meta.clone().dyn_into().unwrap();
            cm_el.style().set_css_text(
                "display: flex; gap: 6px; font-size: 9px; \
                 color: var(--text-muted); font-family: var(--font-mono);",
            );

            let prio = document.create_element("span").unwrap();
            prio.set_text_content(Some(priority));
            let p_el: HtmlElement = prio.clone().dyn_into().unwrap();
            p_el.style().set_css_text(&format!(
                "padding: 1px 4px; border-radius: 2px; background: {}; \
                 color: var(--text-primary); font-weight: 600;",
                priority_color(priority)
            ));
            card_meta.append_child(&prio).unwrap();

            let who = document.create_element("span").unwrap();
            who.set_text_content(Some(&short_did(assignee)));
            card_meta.append_child(&who).unwrap();
            card.append_child(&card_meta).unwrap();

            col.append_child(&card).unwrap();
        }

        // Add card button
        let add_btn = document.create_element("button").unwrap();
        add_btn.set_text_content(Some("+ Add"));
        let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
        ab_el.style().set_css_text(
            "padding: 4px; border: 1px dashed var(--border-medium); \
             background: transparent; color: var(--text-muted); \
             border-radius: 4px; cursor: pointer; font-size: 10px; \
             text-align: center;",
        );
        col.append_child(&add_btn).unwrap();

        board.append_child(&col).unwrap();
    }

    wrapper.append_child(&board).unwrap();

    // Honesty footer
    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} work item board requires wellfair_work_item_board engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn card_id(col: &str) -> &str {
    col
}

fn priority_color(p: &str) -> &'static str {
    match p {
        "P0" => "rgba(255, 99, 71, 0.3)",
        "P1" => "rgba(255, 165, 0, 0.3)",
        "P2" => "rgba(100, 149, 237, 0.3)",
        _ => "rgba(128, 128, 128, 0.3)",
    }
}

fn short_did(did: &str) -> String {
    let parts: Vec<&str> = did.split(':').collect();
    if parts.len() >= 3 {
        format!("{}...{}", &parts[2..3].concat()[..6], "")
    } else {
        did.to_string()
    }
}
