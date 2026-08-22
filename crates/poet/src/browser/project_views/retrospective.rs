//! Retrospective — sprint/phase retrospective with append-only entries (§2.10.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("current", "Current Retrospective"),
    ("history", "Past Retrospectives"),
];

const RETRO_ITEMS: &[(&str, &str, &str, &str)] = &[
    (
        "went_well",
        "Ontology spec approved on time",
        "did:qualia:timothy_charles_holborn",
        "2026-08-03",
    ),
    (
        "went_well",
        "Contributor onboarding smooth",
        "did:qualia:timothy_charles_holborn",
        "2026-08-01",
    ),
    (
        "improve",
        "FST engine behind schedule",
        "did:qualia:contributor_03",
        "2026-08-18",
    ),
    (
        "improve",
        "Review queue bottleneck",
        "did:qualia:contributor_02",
        "2026-08-18",
    ),
    (
        "action",
        "Add parallel dev tracks for FST",
        "did:qualia:timothy_charles_holborn",
        "2026-08-18",
    ),
    (
        "action",
        "Rotate review assignments weekly",
        "did:qualia:contributor_02",
        "2026-08-18",
    ),
];

const PAST_RETROS: &[(&str, &str, &str, &str)] = &[
    (
        "Sprint 1: Planning",
        "2026-07-31",
        "3 items",
        "2/3 actions completed",
    ),
    (
        "Sprint 2: Design Phase",
        "2026-08-15",
        "5 items",
        "4/5 actions completed",
    ),
];

pub fn build_retrospective_view(document: &Document) -> Element {
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

    content.append_child(&build_current_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} retrospective is append-only (COP-X1). \
         Links to decisions and work items for evidence.",
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
        tab.set_attribute("data-retro-tab", tab_id).unwrap();
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

fn build_current_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-retro-panel", "current").unwrap();

    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Sprint 3: Build Phase \u{2014} Retrospective"));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 12px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 6px;",
    );
    panel.append_child(&title).unwrap();

    let columns: &[(&str, &str)] = &[
        ("went_well", "What Went Well"),
        ("improve", "What to Improve"),
        ("action", "Action Items"),
    ];

    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style()
        .set_css_text("display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;");

    for (col_key, col_title) in columns {
        let col = document.create_element("div").unwrap();
        let c_el: HtmlElement = col.clone().dyn_into().unwrap();
        let border_color = match *col_key {
            "went_well" => "rgba(100, 200, 100, 0.3)",
            "improve" => "rgba(255, 165, 0, 0.3)",
            "action" => "rgba(0, 200, 255, 0.3)",
            _ => "var(--border-subtle)",
        };
        c_el.style().set_css_text(&format!(
            "border: 1px solid {}; border-radius: 6px; padding: 8px; \
             background: var(--surface-panel); min-height: 200px;",
            border_color,
        ));

        let hdr = document.create_element("div").unwrap();
        hdr.set_text_content(Some(col_title));
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        let hdr_color = match *col_key {
            "went_well" => "rgba(100, 200, 100, 0.8)",
            "improve" => "rgba(255, 165, 0, 0.8)",
            "action" => "rgba(0, 200, 255, 0.8)",
            _ => "var(--text-primary)",
        };
        h_el.style().set_css_text(&format!(
            "font-size: 10px; font-weight: 600; color: {}; \
             font-family: var(--font-mono); margin-bottom: 6px;",
            hdr_color,
        ));
        col.append_child(&hdr).unwrap();

        for (kind, text, author, date) in RETRO_ITEMS {
            if *kind != *col_key {
                continue;
            }

            let card = document.create_element("div").unwrap();
            let card_el: HtmlElement = card.clone().dyn_into().unwrap();
            card_el.style().set_css_text(
                "border: 1px solid var(--border-subtle); border-radius: 3px; \
                 padding: 4px 6px; margin-bottom: 4px; background: var(--surface-base);",
            );

            let t = document.create_element("div").unwrap();
            t.set_text_content(Some(text));
            let t_el: HtmlElement = t.clone().dyn_into().unwrap();
            t_el.style().set_css_text(
                "font-size: 9px; color: var(--text-primary); \
                 font-family: var(--font-mono);",
            );
            card.append_child(&t).unwrap();

            let meta = document.create_element("div").unwrap();
            meta.set_text_content(Some(&format!("{} \u{2014} {}", author, date)));
            let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
            m_el.style().set_css_text(
                "font-size: 7px; color: var(--text-muted); \
                 font-family: var(--font-mono); margin-top: 2px;",
            );
            card.append_child(&meta).unwrap();

            col.append_child(&card).unwrap();
        }

        let add_btn = document.create_element("button").unwrap();
        add_btn.set_text_content(Some("+ Add"));
        let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
        ab_el.style().set_css_text(
            "padding: 2px 6px; border: 1px dashed var(--border-medium); \
             background: transparent; color: var(--text-muted); border-radius: 3px; \
             cursor: pointer; font-size: 9px; font-family: var(--font-mono); \
             width: 100%;",
        );
        col.append_child(&add_btn).unwrap();

        grid.append_child(&col).unwrap();
    }

    panel.append_child(&grid).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-retro-panel", tab_id).unwrap();

    match tab_id {
        "history" => {
            let table = make_table(document, &["Sprint", "Date", "Items", "Actions Status"]);
            let tbody = document.create_element("tbody").unwrap();
            for (name, date, items, actions) in PAST_RETROS {
                let tr = document.create_element("tr").unwrap();
                for val in [name, date, items, actions].iter() {
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
        _ => {}
    }

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
