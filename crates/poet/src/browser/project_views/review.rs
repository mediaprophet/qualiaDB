//! Reviews & Decisions — review assignments, decisions, decision log, argumentation.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("assignments", "Review Assignments"),
    ("decisions", "Review Decisions"),
    ("decision_log", "Decision Log"),
    ("argumentation", "Argumentation"),
];

pub fn build_review_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-review-tab", tab_id).unwrap();
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
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content
        .append_child(&build_assignments_tab(document))
        .unwrap();

    for (i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_tab_panel(document, tab_id);
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} reviews require COP-A4 ReviewAssignment engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_assignments_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-review-panel", "assignments")
        .unwrap();

    let assignments = &[
        (
            "SHACL Shapes",
            "did:qualia:reviewer_02",
            "ontology_credential",
            "2026-09-01",
            "open",
        ),
        (
            "Benchmark Results",
            "did:qualia:reviewer_01",
            "data_science_credential",
            "2026-09-15",
            "open",
        ),
        (
            "NLP Pipeline v0.1",
            "did:qualia:reviewer_02",
            "software_credential",
            "2026-08-15",
            "completed",
        ),
    ];

    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 10px;");

    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in &["Work Item", "Reviewer", "Credential", "Deadline", "Status"] {
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

    let tbody = document.create_element("tbody").unwrap();
    for (item, reviewer, cred, deadline, status) in assignments {
        let tr = document.create_element("tr").unwrap();
        for val in &[item, reviewer, cred, deadline, status] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px;",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-review-panel", tab_id).unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text("display: none;");

    let content = match tab_id {
        "decisions" => {
            "Review Decisions:\n\
             \u{2022} NLP Pipeline v0.1 \u{2014} Approve \u{2014} signed by reviewer_02 \u{2014} 2026-08-15\n\
             \u{2022} Ontology Spec \u{2014} Approve \u{2014} signed by reviewer_01 \u{2014} 2026-08-10\n\
             \u{2022} SHACL Shapes \u{2014} ChangesRequested \u{2014} signed by reviewer_02 \u{2014} 2026-08-20"
        }
        "decision_log" => {
            "Project Decision Log:\n\
             \u{2022} 2026-08-01 \u{2014} Project scope approved (3-of-3 consensus \u{2705})\n\
             \u{2022} 2026-08-10 \u{2014} Ontology approach approved (2-of-3 consensus \u{2705})\n\
             \u{2022} 2026-08-20 \u{2014} SHACL revision requested (1-of-3 \u{2014} pending)"
        }
        "argumentation" => {
            "Argumentation Framework:\n\
             \u{2022} Arg1: \"Use N3 for ontology\" \u{2192} attacks Arg2\n\
             \u{2022} Arg2: \"Use Turtle for ontology\" \u{2192} attacked by Arg1\n\
             \u{2022} Arg3: \"N3 supports rules\" \u{2192} supports Arg1\n\
             \u{2022} Grounded extension: {Arg1, Arg3} \u{2014} accepted"
        }
        _ => "",
    };

    let pre = document.create_element("pre").unwrap();
    pre.set_text_content(Some(content));
    let p_el: HtmlElement = pre.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "padding: 8px; font-size: 10px; color: var(--text-secondary); \
             font-family: var(--font-mono); white-space: pre-wrap; line-height: 1.5;",
    );
    panel.append_child(&pre).unwrap();

    panel
}
