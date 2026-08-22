//! Commons Publication & Artefacts — classify, publish, track obligation paydown.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const ARTEFACTS: &[(&str, &str, &str, &str, &str)] = &[
    // (hash, kind, obligation_state, consumer_classes, shifted_at)
    (
        "0xabc123def456",
        "ontology",
        "State A",
        "corporation, government",
        "2026-08-01",
    ),
    (
        "0xdef789abc012",
        "dataset",
        "State A",
        "corporation",
        "2026-08-10",
    ),
    (
        "0xghi345jkl678",
        "ruleset",
        "State B",
        "corporation, government, researcher",
        "2026-08-20",
    ),
    (
        "0xmno901pqr234",
        "document",
        "State A",
        "corporation",
        "2026-08-25",
    ),
];

const CLASSIFICATIONS: &[(&str, &str)] = &[
    (
        "Selfhood",
        "\u{1F512} Never publishable \u{2014} personal/biometric/health",
    ),
    (
        "Personhood",
        "\u{1F464} Publishable with consent \u{2014} personal works",
    ),
    (
        "Unmarked",
        "\u{2753} Needs classification before publication",
    ),
    (
        "Permissive Commons",
        "\u{1F33F} Publishable \u{2014} share-alike, obligation-bound",
    ),
];

pub fn build_commons_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 8px; overflow: hidden;",
    );

    // Classification guide
    let guide = document.create_element("div").unwrap();
    let g_el: HtmlElement = guide.clone().dyn_into().unwrap();
    g_el.style().set_css_text(
        "padding: 8px; background: var(--surface-panel); border-radius: 4px; \
         border: 1px solid var(--border-subtle);",
    );

    let g_title = document.create_element("div").unwrap();
    g_title.set_text_content(Some("Artefact Classification"));
    let gt_el: HtmlElement = g_title.clone().dyn_into().unwrap();
    gt_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
             text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;",
    );
    guide.append_child(&g_title).unwrap();

    for (label, desc) in CLASSIFICATIONS {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style()
            .set_css_text("display: flex; gap: 6px; padding: 2px 0; font-size: 10px;");
        let lbl = document.create_element("span").unwrap();
        lbl.set_text_content(Some(label));
        let lb_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        lb_el
            .style()
            .set_css_text("font-weight: 600; color: var(--accent-cyan); min-width: 130px;");
        row.append_child(&lbl).unwrap();
        let d = document.create_element("span").unwrap();
        d.set_text_content(Some(desc));
        let d_el: HtmlElement = d.clone().dyn_into().unwrap();
        d_el.style().set_css_text("color: var(--text-secondary);");
        row.append_child(&d).unwrap();
        guide.append_child(&row).unwrap();
    }
    wrapper.append_child(&guide).unwrap();

    // Artefact registry
    let reg_title = document.create_element("div").unwrap();
    reg_title.set_text_content(Some("Published Artefacts"));
    let rt_el: HtmlElement = reg_title.clone().dyn_into().unwrap();
    rt_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
             text-transform: uppercase; letter-spacing: 0.5px; padding: 0 8px;",
    );
    wrapper.append_child(&reg_title).unwrap();

    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 10px;");

    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in &["Hash", "Kind", "State", "Consumer Classes", "Shifted At"] {
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

    let tbody = document.create_element("tbody").unwrap();
    for (hash, kind, state, classes, shifted) in ARTEFACTS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [hash, kind, state, classes, shifted].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = if val.contains("State B") {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "rgba(255, 165, 0, 0.8)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px;",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    wrapper.append_child(&table).unwrap();

    // Publish button
    let publish_btn = document.create_element("button").unwrap();
    publish_btn.set_text_content(Some("\u{1F4E1} Publish to Commons"));
    let pb_el: HtmlElement = publish_btn.clone().dyn_into().unwrap();
    pb_el.style().set_css_text(
        "margin: 6px 8px; padding: 6px 16px; border: 1px solid var(--border-medium); \
             background: rgba(100, 200, 100, 0.1); color: var(--text-primary); \
             border-radius: 3px; cursor: pointer; font-size: 11px; font-weight: 600; \
             align-self: flex-start;",
    );
    wrapper.append_child(&publish_btn).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} commons publication requires COP-M1 engine command. \
         Selfhood artefacts cannot be published.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
