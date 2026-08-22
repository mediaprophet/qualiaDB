//! Life Records — life event + welfare case + case task reports (§2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("events", "Life Events"),
    ("cases", "Welfare Cases"),
    ("tasks", "Case Tasks"),
];

const EVENTS: &[(&str, &str, &str, &str)] = &[
    (
        "LFE-001",
        "Birth registered",
        "1990-05-15",
        "Registry Office",
    ),
    (
        "LFE-002",
        "School enrolment",
        "2005-02-01",
        "Department of Education",
    ),
    ("LFE-003", "First employment", "2012-06-01", "Tax Office"),
    ("LFE-004", "Address change", "2024-01-10", "Local Council"),
    (
        "LFE-005",
        "Medical hardship declared",
        "2026-08-01",
        "Health Services",
    ),
];

const CASES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "CASE-001",
        "Housing Support",
        "2026-07-01",
        "open",
        "Case Worker: J. Smith",
    ),
    (
        "CASE-002",
        "Medical Expense Claim",
        "2026-08-01",
        "under_review",
        "Case Worker: M. Lee",
    ),
    (
        "CASE-003",
        "Emergency Relief",
        "2026-03-15",
        "closed",
        "Case Worker: J. Smith",
    ),
];

const TASKS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "TASK-001",
        "Submit income documents",
        "CASE-001",
        "2026-08-20",
        "pending",
    ),
    (
        "TASK-002",
        "Attend housing inspection",
        "CASE-001",
        "2026-08-25",
        "scheduled",
    ),
    (
        "TASK-003",
        "Provide medical receipts",
        "CASE-002",
        "2026-08-15",
        "completed",
    ),
    (
        "TASK-004",
        "Sign relief declaration",
        "CASE-003",
        "2026-03-20",
        "completed",
    ),
];

pub fn build_life_records_view(document: &Document) -> Element {
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

    content.append_child(&build_events_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} life records require wellfare-core/life_records.rs.",
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
        tab.set_attribute("data-life-tab", tab_id).unwrap();
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

fn build_events_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-life-panel", "events").unwrap();

    let table = make_table(document, &["ID", "Event", "Date", "Authority"]);
    let tbody = document.create_element("tbody").unwrap();
    for (id, event, date, authority) in EVENTS {
        let tr = document.create_element("tr").unwrap();
        for val in [id, event, date, authority].iter() {
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
    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-life-panel", tab_id).unwrap();

    match tab_id {
        "cases" => {
            let table = make_table(document, &["ID", "Case", "Opened", "Status", "Assigned"]);
            let tbody = document.create_element("tbody").unwrap();
            for (id, name, opened, status, assigned) in CASES {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [id, name, opened, status, assigned].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 3 {
                        let color = match **val {
                            "open" => "rgba(0, 200, 255, 0.8)",
                            "under_review" => "rgba(255, 165, 0, 0.8)",
                            "closed" => "rgba(100, 200, 100, 0.8)",
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
        "tasks" => {
            let table = make_table(document, &["ID", "Task", "Case", "Due", "Status"]);
            let tbody = document.create_element("tbody").unwrap();
            for (id, task, case, due, status) in TASKS {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [id, task, case, due, status].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 4 {
                        let color = match **val {
                            "completed" => "rgba(100, 200, 100, 0.8)",
                            "pending" => "rgba(255, 165, 0, 0.8)",
                            "scheduled" => "rgba(0, 200, 255, 0.8)",
                            "overdue" => "rgba(255, 100, 100, 0.8)",
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
