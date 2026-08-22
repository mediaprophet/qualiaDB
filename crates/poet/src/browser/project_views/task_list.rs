//! Task List — flat/filterable task list complementary to Kanban (§2.5.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TASKS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "TASK-001",
        "Define ontology schema",
        "did:qualia:timothy_charles_holborn",
        "Design",
        "high",
        "done",
    ),
    (
        "TASK-002",
        "Build NLP pipeline",
        "did:qualia:timothy_charles_holborn",
        "Build",
        "high",
        "in_progress",
    ),
    (
        "TASK-003",
        "Create SHACL shapes",
        "did:qualia:contributor_02",
        "Build",
        "medium",
        "in_progress",
    ),
    (
        "TASK-004",
        "FST morphology engine",
        "did:qualia:contributor_03",
        "Build",
        "medium",
        "in_progress",
    ),
    (
        "TASK-005",
        "Review ontology spec",
        "did:qualia:contributor_02",
        "Review",
        "medium",
        "done",
    ),
    (
        "TASK-006",
        "Alpha release prep",
        "did:qualia:timothy_charles_holborn",
        "Release",
        "high",
        "not_started",
    ),
    (
        "TASK-007",
        "Documentation pass",
        "unassigned",
        "Docs",
        "low",
        "not_started",
    ),
    (
        "TASK-008",
        "Performance benchmarks",
        "did:qualia:contributor_03",
        "Build",
        "low",
        "not_started",
    ),
];

pub fn build_task_list_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 6px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle);",
    );

    for label in &[
        "All",
        "My Tasks",
        "High Priority",
        "Unassigned",
        "+ New Task",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 8px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 9px; font-family: var(--font-mono);",
        );
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    let table = make_table(
        document,
        &["ID", "Title", "Assignee", "Phase", "Priority", "Status"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, title, assignee, phase, priority, status) in TASKS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, title, assignee, phase, priority, status]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
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
            } else if i == 5 {
                let color = match **val {
                    "done" => "rgba(100, 200, 100, 0.8)",
                    "in_progress" => "rgba(0, 200, 255, 0.8)",
                    "not_started" => "var(--text-muted)",
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

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} task list aggregates from wellfair_work_item_board. CSV export pending.",
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
