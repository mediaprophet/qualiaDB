//! Voting — active and historical votes with proposals (§2.4.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("active", "Active Votes"),
    ("history", "Vote History"),
    ("settings", "Voting Settings"),
];

const ACTIVE: &[(&str, &str, &str, &str, &str)] = &[
    (
        "RES-003: Adopt FST morphology engine",
        "ranked_choice",
        "3 of 5",
        "2026-08-20",
        "2/5 cast",
    ),
    (
        "RES-004: Approve funding proposal",
        "m_of_n",
        "4 of 5",
        "2026-08-25",
        "1/5 cast",
    ),
];

const HISTORY: &[(&str, &str, &str, &str, &str)] = &[
    (
        "RES-001: Project formation",
        "m_of_n",
        "5/5 unanimous",
        "passed",
        "2026-07-01",
    ),
    (
        "RES-002: Approve ontology spec v3",
        "ranked_choice",
        "4/5 (80%)",
        "passed",
        "2026-08-03",
    ),
    (
        "RES-002a: Amendment to RES-002",
        "m_of_n",
        "3/5 (60%)",
        "passed",
        "2026-08-05",
    ),
    (
        "RES-002b: Counter-proposal",
        "ranked_choice",
        "1/5 (20%)",
        "rejected",
        "2026-08-05",
    ),
];

pub fn build_voting_view(document: &Document) -> Element {
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

    content.append_child(&build_active_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} voting requires LOG-5 consensus engine + COP-X2 decision records.",
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
        tab.set_attribute("data-voting-tab", tab_id).unwrap();
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

fn build_active_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-voting-panel", "active").unwrap();

    for (proposal, protocol, threshold, deadline, progress) in ACTIVE {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "border: 1px solid var(--border-medium); border-radius: 6px; \
             padding: 8px; margin-bottom: 6px; background: var(--surface-panel);",
        );

        let title = document.create_element("div").unwrap();
        title.set_text_content(Some(proposal));
        let t_el: HtmlElement = title.clone().dyn_into().unwrap();
        t_el.style().set_css_text(
            "font-size: 11px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono); margin-bottom: 4px;",
        );
        card.append_child(&title).unwrap();

        let meta = document.create_element("div").unwrap();
        meta.set_text_content(Some(&format!(
            "Protocol: {}  |  Threshold: {}  |  Deadline: {}  |  Progress: {}",
            protocol, threshold, deadline, progress,
        )));
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&meta).unwrap();

        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some("Cast Vote"));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 3px 10px; border: 1px solid var(--accent-cyan); \
             background: transparent; color: var(--accent-cyan); border-radius: 3px; \
             cursor: pointer; font-size: 9px; margin-top: 4px; \
             font-family: var(--font-mono);",
        );
        card.append_child(&btn).unwrap();

        panel.append_child(&card).unwrap();
    }

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-voting-panel", tab_id).unwrap();

    match tab_id {
        "history" => {
            let table = make_table(
                document,
                &["Proposal", "Protocol", "Turnout", "Result", "Date"],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (proposal, protocol, turnout, result, date) in HISTORY {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [proposal, protocol, turnout, result, date]
                    .iter()
                    .enumerate()
                {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 3 {
                        let color = match **val {
                            "passed" => "rgba(100, 200, 100, 0.8)",
                            "rejected" => "rgba(255, 100, 100, 0.8)",
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
        "settings" => {
            let settings: &[(&str, &str, &str)] = &[
                ("Default Protocol", "select", "Ranked-choice"),
                ("M-of-N Threshold", "text", "3 of 5 (60%)"),
                ("Voting Timeout", "text", "14 days"),
                ("Amendment Quorum", "text", "4 of 5 (80%)"),
                ("Anonymous Voting", "select", "Disabled"),
            ];

            for (label, input_type, value) in settings {
                let row = document.create_element("div").unwrap();
                let r_el: HtmlElement = row.clone().dyn_into().unwrap();
                r_el.style().set_css_text(
                    "display: flex; align-items: center; gap: 8px; padding: 4px 0; \
                     border-bottom: 1px solid var(--border-subtle);",
                );

                let lbl = document.create_element("label").unwrap();
                lbl.set_text_content(Some(label));
                let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
                l_el.style().set_css_text(
                    "font-size: 10px; color: var(--text-secondary); \
                     font-family: var(--font-mono); min-width: 200px;",
                );
                row.append_child(&lbl).unwrap();

                if *input_type == "select" {
                    let sel = document.create_element("select").unwrap();
                    let opt = document.create_element("option").unwrap();
                    opt.set_text_content(Some(value));
                    sel.append_child(&opt).unwrap();
                    let s_el: HtmlElement = sel.clone().dyn_into().unwrap();
                    s_el.style().set_css_text(
                        "flex: 1; padding: 3px 6px; border: 1px solid var(--border-medium); \
                         border-radius: 3px; background: var(--surface-panel); \
                         color: var(--text-primary); font-size: 10px; \
                         font-family: var(--font-mono);",
                    );
                    row.append_child(&sel).unwrap();
                } else {
                    let inp = document.create_element("input").unwrap();
                    inp.set_attribute("type", "text").unwrap();
                    inp.set_attribute("value", value).unwrap();
                    let i_el: HtmlElement = inp.clone().dyn_into().unwrap();
                    i_el.style().set_css_text(
                        "flex: 1; padding: 3px 6px; border: 1px solid var(--border-medium); \
                         border-radius: 3px; background: var(--surface-panel); \
                         color: var(--text-primary); font-size: 10px; \
                         font-family: var(--font-mono);",
                    );
                    row.append_child(&inp).unwrap();
                }

                panel.append_child(&row).unwrap();
            }
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
