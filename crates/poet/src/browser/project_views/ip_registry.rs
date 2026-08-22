//! IP Registry — intellectual property management (§8j.4).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("registry", "Registry"),
    ("workflow", "Workflow"),
    ("attribution", "Attribution"),
    ("enforcement", "Enforcement"),
];

const IP_ITEMS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "NLP Pipeline Architecture",
        "patent",
        "draft",
        "did:qualia:timothy_charles_holborn",
        "AU",
        "2026-08-01",
    ),
    (
        "Ontology Specification",
        "copyright",
        "granted",
        "did:qualia:timothy_charles_holborn",
        "global",
        "2026-08-03",
    ),
    (
        "SHACL Shapes Library",
        "copyright",
        "granted",
        "did:qualia:contributor_02",
        "global",
        "2026-08-05",
    ),
    (
        "Benchmark Dataset",
        "database_right",
        "granted",
        "did:qualia:contributor_04",
        "EU",
        "2026-08-10",
    ),
    (
        "FST Morphology Engine",
        "trade_secret",
        "filed",
        "did:qualia:timothy_charles_holborn",
        "AU",
        "2026-08-12",
    ),
    (
        "HyperCanvas Layout System",
        "design_right",
        "draft",
        "did:qualia:contributor_03",
        "AU",
        "2026-08-15",
    ),
];

const WORKFLOW_STAGES: &[(&str, &str, &str)] = &[
    ("conception", "Conception", "completed"),
    ("disclosure", "Disclosure", "completed"),
    ("review", "Prior Art Review", "active"),
    ("filing_decision", "Filing Decision", "pending"),
    ("filing", "Filing", "pending"),
    ("maintenance", "Maintenance", "pending"),
];

const ATTRIBUTIONS: &[(&str, &str, &str, &str)] = &[
    (
        "NLP Pipeline Architecture",
        "did:qualia:timothy_charles_holborn",
        "inventor",
        "2026-08-01",
    ),
    (
        "Ontology Specification",
        "did:qualia:timothy_charles_holborn",
        "author",
        "2026-08-03",
    ),
    (
        "SHACL Shapes Library",
        "did:qualia:contributor_02",
        "author",
        "2026-08-05",
    ),
    (
        "Benchmark Dataset",
        "did:qualia:contributor_04",
        "curator",
        "2026-08-10",
    ),
    (
        "FST Morphology Engine",
        "did:qualia:timothy_charles_holborn",
        "inventor",
        "2026-08-12",
    ),
    (
        "HyperCanvas Layout System",
        "did:qualia:contributor_03",
        "designer",
        "2026-08-15",
    ),
];

const ENFORCEMENT_LOG: &[(&str, &str, &str, &str)] = &[
    (
        "Ontology Specification",
        "cease_and_desist",
        "2026-08-14",
        "resolved",
    ),
    (
        "SHACL Shapes Library",
        "licensing_offer",
        "2026-08-16",
        "open",
    ),
];

pub fn build_ip_registry_view(document: &Document) -> Element {
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

    content.append_child(&build_registry_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} IP registry requires COP-X1 intellectual property engine command.",
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
        tab.set_attribute("data-ip-tab", tab_id).unwrap();
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

fn build_registry_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-ip-panel", "registry").unwrap();

    let table = make_table(
        document,
        &[
            "Title",
            "Type",
            "Status",
            "Inventor/Author",
            "Jurisdiction",
            "Date",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (title, ip_type, status, inventor, jurisdiction, date) in IP_ITEMS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [title, ip_type, status, inventor, jurisdiction, date]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "granted" => "rgba(100, 200, 100, 0.8)",
                    "filed" => "rgba(0, 200, 255, 0.8)",
                    "draft" => "rgba(255, 165, 0, 0.8)",
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

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ Register IP"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-ip-panel", tab_id).unwrap();

    match tab_id {
        "workflow" => build_workflow_tab(document, &panel),
        "attribution" => build_attribution_tab(document, &panel),
        "enforcement" => build_enforcement_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_workflow_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "IP Creation Workflow: Conception \u{2192} Disclosure \u{2192} Prior Art Review \u{2192} Filing Decision \u{2192} Filing \u{2192} Maintenance",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    for (i, (_id, label, status)) in WORKFLOW_STAGES.iter().enumerate() {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();

        let (bg, border_c, text_c) = match *status {
            "completed" => (
                "rgba(100, 200, 100, 0.08)",
                "rgba(100, 200, 100, 0.4)",
                "rgba(100, 200, 100, 0.9)",
            ),
            "active" => (
                "rgba(0, 200, 255, 0.08)",
                "rgba(0, 200, 255, 0.4)",
                "rgba(0, 200, 255, 0.9)",
            ),
            _ => ("transparent", "var(--border-subtle)", "var(--text-muted)"),
        };

        r_el.style().set_css_text(&format!(
            "display: flex; align-items: center; gap: 8px; padding: 6px 8px; \
             margin-bottom: 4px; border: 1px solid {}; border-radius: 4px; \
             background: {};",
            border_c, bg,
        ));

        let num = document.create_element("span").unwrap();
        num.set_text_content(Some(&format!("{}", i + 1)));
        let n_el: HtmlElement = num.clone().dyn_into().unwrap();
        n_el.style().set_css_text(&format!(
            "width: 20px; height: 20px; border-radius: 50%; display: flex; \
             align-items: center; justify-content: center; font-size: 9px; \
             font-family: var(--font-mono); border: 1px solid {}; color: {};",
            border_c, text_c,
        ));
        row.append_child(&num).unwrap();

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(label));
        let nm_el: HtmlElement = name.clone().dyn_into().unwrap();
        nm_el.style().set_css_text(&format!(
            "font-size: 10px; font-weight: 600; color: {}; flex: 1;",
            text_c
        ));
        row.append_child(&name).unwrap();

        let st = document.create_element("span").unwrap();
        st.set_text_content(Some(status));
        let st_el: HtmlElement = st.clone().dyn_into().unwrap();
        st_el.style().set_css_text(&format!(
            "font-size: 9px; color: {}; font-family: var(--font-mono);",
            text_c
        ));
        row.append_child(&st).unwrap();

        panel.append_child(&row).unwrap();
    }
}

fn build_attribution_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Inventor/attribution is append-only with provenance. Disputable via \u{00A7}8e.1.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["IP Item", "Attributed To", "Role", "Date"]);
    let tbody = document.create_element("tbody").unwrap();
    for (item, did, role, date) in ATTRIBUTIONS {
        let tr = document.create_element("tr").unwrap();
        for val in [item, did, role, date].iter() {
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

fn build_enforcement_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Enforcement actions are append-only with provenance. Links to disputes (\u{00A7}8e).",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["IP Item", "Action", "Date", "Status"]);
    let tbody = document.create_element("tbody").unwrap();
    for (item, action, date, status) in ENFORCEMENT_LOG {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [item, action, date, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "resolved" => "rgba(100, 200, 100, 0.8)",
                    "open" => "rgba(255, 165, 0, 0.8)",
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
