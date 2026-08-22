//! LOD Chain — per-LOD level decimation, error, .10d size (§2.2, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const LOD_LEVELS: &[(&str, u32, f64, &str, &str)] = &[
    ("LOD 0 (Full)", 0, 0.0, "45.2 MB", "100% vertices"),
    ("LOD 1", 1, 0.5, "18.7 MB", "60% vertices"),
    ("LOD 2", 2, 1.2, "7.3 MB", "35% vertices"),
    ("LOD 3", 3, 2.8, "2.1 MB", "15% vertices"),
    ("LOD 4 (Impostor)", 4, 5.0, "0.4 MB", "Billboard sprite"),
];

pub fn build_lod_chain_view(document: &Document) -> Element {
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
    for label in &["+ LOD Level", "Auto-generate", "Export .10d", "Preview"] {
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

    // Asset header
    let meta = document.create_element("div").unwrap();
    meta.set_text_content(Some(
        "Asset: house.glb  |  Full mesh: 128,420 vertices  |  5 LOD levels",
    ));
    let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
    m_el.style().set_css_text(
        "padding: 6px 8px; background: var(--surface-panel); border-radius: 4px; \
         margin-bottom: 8px; font-size: 9px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    content.append_child(&meta).unwrap();

    // LOD levels table
    let table = make_table(
        document,
        &[
            "Level",
            "LOD Index",
            "Error (px)",
            "Size",
            "Description",
            "Bar",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();

    for (name, idx, error, size, desc) in LOD_LEVELS {
        let tr = document.create_element("tr").unwrap();

        let vals: Vec<String> = vec![
            name.to_string(),
            idx.to_string(),
            format!("{:.1}", error),
            size.to_string(),
            desc.to_string(),
        ];

        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 0 {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 9px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }

        // Size bar
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
        let pct = match *idx {
            0 => 100.0,
            1 => 60.0,
            2 => 35.0,
            3 => 15.0,
            _ => 3.0,
        };
        let bar_color = if *idx == 0 {
            "rgba(100, 200, 100, 0.6)"
        } else if *idx <= 2 {
            "rgba(0, 200, 255, 0.5)"
        } else {
            "rgba(255, 165, 0, 0.5)"
        };
        bf_el.style().set_css_text(&format!(
            "position: absolute; left: 0; top: 0; bottom: 0; width: {}%; \
             background: {}; border-radius: 3px;",
            pct, bar_color,
        ));
        bar_bg.append_child(&bar_fill).unwrap();
        td.append_child(&bar_bg).unwrap();
        tr.append_child(&td).unwrap();

        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();

    // Error threshold chart placeholder
    let chart_header = document.create_element("div").unwrap();
    chart_header.set_text_content(Some("Error Threshold per LOD"));
    let ch_el: HtmlElement = chart_header.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&chart_header).unwrap();

    let chart = document.create_element("div").unwrap();
    let ct_el: HtmlElement = chart.clone().dyn_into().unwrap();
    ct_el.style().set_css_text(
        "height: 60px; background: var(--surface-panel); border-radius: 4px; \
         display: flex; align-items: flex-end; gap: 2px; padding: 4px; \
         border: 1px solid var(--border-subtle);",
    );
    let errors = [0.0, 0.5, 1.2, 2.8, 5.0];
    for e in &errors {
        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        let h = (e / 5.0_f64 * 100.0_f64).max(5.0_f64);
        b_el.style().set_css_text(&format!(
            "flex: 1; height: {}%; background: rgba(255, 165, 0, 0.4); border-radius: 1px;",
            h,
        ));
        ct_el.append_child(&bar).unwrap();
    }
    content.append_child(&chart).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} LOD chain requires render/compile_10d.rs engine.",
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
