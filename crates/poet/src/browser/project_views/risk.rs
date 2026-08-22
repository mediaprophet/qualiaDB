//! Risk — risk register with heatmap matrix (§2.4.3).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("register", "Risk Register"),
    ("heatmap", "Heatmap"),
    ("mitigations", "Mitigations"),
];

const RISKS: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    (
        "R-001",
        "Engine delay: FST morphology",
        "high",
        "high",
        "critical",
        "open",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "R-002",
        "Contributor availability",
        "medium",
        "medium",
        "moderate",
        "mitigated",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "R-003",
        "Ontology schema conflicts",
        "medium",
        "high",
        "major",
        "open",
        "did:qualia:contributor_02",
    ),
    (
        "R-004",
        "Funding shortfall",
        "low",
        "high",
        "moderate",
        "open",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "R-005",
        "SHACL validation failures",
        "low",
        "low",
        "minor",
        "closed",
        "did:qualia:contributor_02",
    ),
];

const MITIGATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "R-001",
        "Parallel development tracks",
        "in_progress",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "R-002",
        "Onboarded contributor_03",
        "complete",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "R-003",
        "Alignment matrix review scheduled",
        "pending",
        "did:qualia:contributor_02",
    ),
    (
        "R-004",
        "Funding proposal submitted",
        "in_progress",
        "did:qualia:timothy_charles_holborn",
    ),
];

pub fn build_risk_view(document: &Document) -> Element {
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

    content.append_child(&build_register_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} risk register requires risk ontology extension + COP-P2 work item links.",
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
        tab.set_attribute("data-risk-tab", tab_id).unwrap();
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

fn build_register_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-risk-panel", "register").unwrap();

    let table = make_table(
        document,
        &[
            "ID",
            "Description",
            "Probability",
            "Impact",
            "Severity",
            "Status",
            "Owner",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, desc, prob, impact, severity, status, owner) in RISKS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, desc, prob, impact, severity, status, owner]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "critical" => "rgba(255, 0, 0, 0.9)",
                    "major" => "rgba(255, 100, 100, 0.8)",
                    "moderate" => "rgba(255, 165, 0, 0.8)",
                    "minor" => "rgba(100, 200, 100, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 700;",
                    color,
                ));
            } else if i == 5 {
                let color = match **val {
                    "open" => "rgba(255, 165, 0, 0.8)",
                    "mitigated" => "rgba(100, 200, 100, 0.8)",
                    "closed" => "var(--text-muted)",
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
    panel.set_attribute("data-risk-panel", tab_id).unwrap();

    match tab_id {
        "heatmap" => build_heatmap_tab(document, &panel),
        "mitigations" => build_mitigations_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_heatmap_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Risk heatmap: probability (rows) \u{00D7} impact (columns). \
         Color intensity indicates severity.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let probs = ["high", "medium", "low"];
    let impacts = ["low", "medium", "high"];

    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style()
        .set_css_text("display: grid; grid-template-columns: 80px repeat(3, 1fr); gap: 2px;");

    let corner = document.create_element("div").unwrap();
    let c_el: HtmlElement = corner.clone().dyn_into().unwrap();
    c_el.style().set_css_text(
        "font-size: 8px; color: var(--text-muted); text-align: center; \
         font-family: var(--font-mono); padding: 4px;",
    );
    grid.append_child(&corner).unwrap();

    for imp in &impacts {
        let hdr = document.create_element("div").unwrap();
        hdr.set_text_content(Some(imp));
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); text-align: center; \
             font-family: var(--font-mono); padding: 4px;",
        );
        grid.append_child(&hdr).unwrap();
    }

    for prob in &probs {
        let lbl = document.create_element("div").unwrap();
        lbl.set_text_content(Some(prob));
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); text-align: right; \
             font-family: var(--font-mono); padding: 4px; padding-right: 6px;",
        );
        grid.append_child(&lbl).unwrap();

        for imp in &impacts {
            let cell = document.create_element("div").unwrap();
            let c_el: HtmlElement = cell.clone().dyn_into().unwrap();

            let (bg, label) = match (*prob, *imp) {
                ("high", "high") => ("rgba(255, 0, 0, 0.6)", "critical"),
                ("high", "medium") | ("medium", "high") => ("rgba(255, 100, 100, 0.5)", "major"),
                ("high", "low") | ("low", "high") | ("medium", "medium") => {
                    ("rgba(255, 165, 0, 0.4)", "moderate")
                }
                _ => ("rgba(100, 200, 100, 0.3)", "minor"),
            };

            c_el.style().set_css_text(&format!(
                "height: 40px; border-radius: 3px; background: {}; \
                 display: flex; align-items: center; justify-content: center; \
                 font-size: 8px; color: var(--text-primary); font-family: var(--font-mono);",
                bg,
            ));
            cell.set_text_content(Some(label));
            grid.append_child(&cell).unwrap();
        }
    }

    panel.append_child(&grid).unwrap();
}

fn build_mitigations_tab(document: &Document, panel: &Element) {
    let table = make_table(document, &["Risk", "Mitigation", "Status", "Owner"]);
    let tbody = document.create_element("tbody").unwrap();
    for (risk, mitigation, status, owner) in MITIGATIONS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [risk, mitigation, status, owner].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "complete" => "rgba(100, 200, 100, 0.8)",
                    "in_progress" => "rgba(255, 165, 0, 0.8)",
                    "pending" => "rgba(200, 200, 100, 0.8)",
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
