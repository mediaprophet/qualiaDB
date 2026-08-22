//! Portfolio — cross-project dashboard for the principal (§2.8.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("overview", "Overview"),
    ("budget", "Aggregate Budget"),
    ("conflicts", "Resource Conflicts"),
];

const PROJECTS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "Qualia NLP Engine",
        "execution",
        "active",
        "42,000 XEC",
        "18,000 XEC",
        "3 contributors",
    ),
    (
        "Ontology Commons",
        "operation",
        "active",
        "5,000 XEC",
        "2,000 XEC",
        "2 contributors",
    ),
    (
        "SHACL Library",
        "review",
        "active",
        "8,000 XEC",
        "6,500 XEC",
        "2 contributors",
    ),
    (
        "Webizen Research",
        "planning",
        "pending",
        "0 XEC",
        "0 XEC",
        "1 contributor",
    ),
];

const BUDGET: &[(&str, &str, &str, &str)] = &[
    ("Qualia NLP Engine", "42,000", "18,000", "24,000"),
    ("Ontology Commons", "5,000", "2,000", "3,000"),
    ("SHACL Library", "8,000", "6,500", "1,500"),
    ("Webizen Research", "0", "0", "0"),
    ("TOTAL", "55,000", "26,500", "28,500"),
];

const CONFLICTS: &[(&str, &str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "Qualia NLP Engine + SHACL Library",
        "120%",
        "over-allocated",
    ),
    (
        "did:qualia:contributor_02",
        "SHACL Library + Ontology Commons",
        "100%",
        "at capacity",
    ),
    (
        "did:qualia:contributor_03",
        "Qualia NLP Engine only",
        "40%",
        "under-allocated",
    ),
];

pub fn build_portfolio_view(document: &Document) -> Element {
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

    content.append_child(&build_overview_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} portfolio aggregates across all projects for the principal. \
         Cross-project dependency graph pending.",
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
        tab.set_attribute("data-portfolio-tab", tab_id).unwrap();
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

fn build_overview_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-portfolio-panel", "overview")
        .unwrap();

    let kpi_grid = document.create_element("div").unwrap();
    let k_el: HtmlElement = kpi_grid.clone().dyn_into().unwrap();
    k_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; margin-bottom: 8px;",
    );

    let kpis: &[(&str, &str, &str)] = &[
        ("4", "Active Projects", "var(--accent-cyan)"),
        ("55,000", "Total Budget (XEC)", "var(--text-primary)"),
        ("26,500", "Spent (XEC)", "rgba(255, 165, 0, 0.8)"),
        ("8", "Contributors", "var(--text-primary)"),
    ];

    for (value, label, color) in kpis {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "border: 1px solid var(--border-subtle); border-radius: 6px; \
             padding: 8px; background: var(--surface-panel); text-align: center;",
        );

        let v = document.create_element("div").unwrap();
        v.set_text_content(Some(value));
        let v_el: HtmlElement = v.clone().dyn_into().unwrap();
        v_el.style().set_css_text(&format!(
            "font-size: 18px; font-weight: 700; color: {}; font-family: var(--font-mono);",
            color,
        ));
        card.append_child(&v).unwrap();

        let l = document.create_element("div").unwrap();
        l.set_text_content(Some(label));
        let l_el: HtmlElement = l.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
             margin-top: 2px;",
        );
        card.append_child(&l).unwrap();

        kpi_grid.append_child(&card).unwrap();
    }
    panel.append_child(&kpi_grid).unwrap();

    let table = make_table(
        document,
        &["Project", "Stage", "Status", "Budget", "Spent", "Team"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (name, stage, status, budget, spent, team) in PROJECTS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, stage, status, budget, spent, team]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "active" => "rgba(100, 200, 100, 0.8)",
                    "pending" => "rgba(255, 165, 0, 0.8)",
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

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-portfolio-panel", tab_id).unwrap();

    match tab_id {
        "budget" => {
            let table = make_table(document, &["Project", "Budget", "Spent", "Remaining"]);
            let tbody = document.create_element("tbody").unwrap();
            for (name, budget, spent, remaining) in BUDGET {
                let tr = document.create_element("tr").unwrap();
                let is_total = *name == "TOTAL";
                for (_i, val) in [name, budget, spent, remaining].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if is_total {
                        td_el.style().set_css_text(
                            "padding: 4px 6px; border-top: 2px solid var(--border-medium); \
                             color: var(--text-primary); font-size: 10px; font-weight: 700; \
                             font-family: var(--font-mono);",
                        );
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
        "conflicts" => {
            let info = document.create_element("div").unwrap();
            info.set_text_content(Some(
                "Resource conflicts: contributors allocated across multiple projects. \
                 Over-allocation (>100%) may indicate scheduling conflicts.",
            ));
            let i_el: HtmlElement = info.clone().dyn_into().unwrap();
            i_el.style().set_css_text(
                "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
                 font-family: var(--font-mono); margin-bottom: 6px; \
                 background: var(--surface-panel); border-radius: 4px;",
            );
            panel.append_child(&info).unwrap();

            let table = make_table(
                document,
                &["Contributor", "Projects", "Allocation", "Status"],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (contributor, projects, alloc, status) in CONFLICTS {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [contributor, projects, alloc, status].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 3 {
                        let color = match **val {
                            "over-allocated" => "rgba(255, 100, 100, 0.8)",
                            "at capacity" => "rgba(255, 165, 0, 0.8)",
                            "under-allocated" => "rgba(100, 200, 100, 0.8)",
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
