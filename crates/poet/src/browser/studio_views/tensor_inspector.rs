//! Tensor Inspector — Tensor10D field editor with live preview (§2.2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TENSOR_FIELDS: &[(&str, &str, f64, &str)] = &[
    (
        "\u{03C3} (sigma)",
        "Phenomenal intensity",
        0.75,
        "0.0 - 1.0",
    ),
    ("\u{03B1} (alpha)", "Onset/decay alpha", 0.30, "0.0 - 1.0"),
    ("\u{03BC} (mu)", "Mean position", 0.50, "0.0 - 1.0"),
    ("x", "Spatial X", 0.20, "-1.0 - 1.0"),
    ("y", "Spatial Y", 0.50, "-1.0 - 1.0"),
    ("z", "Spatial Z", 0.80, "-1.0 - 1.0"),
    ("v", "Velocity V", 0.10, "-1.0 - 1.0"),
    ("w", "Velocity W", 0.00, "-1.0 - 1.0"),
    ("q", "Quality Q", 0.85, "0.0 - 1.0"),
    ("t", "Time T", 0.00, "0.0 - 1.0"),
];

pub fn build_tensor_inspector_view(document: &Document) -> Element {
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
    for label in &[
        "Upload Tensor",
        "Export .10d",
        "Reset Fields",
        "Live Preview",
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
    wrapper.append_child(&toolbar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Tensor metadata
    let meta = document.create_element("div").unwrap();
    meta.set_text_content(Some(
        "Tensor10D: Field A\n\
         Dimensions: 256\u{00B3}  |  LOD: 0  |  Nodes: 16,777,216  |  Sensitivity: Restricted",
    ));
    let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
    m_el.style().set_css_text(
        "padding: 6px 8px; background: var(--surface-panel); border-radius: 4px; \
         margin-bottom: 8px; font-size: 9px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    content.append_child(&meta).unwrap();

    // Field editor table
    let table = make_table(
        document,
        &["Field", "Description", "Value", "Range", "Slider"],
    );
    let tbody = document.create_element("tbody").unwrap();

    for (symbol, desc, value, range) in TENSOR_FIELDS {
        let tr = document.create_element("tr").unwrap();

        // Symbol
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(symbol));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--accent-cyan); font-size: 11px; font-weight: 700; \
             font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Description
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(desc));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Value
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(&format!("{:.3}", value)));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-primary); font-size: 10px; font-weight: 600; \
             font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Range
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(range));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Slider bar
        let td = document.create_element("td").unwrap();
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); min-width: 80px;",
        );

        let bar_bg = document.create_element("div").unwrap();
        let bb_el: HtmlElement = bar_bg.clone().dyn_into().unwrap();
        bb_el.style().set_css_text(
            "height: 6px; background: var(--surface-bg); border-radius: 3px; position: relative;",
        );
        let bar_fill = document.create_element("div").unwrap();
        let bf_el: HtmlElement = bar_fill.clone().dyn_into().unwrap();
        let pct = ((value + 1.0) / 2.0 * 100.0).max(0.0).min(100.0);
        bf_el.style().set_css_text(&format!(
            "position: absolute; left: 0; top: 0; bottom: 0; width: {}%; \
             background: var(--accent-cyan); border-radius: 3px;",
            pct,
        ));
        bar_bg.append_child(&bar_fill).unwrap();
        td.append_child(&bar_bg).unwrap();
        tr.append_child(&td).unwrap();

        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();

    // Preview placeholder
    let preview = document.create_element("div").unwrap();
    let p_el: HtmlElement = preview.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "margin-top: 8px; height: 80px; background: var(--surface-panel); \
         border-radius: 6px; display: flex; align-items: center; justify-content: center; \
         border: 1px solid var(--border-subtle);",
    );
    let ph = document.create_element("div").unwrap();
    ph.set_text_content(Some(
        "Live Preview \u{2014} PortalGpu ambient particle field (not wired)",
    ));
    let ph_el: HtmlElement = ph.clone().dyn_into().unwrap();
    ph_el
        .style()
        .set_css_text("font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);");
    p_el.append_child(&ph).unwrap();
    content.append_child(&preview).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} tensor inspector requires PortalGpu upload_tensor engine.",
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
            "text-align: left; padding: 4px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
