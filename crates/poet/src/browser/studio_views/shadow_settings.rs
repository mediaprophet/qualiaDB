//! Shadow Settings — shadow mapping configuration (§2.1, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const SHADOW_PARAMS: &[(&str, &str, &str)] = &[
    ("Enabled", "On", "toggle"),
    (
        "Shadow Map Size",
        "2048x2048",
        "select: 512 / 1024 / 2048 / 4096",
    ),
    (
        "Shadow Filter",
        "PCF Soft",
        "select: Hard / PCF / PCF Soft / VSM",
    ),
    ("Cascade Count", "4", "select: 1 / 2 / 4 / 8"),
    (
        "Cascade Split",
        "0.1 / 0.3 / 0.6 / 1.0",
        "per-cascade split ratios",
    ),
    ("Bias", "0.005", "numeric: 0.0 - 0.1"),
    ("Normal Bias", "0.02", "numeric: 0.0 - 0.1"),
    ("Max Distance", "80.0 m", "numeric: shadow draw distance"),
    ("Fade Start", "60.0 m", "numeric: fade-out start distance"),
    ("Quality", "High", "select: Low / Medium / High / Ultra"),
];

pub fn build_shadow_settings_view(document: &Document) -> Element {
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
    for label in &["Enable Shadows", "Reset Defaults", "Preview"] {
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

    // Thermal warning
    let warning = document.create_element("div").unwrap();
    warning.set_text_content(Some(
        "\u{26A0} Shadow mapping is thermal-governed. Disabled on Warm/Critical thermal state.",
    ));
    let w_el: HtmlElement = warning.clone().dyn_into().unwrap();
    w_el.style().set_css_text(
        "padding: 4px 8px; background: rgba(255, 165, 0, 0.1); border-radius: 4px; \
         margin-bottom: 8px; font-size: 8px; color: rgba(255, 165, 0, 0.8); \
         font-family: var(--font-mono);",
    );
    content.append_child(&warning).unwrap();

    // Params table
    let table = make_table(document, &["Parameter", "Value", "Type"]);
    let tbody = document.create_element("tbody").unwrap();
    for (param, value, ptype) in SHADOW_PARAMS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![param.to_string(), value.to_string(), ptype.to_string()];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();

    // Cascade visualization
    let cascade_header = document.create_element("div").unwrap();
    cascade_header.set_text_content(Some("Cascade Split Visualization"));
    let ch_el: HtmlElement = cascade_header.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&cascade_header).unwrap();

    let cascade_bar = document.create_element("div").unwrap();
    let cb_el: HtmlElement = cascade_bar.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "height: 24px; display: flex; border-radius: 4px; overflow: hidden; \
         border: 1px solid var(--border-subtle);",
    );
    let cascades = [
        ("C1: 0-10%", "10%", "rgba(100, 200, 100, 0.4)"),
        ("C2: 10-30%", "20%", "rgba(0, 200, 255, 0.4)"),
        ("C3: 30-60%", "30%", "rgba(255, 165, 0, 0.4)"),
        ("C4: 60-100%", "40%", "rgba(255, 100, 100, 0.4)"),
    ];
    for (label, width, color) in &cascades {
        let seg = document.create_element("div").unwrap();
        seg.set_text_content(Some(label));
        let s_el: HtmlElement = seg.clone().dyn_into().unwrap();
        s_el.style().set_css_text(&format!(
            "width: {}; background: {}; display: flex; align-items: center; \
             justify-content: center; font-size: 7px; color: var(--text-primary); \
             font-family: var(--font-mono);",
            width, color,
        ));
        cb_el.append_child(&seg).unwrap();
    }
    content.append_child(&cascade_bar).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} shadow settings require R3D-9 shadow mapping engine.",
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
