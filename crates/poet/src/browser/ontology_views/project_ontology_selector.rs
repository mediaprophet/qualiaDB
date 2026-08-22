//! Project Ontology Selector — select which ontologies are loaded for a project (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const PROJECT_ONTOLOGIES: &[(&str, &str, bool, i32, &str)] = &[
    ("social.n3", "soc", true, 1, "No conflicts"),
    ("epistemics.n3", "epi", true, 2, "No conflicts"),
    ("agency.n3", "agn", true, 3, "No conflicts"),
    ("personhood.n3", "per", true, 4, "No conflicts"),
    ("selfhood.n3", "self", true, 5, "No conflicts"),
    ("provenance.n3", "prov", true, 6, "No conflicts"),
    ("values.n3", "val", true, 7, "No conflicts"),
    ("obligations.n3", "obl", true, 8, "No conflicts"),
    ("duty-of-care.n3", "doc", true, 9, "No conflicts"),
    ("care-scope.n3", "cs", true, 10, "No conflicts"),
    ("research.n3", "res", true, 11, "No conflicts"),
    ("agent-nomenclature.n3", "anm", true, 12, "No conflicts"),
    (
        "social-connections.n3",
        "sc",
        false,
        0,
        "Prefix conflict: sc vs soc",
    ),
    ("guardianship.n3", "grd", false, 0, "No conflicts"),
    ("ungrounded-generation.n3", "ug", false, 0, "No conflicts"),
    ("adversarial-conduct.n3", "adv", false, 0, "No conflicts"),
    ("faith-systems.n3", "faith", false, 0, "No conflicts"),
    ("game-design.n3", "gd", false, 0, "No conflicts"),
    ("spatial-3d.n3", "sp3d", false, 0, "No conflicts"),
    ("audio-production.n3", "aud", false, 0, "No conflicts"),
];

pub fn build_project_ontology_selector_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         align-items: center;",
    );
    for label in &[
        "Load Selected",
        "Unload Selected",
        "Resolve Conflicts",
        "Export Config",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 6px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
        );
        toolbar.append_child(&btn).unwrap();
    }

    let spacer = document.create_element("div").unwrap();
    let sp_el: HtmlElement = spacer.clone().dyn_into().unwrap();
    sp_el.style().set_css_text("flex: 1;");
    toolbar.append_child(&spacer).unwrap();

    let stats = document.create_element("span").unwrap();
    stats.set_text_content(Some("12 loaded | 8 available | 1 conflict"));
    let st_el: HtmlElement = stats.clone().dyn_into().unwrap();
    st_el
        .style()
        .set_css_text("font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);");
    toolbar.append_child(&stats).unwrap();
    wrapper.append_child(&toolbar).unwrap();

    // Project info
    let project_info = document.create_element("div").unwrap();
    project_info.set_text_content(Some(
        "Project: QualiaDB Core  |  12 ontologies loaded  |  612 classes  |  248 properties  |  1 prefix conflict",
    ));
    let pi_el: HtmlElement = project_info.clone().dyn_into().unwrap();
    pi_el.style().set_css_text(
        "padding: 4px 8px; background: var(--surface-panel); border-radius: 4px; \
         margin: 4px 8px; font-size: 9px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&project_info).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px 8px;");

    // Conflict warning
    let warning = document.create_element("div").unwrap();
    warning.set_text_content(Some(
        "\u{26A0} 1 prefix conflict: social-connections.n3 uses 'sc' which may conflict with 'soc' from social.n3. Consider renaming prefix.",
    ));
    let w_el: HtmlElement = warning.clone().dyn_into().unwrap();
    w_el.style().set_css_text(
        "padding: 4px 8px; background: rgba(255, 165, 0, 0.1); border-radius: 4px; \
         margin-bottom: 6px; font-size: 8px; color: rgba(255, 165, 0, 0.8); \
         font-family: var(--font-mono);",
    );
    content.append_child(&warning).unwrap();

    // Ontology list
    let table = make_table(
        document,
        &["Ontology", "Prefix", "Loaded", "Priority", "Status"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (file, prefix, loaded, priority, status) in PROJECT_ONTOLOGIES {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            file.to_string(),
            prefix.to_string(),
            if *loaded { "Loaded" } else { "Available" }.to_string(),
            if *loaded {
                priority.to_string()
            } else {
                "\u{2014}".to_string()
            },
            status.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = if *loaded {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 4 {
                let color = if status.contains("conflict") {
                    "rgba(255, 165, 0, 0.8)"
                } else {
                    "rgba(100, 200, 100, 0.6)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 7px; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 0 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 9px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 8px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
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
        "\u{26A0} Mock data \u{2014} project ontology selector requires qualia_core_db ontology_loader.",
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
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 9px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 3px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
