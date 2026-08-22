//! Desk Persistence — save/recall desk state + sharing (§5.2, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const DESK_PRESETS: &[(&str, &str, &str, &str)] = &[
    (
        "Studio Mix A",
        "2026-08-18",
        "9 channels, 4 buses",
        "Private",
    ),
    (
        "Studio Mix B",
        "2026-08-17",
        "9 channels, 4 buses, automation",
        "Private",
    ),
    (
        "Live Set",
        "2026-08-15",
        "12 channels, 6 buses",
        "Shared: Band",
    ),
    (
        "Podcast Template",
        "2026-08-10",
        "4 channels, 2 buses",
        "Public",
    ),
    (
        "Mastering Chain",
        "2026-08-05",
        "3 channels, mastering plugins",
        "Private",
    ),
    (
        "Field Recording",
        "2026-07-28",
        "2 channels, ambient",
        "Public",
    ),
];

const SHARED_WITH: &[(&str, &str, &str)] = &[
    ("did:qualia:band_member_1", "Read", "Live Set"),
    ("did:qualia:band_member_2", "Read", "Live Set"),
    ("did:qualia:producer_a", "Read/Write", "Studio Mix A"),
    ("did:qualia:collaborator_b", "Read", "Podcast Template"),
];

pub fn build_desk_persistence_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle);",
    );
    for label in &["Save Current", "Load Selected", "Share", "Export .hcf"] {
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
    wrapper.append_child(&toolbar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Current desk state
    let state_header = document.create_element("div").unwrap();
    state_header.set_text_content(Some("Current Desk State"));
    let sh_el: HtmlElement = state_header.clone().dyn_into().unwrap();
    sh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&state_header).unwrap();

    let state_info = document.create_element("div").unwrap();
    state_info.set_text_content(Some(
        "Channels: 9  |  Buses: 4  |  Automation: 10 lanes  |  Plugins: 6 loaded  |  \
         Sample rate: 48 kHz  |  Buffer: 256  |  Unsaved changes: Yes",
    ));
    let si_el: HtmlElement = state_info.clone().dyn_into().unwrap();
    si_el.style().set_css_text(
        "padding: 6px 8px; background: var(--surface-panel); border-radius: 4px; \
         margin-bottom: 8px; font-size: 9px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    content.append_child(&state_info).unwrap();

    // Presets table
    let presets_header = document.create_element("div").unwrap();
    presets_header.set_text_content(Some("Saved Desk Presets (6)"));
    let ph_el: HtmlElement = presets_header.clone().dyn_into().unwrap();
    ph_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&presets_header).unwrap();

    let presets_table = make_table(document, &["Preset", "Date", "Description", "Visibility"]);
    let presets_tbody = document.create_element("tbody").unwrap();
    for (name, date, desc, vis) in DESK_PRESETS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            date.to_string(),
            desc.to_string(),
            vis.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 0 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else if i == 3 {
                let color = if vis.starts_with("Public") {
                    "rgba(100, 200, 100, 0.8)"
                } else if vis.starts_with("Shared") {
                    "rgba(0, 200, 255, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        presets_tbody.append_child(&tr).unwrap();
    }
    presets_table.append_child(&presets_tbody).unwrap();
    content.append_child(&presets_table).unwrap();

    // Shared with
    let shared_header = document.create_element("div").unwrap();
    shared_header.set_text_content(Some("Shared With (4)"));
    let sh2_el: HtmlElement = shared_header.clone().dyn_into().unwrap();
    sh2_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&shared_header).unwrap();

    let shared_table = make_table(document, &["DID", "Permission", "Preset"]);
    let shared_tbody = document.create_element("tbody").unwrap();
    for (did, perm, preset) in SHARED_WITH {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![did.to_string(), perm.to_string(), preset.to_string()];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                let color = if perm.contains("Write") {
                    "rgba(255, 165, 0, 0.8)"
                } else {
                    "rgba(0, 200, 255, 0.8)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        shared_tbody.append_child(&tr).unwrap();
    }
    shared_table.append_child(&shared_tbody).unwrap();
    content.append_child(&shared_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} desk persistence requires AUD-16 save/recall engine.",
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
