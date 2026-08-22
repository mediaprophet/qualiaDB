//! Animation Export — export animation clips to GLB/USD/Alembic (§3.3, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const FORMATS: &[(&str, &str, &str, bool)] = &[
    ("GLB 2.0", "Binary glTF with animation", "1.8 MB", true),
    ("USD", "Pixar Universal Scene Description", "3.2 MB", false),
    ("Alembic", "Alembic .abc cache", "12.7 MB", false),
    ("FBX", "Autodesk FBX (legacy)", "4.5 MB", false),
    ("Collada", "DAE XML format", "8.1 MB", false),
    ("BVH", "Biovision Hierarchy motion data", "0.3 MB", false),
];

const CLIPS: &[(&str, &str, f64, &str)] = &[
    ("Walk Cycle", "skeletal", 2.0, "30 fps"),
    ("Run Cycle", "skeletal", 1.5, "30 fps"),
    ("Idle", "skeletal", 4.0, "30 fps"),
    ("Jump", "skeletal", 1.0, "60 fps"),
    ("Wave", "skeletal", 3.0, "30 fps"),
    ("Camera Pan", "camera", 8.0, "24 fps"),
];

pub fn build_animation_export_view(document: &Document) -> Element {
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
    for label in &["Export Selected", "Export All", "Bake Animation", "Preview"] {
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

    // Clips table
    let clips_header = document.create_element("div").unwrap();
    clips_header.set_text_content(Some("Animation Clips (6)"));
    let ch_el: HtmlElement = clips_header.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&clips_header).unwrap();

    let clips_table = make_table(
        document,
        &["Clip", "Type", "Duration (s)", "Frame Rate", "Export"],
    );
    let clips_tbody = document.create_element("tbody").unwrap();
    for (name, ctype, duration, fps) in CLIPS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            ctype.to_string(),
            format!("{:.1}", duration),
            fps.to_string(),
            "\u{2705}".to_string(),
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
            } else if i == 1 {
                let color = if *ctype == "skeletal" {
                    "rgba(100, 200, 100, 0.6)"
                } else {
                    "rgba(0, 200, 255, 0.6)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 9px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        clips_tbody.append_child(&tr).unwrap();
    }
    clips_table.append_child(&clips_tbody).unwrap();
    content.append_child(&clips_table).unwrap();

    // Export formats
    let fmt_header = document.create_element("div").unwrap();
    fmt_header.set_text_content(Some("Export Formats"));
    let fh_el: HtmlElement = fmt_header.clone().dyn_into().unwrap();
    fh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&fmt_header).unwrap();

    let fmt_grid = document.create_element("div").unwrap();
    let fg_el: HtmlElement = fmt_grid.clone().dyn_into().unwrap();
    fg_el
        .style()
        .set_css_text("display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;");

    for (name, desc, size, selected) in FORMATS {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        let border = if *selected {
            "var(--accent-cyan)"
        } else {
            "var(--border-subtle)"
        };
        cd_el.style().set_css_text(&format!(
            "padding: 6px 8px; background: var(--surface-panel); border-radius: 6px; \
             border: 2px solid {}; cursor: pointer;",
            border,
        ));

        let name_div = document.create_element("div").unwrap();
        name_div.set_text_content(Some(name));
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        let name_color = if *selected {
            "var(--accent-cyan)"
        } else {
            "var(--text-primary)"
        };
        n_el.style().set_css_text(&format!(
            "font-size: 10px; font-weight: 600; color: {}; font-family: var(--font-mono);",
            name_color,
        ));
        card.append_child(&name_div).unwrap();

        let desc_div = document.create_element("div").unwrap();
        desc_div.set_text_content(Some(desc));
        let d_el: HtmlElement = desc_div.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             margin-top: 2px;",
        );
        card.append_child(&desc_div).unwrap();

        let size_div = document.create_element("div").unwrap();
        size_div.set_text_content(Some(size));
        let s_el: HtmlElement = size_div.clone().dyn_into().unwrap();
        s_el.style().set_css_text(
            "font-size: 8px; color: var(--text-secondary); font-family: var(--font-mono); \
             margin-top: 2px;",
        );
        card.append_child(&size_div).unwrap();

        fg_el.append_child(&card).unwrap();
    }
    content.append_child(&fmt_grid).unwrap();

    // Export options
    let opts_header = document.create_element("div").unwrap();
    opts_header.set_text_content(Some("Export Options"));
    let oh_el: HtmlElement = opts_header.clone().dyn_into().unwrap();
    oh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&opts_header).unwrap();

    let opts = [
        "Bake constraints: On",
        "Sample rate: 30 fps",
        "Include mesh: On",
        "Include materials: On",
        "Embed textures: Off",
        "Compression: Draco",
        "Sensitivity: Public",
        "Provenance: did:qualia:timothy_charles_holborn",
    ];
    for opt in &opts {
        let row = document.create_element("div").unwrap();
        row.set_text_content(Some(opt));
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "padding: 2px 8px; font-size: 8px; color: var(--text-secondary); \
             font-family: var(--font-mono);",
        );
        content.append_child(&row).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} animation export requires ANI-10 export engine.",
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
