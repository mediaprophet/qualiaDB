//! Bulk Import — CSV/JSON data entry with validation (§8f.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("upload", "Upload & Map"),
    ("preview", "Preview & Validate"),
    ("drafts", "Drafts"),
];

const IMPORT_TYPES: &[(&str, &str, &str)] = &[
    (
        "Contributors",
        "CSV",
        "DID (or handle), role, skill level, compensation status, join date",
    ),
    (
        "Contributions",
        "CSV/JSON",
        "Contributor, date, type, quantity, fair value, compensation status",
    ),
    (
        "Agreements",
        "JSON",
        "Title, parties, instrument, status, date signed",
    ),
    (
        "Assets",
        "CSV/JSON",
        "Name, type, license, provenance, value",
    ),
];

const PREVIEW_ROWS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "1",
        "did:qualia:contributor_02",
        "reviewer",
        "intermediate",
        "valid",
    ),
    (
        "2",
        "did:qualia:contributor_03",
        "contributor",
        "advanced",
        "valid",
    ),
    ("3", "anon_nlp_specialist", "contributor", "expert", "valid"),
    (
        "4",
        "",
        "contributor",
        "entry",
        "error: missing DID or handle",
    ),
    (
        "5",
        "did:qualia:contributor_05",
        "contributor",
        "advanced",
        "warning: retroactive date",
    ),
];

const DRAFTS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "DRAFT-001",
        "Contributors",
        "5 rows",
        "3 valid, 1 error, 1 warning",
        "2026-08-17",
    ),
    (
        "DRAFT-002",
        "Contributions",
        "23 rows",
        "23 valid",
        "2026-08-16",
    ),
    ("DRAFT-003", "Agreements", "2 rows", "2 valid", "2026-08-14"),
];

pub fn build_bulk_import_view(document: &Document) -> Element {
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

    content.append_child(&build_upload_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} bulk import requires COP-P1 project lifecycle engine command. \
         Retroactive dates supported (asserted time vs valid time).",
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
        tab.set_attribute("data-bulkimport-tab", tab_id).unwrap();
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

fn build_upload_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-bulkimport-panel", "upload")
        .unwrap();

    let dropzone = document.create_element("div").unwrap();
    let d_el: HtmlElement = dropzone.clone().dyn_into().unwrap();
    d_el.style().set_css_text(
        "border: 2px dashed var(--border-medium); border-radius: 8px; \
         padding: 24px; text-align: center; margin-bottom: 8px; \
         background: var(--surface-panel);",
    );

    let icon = document.create_element("div").unwrap();
    icon.set_text_content(Some("\u{1F4E4}"));
    let i_el: HtmlElement = icon.clone().dyn_into().unwrap();
    i_el.style()
        .set_css_text("font-size: 32px; opacity: 0.4; margin-bottom: 8px;");
    dropzone.append_child(&icon).unwrap();

    let msg = document.create_element("div").unwrap();
    msg.set_text_content(Some("Drop CSV or JSON file here, or click to browse"));
    let m_el: HtmlElement = msg.clone().dyn_into().unwrap();
    m_el.style()
        .set_css_text("font-size: 11px; color: var(--text-muted); font-family: var(--font-mono);");
    dropzone.append_child(&msg).unwrap();

    panel.append_child(&dropzone).unwrap();

    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Supported Import Types"));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    panel.append_child(&title).unwrap();

    let table = make_table(document, &["Type", "Format", "Fields"]);
    let tbody = document.create_element("tbody").unwrap();
    for (name, format, fields) in IMPORT_TYPES {
        let tr = document.create_element("tr").unwrap();
        for val in [name, format, fields].iter() {
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
    panel
        .set_attribute("data-bulkimport-panel", tab_id)
        .unwrap();

    match tab_id {
        "preview" => build_preview_tab(document, &panel),
        "drafts" => build_drafts_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_preview_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Preview: DRAFT-001 (Contributors) \u{2014} 5 rows. Review errors before commit.",
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
        &["Row", "DID/Handle", "Role", "Skill", "Validation"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (row, did, role, skill, validation) in PREVIEW_ROWS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [row, did, role, skill, validation].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = if val.starts_with("error") {
                    "rgba(255, 100, 100, 0.8)"
                } else if val.starts_with("warning") {
                    "rgba(255, 165, 0, 0.8)"
                } else if val.starts_with("valid") {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "var(--text-primary)"
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

    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style()
        .set_css_text("display: flex; gap: 8px; margin-top: 6px;");

    let commit_btn = document.create_element("button").unwrap();
    commit_btn.set_text_content(Some("Commit Valid (3)"));
    let cb_el: HtmlElement = commit_btn.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid rgba(100, 200, 100, 0.4); \
         background: rgba(100, 200, 100, 0.08); color: rgba(100, 200, 100, 0.9); \
         border-radius: 3px; cursor: pointer; font-size: 10px;",
    );
    actions.append_child(&commit_btn).unwrap();

    let save_btn = document.create_element("button").unwrap();
    save_btn.set_text_content(Some("Save as Draft"));
    let sb_el: HtmlElement = save_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    actions.append_child(&save_btn).unwrap();

    panel.append_child(&actions).unwrap();
}

fn build_drafts_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Drafts are savable and resumable. Commit all at once after review.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["ID", "Type", "Size", "Validation", "Date"]);
    let tbody = document.create_element("tbody").unwrap();
    for (id, dtype, size, validation, date) in DRAFTS {
        let tr = document.create_element("tr").unwrap();
        for val in [id, dtype, size, validation, date].iter() {
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
