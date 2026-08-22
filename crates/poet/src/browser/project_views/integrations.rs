//! Integrations — external service connections (§2.9.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("services", "Connected Services"),
    ("webhooks", "Webhooks"),
    ("pipelines", "Import/Export"),
];

const SERVICES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "GitHub",
        "git",
        "connected",
        "did:qualia:timothy_charles_holborn",
        "2026-07-01",
    ),
    (
        "CI/CD Pipeline",
        "ci",
        "connected",
        "did:qualia:timothy_charles_holborn",
        "2026-07-15",
    ),
    (
        "Magnet Link Server",
        "storage",
        "connected",
        "did:qualia:timothy_charles_holborn",
        "2026-08-01",
    ),
    (
        "ILP Payment Gateway",
        "payment",
        "disconnected",
        "unassigned",
        "",
    ),
    (
        "External API: Weather",
        "api",
        "error",
        "did:qualia:contributor_03",
        "2026-08-10",
    ),
];

const WEBHOOKS: &[(&str, &str, &str, &str)] = &[
    (
        "push.events",
        "POST",
        "https://api.qualia.example/webhook/push",
        "active",
    ),
    (
        "contribution.committed",
        "POST",
        "https://ci.qualia.example/trigger",
        "active",
    ),
    (
        "milestone.reached",
        "POST",
        "https://notify.qualia.example/milestone",
        "active",
    ),
    (
        "dispute.filed",
        "POST",
        "https://notify.qualia.example/dispute",
        "paused",
    ),
];

const PIPELINES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "CSV Import: Contributors",
        "import",
        "csv",
        "2026-08-15",
        "success",
    ),
    (
        "CSV Import: Historical Contributions",
        "import",
        "csv",
        "2026-08-15",
        "success",
    ),
    (
        "Export: Budget Report",
        "export",
        "csv",
        "2026-08-18",
        "success",
    ),
    (
        "Export: Ontology (N3)",
        "export",
        "n3",
        "2026-08-18",
        "success",
    ),
    (
        "Import: SHACL Shapes",
        "import",
        "n3",
        "2026-08-10",
        "success",
    ),
    (
        "Export: Token Registry",
        "export",
        "json",
        "2026-08-17",
        "failed",
    ),
];

pub fn build_integrations_view(document: &Document) -> Element {
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

    content.append_child(&build_services_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} integrations require external transport layer + credential vault.",
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
        tab.set_attribute("data-integrations-tab", tab_id).unwrap();
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

fn build_services_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-integrations-panel", "services")
        .unwrap();

    let table = make_table(
        document,
        &["Service", "Type", "Status", "Connected By", "Since"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (name, stype, status, connected_by, since) in SERVICES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, stype, status, connected_by, since]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "connected" => "rgba(100, 200, 100, 0.8)",
                    "disconnected" => "var(--text-muted)",
                    "error" => "rgba(255, 100, 100, 0.8)",
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

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ Connect Service"));
    let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 6px;",
    );
    panel.append_child(&btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-integrations-panel", tab_id)
        .unwrap();

    match tab_id {
        "webhooks" => {
            let table = make_table(document, &["Event", "Method", "URL", "Status"]);
            let tbody = document.create_element("tbody").unwrap();
            for (event, method, url, status) in WEBHOOKS {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [event, method, url, status].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 3 {
                        let color = match **val {
                            "active" => "rgba(100, 200, 100, 0.8)",
                            "paused" => "rgba(255, 165, 0, 0.8)",
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
        "pipelines" => {
            let table = make_table(document, &["Name", "Direction", "Format", "Date", "Result"]);
            let tbody = document.create_element("tbody").unwrap();
            for (name, direction, fmt, date, result) in PIPELINES {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [name, direction, fmt, date, result].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 4 {
                        let color = match **val {
                            "success" => "rgba(100, 200, 100, 0.8)",
                            "failed" => "rgba(255, 100, 100, 0.8)",
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
