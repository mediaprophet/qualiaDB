//! Material Editor — per-mesh PBR/Phong material params (§2.2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const MATERIALS: &[(&str, &str, f64, f64, f64, &str, &str)] = &[
    ("House Model", "#8B7355", 0.7, 0.3, 0.0, "PBR", "#000000"),
    ("Terrain DEM", "#4A7C3A", 0.9, 0.1, 0.0, "PBR", "#000000"),
    (
        "Tensor Field A",
        "#FF6B35",
        0.5,
        0.6,
        0.2,
        "Emissive",
        "#FF3300",
    ),
    ("Roof", "#2C3E50", 0.6, 0.4, 0.0, "PBR", "#000000"),
    ("Windows", "#3498DB", 0.1, 0.9, 0.0, "Glass", "#001133"),
    ("Door", "#5D4037", 0.8, 0.2, 0.0, "PBR", "#000000"),
];

pub fn build_material_editor_view(document: &Document) -> Element {
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
    for label in &["+ Material", "Copy", "Paste", "Reset"] {
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

    // Material cards
    for (name, base_color, roughness, metalness, emission, shader, emit_color) in MATERIALS {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "border: 1px solid var(--border-subtle); border-radius: 6px; \
             margin-bottom: 6px; background: var(--surface-panel); overflow: hidden;",
        );

        // Header with colour swatch
        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "display: flex; align-items: center; gap: 6px; padding: 6px 8px; \
             border-bottom: 1px solid var(--border-subtle);",
        );

        let swatch = document.create_element("div").unwrap();
        let sw_el: HtmlElement = swatch.clone().dyn_into().unwrap();
        sw_el.style().set_css_text(&format!(
            "width: 16px; height: 16px; border-radius: 3px; background: {}; \
             border: 1px solid var(--border-medium);",
            base_color,
        ));
        hdr.append_child(&swatch).unwrap();

        let title = document.create_element("div").unwrap();
        title.set_text_content(Some(name));
        let t_el: HtmlElement = title.clone().dyn_into().unwrap();
        t_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono);",
        );
        hdr.append_child(&title).unwrap();

        let shader_badge = document.create_element("span").unwrap();
        shader_badge.set_text_content(Some(shader));
        let sb_el: HtmlElement = shader_badge.clone().dyn_into().unwrap();
        sb_el.style().set_css_text(
            "margin-left: auto; font-size: 8px; color: var(--accent-cyan); \
             font-family: var(--font-mono); font-weight: 600;",
        );
        hdr.append_child(&shader_badge).unwrap();
        card.append_child(&hdr).unwrap();

        // Params grid
        let params = document.create_element("div").unwrap();
        let p_el: HtmlElement = params.clone().dyn_into().unwrap();
        p_el.style().set_css_text(
            "display: grid; grid-template-columns: repeat(2, 1fr); gap: 4px; padding: 6px 8px;",
        );

        let param_rows: Vec<(String, String, String)> = vec![
            (
                "Base Colour".into(),
                base_color.to_string(),
                base_color.to_string(),
            ),
            (
                "Roughness".into(),
                format!("{:.2}", roughness),
                "var(--text-primary)".into(),
            ),
            (
                "Metalness".into(),
                format!("{:.2}", metalness),
                "var(--text-primary)".into(),
            ),
            (
                "Emission".into(),
                format!("{:.2}", emission),
                "var(--text-primary)".into(),
            ),
            (
                "Emit Colour".into(),
                emit_color.to_string(),
                emit_color.to_string(),
            ),
            (
                "Shader".into(),
                shader.to_string(),
                "var(--accent-cyan)".into(),
            ),
        ];

        for (label, value, vcolor) in &param_rows {
            let row = document.create_element("div").unwrap();
            let r_el: HtmlElement = row.clone().dyn_into().unwrap();
            r_el.style().set_css_text(
                "display: flex; justify-content: space-between; align-items: center; \
                 padding: 2px 0;",
            );

            let l = document.create_element("span").unwrap();
            l.set_text_content(Some(label));
            let l_el: HtmlElement = l.clone().dyn_into().unwrap();
            l_el.style().set_css_text(
                "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
            );
            row.append_child(&l).unwrap();

            let v = document.create_element("span").unwrap();
            v.set_text_content(Some(value));
            let v_el: HtmlElement = v.clone().dyn_into().unwrap();
            v_el.style().set_css_text(&format!(
                "font-size: 9px; color: {}; font-family: var(--font-mono); font-weight: 600;",
                vcolor,
            ));
            row.append_child(&v).unwrap();
            p_el.append_child(&row).unwrap();
        }
        card.append_child(&params).unwrap();

        // Roughness/metalness sliders (visual bars)
        let bars = document.create_element("div").unwrap();
        let b_el: HtmlElement = bars.clone().dyn_into().unwrap();
        b_el.style().set_css_text("padding: 0 8px 6px;");

        for (label, val) in &[
            ("Roughness", *roughness),
            ("Metalness", *metalness),
            ("Emission", *emission),
        ] {
            let bar_row = document.create_element("div").unwrap();
            let br_el: HtmlElement = bar_row.clone().dyn_into().unwrap();
            br_el
                .style()
                .set_css_text("display: flex; align-items: center; gap: 4px; margin-bottom: 2px;");

            let l = document.create_element("span").unwrap();
            l.set_text_content(Some(label));
            let l_el: HtmlElement = l.clone().dyn_into().unwrap();
            l_el.style().set_css_text(
                "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
                 min-width: 60px;",
            );
            bar_row.append_child(&l).unwrap();

            let bar_bg = document.create_element("div").unwrap();
            let bb_el: HtmlElement = bar_bg.clone().dyn_into().unwrap();
            bb_el.style().set_css_text(
                "flex: 1; height: 6px; background: var(--surface-bg); border-radius: 3px; \
                 position: relative;",
            );

            let bar_fill = document.create_element("div").unwrap();
            let bf_el: HtmlElement = bar_fill.clone().dyn_into().unwrap();
            bf_el.style().set_css_text(&format!(
                "position: absolute; left: 0; top: 0; bottom: 0; width: {}%; \
                 background: var(--accent-cyan); border-radius: 3px;",
                val * 100.0,
            ));
            bar_bg.append_child(&bar_fill).unwrap();
            bar_row.append_child(&bar_bg).unwrap();
            b_el.append_child(&bar_row).unwrap();
        }
        card.append_child(&bars).unwrap();
        content.append_child(&card).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} material editor requires R3D-8 material system engine.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
