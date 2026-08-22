//! Asset Manager — digital asset registry with licensing (§2.6.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("grid", "Asset Grid"),
    ("registry", "Registry"),
    ("licenses", "Licenses"),
];

const ASSETS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "Ontology Schema",
        "ontology",
        "CC-BY-SA",
        "did:qualia:timothy_charles_holborn",
        "active",
        "public",
    ),
    (
        "NLP Pipeline Code",
        "code",
        "Permissive Commons",
        "did:qualia:timothy_charles_holborn",
        "active",
        "public",
    ),
    (
        "SHACL Shapes Library",
        "ontology",
        "CC-BY",
        "did:qualia:contributor_02",
        "active",
        "public",
    ),
    (
        "FST Dictionary",
        "dataset",
        "CC-BY-SA",
        "did:qualia:timothy_charles_holborn",
        "active",
        "public",
    ),
    (
        "Research Dataset Q3",
        "dataset",
        "restricted",
        "did:qualia:contributor_03",
        "active",
        "restricted",
    ),
    (
        "Brand Assets",
        "image",
        "selfhood",
        "did:qualia:timothy_charles_holborn",
        "active",
        "restricted",
    ),
];

const LICENSES: &[(&str, &str, &str, &str)] = &[
    (
        "CC-BY-SA",
        "Creative Commons Attribution-ShareAlike",
        "public",
        "all natural persons",
    ),
    (
        "CC-BY",
        "Creative Commons Attribution",
        "public",
        "all natural persons",
    ),
    (
        "Permissive Commons",
        "COP permissive license",
        "public",
        "all agents",
    ),
    (
        "restricted",
        "Project-specific restricted",
        "restricted",
        "members only",
    ),
    (
        "selfhood",
        "Selfhood-protected asset",
        "restricted",
        "principal only",
    ),
];

pub fn build_asset_mgr_view(document: &Document) -> Element {
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

    content.append_child(&build_grid_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} asset manager requires COP-X5 + COP-R3 licensing + COP-M1/M3 selfhood engine.",
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
        tab.set_attribute("data-assetmgr-tab", tab_id).unwrap();
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

fn build_grid_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-assetmgr-panel", "grid").unwrap();

    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style()
        .set_css_text("display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;");

    for (name, atype, license, owner, _status, sensitivity) in ASSETS {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        let border = match *sensitivity {
            "public" => "var(--border-medium)",
            "restricted" => "rgba(255, 165, 0, 0.3)",
            _ => "var(--border-subtle)",
        };
        c_el.style().set_css_text(&format!(
            "border: 1px solid {}; border-radius: 6px; padding: 8px; \
             background: var(--surface-panel);",
            border,
        ));

        let icon = match *atype {
            "ontology" => "\u{1F4D6}",
            "code" => "\u{1F4BB}",
            "dataset" => "\u{1F4CA}",
            "image" => "\u{1F5BC}",
            _ => "\u{1F4C1}",
        };

        let n = document.create_element("div").unwrap();
        n.set_text_content(Some(&format!("{} {}", icon, name)));
        let n_el: HtmlElement = n.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono); margin-bottom: 4px;",
        );
        card.append_child(&n).unwrap();

        let meta = document.create_element("div").unwrap();
        meta.set_text_content(Some(&format!("Type: {}  |  License: {}", atype, license)));
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&meta).unwrap();

        let o = document.create_element("div").unwrap();
        o.set_text_content(Some(owner));
        let o_el: HtmlElement = o.clone().dyn_into().unwrap();
        o_el.style().set_css_text(
            "font-size: 8px; color: var(--accent-cyan); font-family: var(--font-mono); \
             margin-top: 2px;",
        );
        card.append_child(&o).unwrap();

        let s = document.create_element("div").unwrap();
        s.set_text_content(Some(sensitivity));
        let s_el: HtmlElement = s.clone().dyn_into().unwrap();
        let s_color = match *sensitivity {
            "public" => "rgba(100, 200, 100, 0.8)",
            "restricted" => "rgba(255, 165, 0, 0.8)",
            _ => "var(--text-muted)",
        };
        s_el.style().set_css_text(&format!(
            "font-size: 8px; color: {}; margin-top: 2px;",
            s_color
        ));
        card.append_child(&s).unwrap();

        grid.append_child(&card).unwrap();
    }

    panel.append_child(&grid).unwrap();
    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-assetmgr-panel", tab_id).unwrap();

    match tab_id {
        "registry" => {
            let table = make_table(
                document,
                &["Name", "Type", "License", "Owner", "Status", "Sensitivity"],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (name, atype, license, owner, status, sensitivity) in ASSETS {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [name, atype, license, owner, status, sensitivity]
                    .iter()
                    .enumerate()
                {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 5 {
                        let color = match **val {
                            "public" => "rgba(100, 200, 100, 0.8)",
                            "restricted" => "rgba(255, 165, 0, 0.8)",
                            _ => "var(--text-muted)",
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
        "licenses" => {
            let table = make_table(document, &["License", "Description", "Class", "Consumer"]);
            let tbody = document.create_element("tbody").unwrap();
            for (name, desc, class, consumer) in LICENSES {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [name, desc, class, consumer].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 2 {
                        let color = match **val {
                            "public" => "rgba(100, 200, 100, 0.8)",
                            "restricted" => "rgba(255, 165, 0, 0.8)",
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
