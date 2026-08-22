//! Awards — project award registry and nomination queue (§8d.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("awarded", "Awarded"),
    ("nominations", "Nominations"),
    ("definitions", "Award Definitions"),
];

const AWARDED: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Outstanding Contribution",
        "did:qualia:timothy_charles_holborn",
        "2026-08-15",
        "Founder",
        "QLA 5,000",
    ),
    (
        "Best Review",
        "did:qualia:contributor_02",
        "2026-08-10",
        "Reviewer",
        "QLA 2,000",
    ),
    (
        "Innovation Excellence",
        "did:qualia:timothy_charles_holborn",
        "2026-08-12",
        "Innovator",
        "CRB 1,000",
    ),
    (
        "Community Builder",
        "did:qualia:contributor_03",
        "2026-08-08",
        "Contributor",
        "QLA 1,000",
    ),
];

const NOMINATIONS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Diligence Award",
        "did:qualia:contributor_03",
        "did:qualia:timothy_charles_holborn",
        "pending",
        "Consistent high-quality contributions",
    ),
    (
        "Mentorship Award",
        "did:qualia:contributor_02",
        "did:qualia:contributor_03",
        "pending",
        "Guided new contributors through onboarding",
    ),
];

const DEFINITIONS: &[(&str, &str, &str, &str)] = &[
    (
        "Outstanding Contribution",
        "Exceptional contribution to project",
        "QLA 5,000",
        "quarterly",
    ),
    (
        "Best Review",
        "Most thorough and constructive review",
        "QLA 2,000",
        "monthly",
    ),
    (
        "Innovation Excellence",
        "Novel approach or tool",
        "CRB 1,000",
        "quarterly",
    ),
    (
        "Community Builder",
        "Fostering collaboration and onboarding",
        "QLA 1,000",
        "quarterly",
    ),
    (
        "Diligence Award",
        "Consistent high-quality work",
        "QLA 1,500",
        "monthly",
    ),
    (
        "Mentorship Award",
        "Outstanding guidance to new contributors",
        "QLA 1,500",
        "quarterly",
    ),
];

pub fn build_awards_view(document: &Document) -> Element {
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

    content.append_child(&build_awarded_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} awards require governance approval workflow + token minting integration.",
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
        tab.set_attribute("data-awards-tab", tab_id).unwrap();
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

fn build_awarded_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-awards-panel", "awarded").unwrap();

    let table = make_table(document, &["Award", "Recipient", "Date", "Role", "Reward"]);
    let tbody = document.create_element("tbody").unwrap();
    for (award, recipient, date, role, reward) in AWARDED {
        let tr = document.create_element("tr").unwrap();
        for val in [award, recipient, date, role, reward].iter() {
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
    panel.set_attribute("data-awards-panel", tab_id).unwrap();

    match tab_id {
        "nominations" => build_nominations_tab(document, &panel),
        "definitions" => build_definitions_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_nominations_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Nominations require governance approval. Approved nominations trigger token minting to recipient.",
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
        &["Award", "Nominee", "Nominator", "Status", "Reason"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (award, nominee, nominator, status, reason) in NOMINATIONS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [award, nominee, nominator, status, reason]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "approved" => "rgba(100, 200, 100, 0.8)",
                    "pending" => "rgba(255, 165, 0, 0.8)",
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

fn build_definitions_tab(document: &Document, panel: &Element) {
    let table = make_table(document, &["Award", "Description", "Reward", "Frequency"]);
    let tbody = document.create_element("tbody").unwrap();
    for (name, desc, reward, freq) in DEFINITIONS {
        let tr = document.create_element("tr").unwrap();
        for val in [name, desc, reward, freq].iter() {
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

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ Define Award"));
    let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 6px;",
    );
    panel.append_child(&btn).unwrap();
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
