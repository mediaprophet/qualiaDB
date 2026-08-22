//! Lighting Editor — per-light params (§2.2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const LIGHTS: &[(&str, &str, &str, f64, f64, &str, &str)] = &[
    (
        "Sun",
        "directional",
        "#FFF4E0",
        1.0,
        0.0,
        "(1, -1, 0.5)",
        "On",
    ),
    (
        "Ambient Sky",
        "ambient",
        "#87CEEB",
        0.3,
        0.0,
        "(0, 1, 0)",
        "On",
    ),
    (
        "Point Light A",
        "point",
        "#FFFFFF",
        0.8,
        10.0,
        "(5, 3, 5)",
        "On",
    ),
    (
        "Point Light B",
        "point",
        "#FFEEAA",
        0.5,
        8.0,
        "(-3, 2, -2)",
        "Off",
    ),
    (
        "Spot Light",
        "spot",
        "#FFFFFF",
        1.2,
        15.0,
        "(0, 10, 0)",
        "On",
    ),
    (
        "Rim Light",
        "directional",
        "#4488FF",
        0.4,
        0.0,
        "(-1, 0, -1)",
        "Off",
    ),
];

pub fn build_lighting_editor_view(document: &Document) -> Element {
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
        "+ Directional",
        "+ Point",
        "+ Spot",
        "+ Ambient",
        "Shadows On",
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

    for (name, ltype, color, intensity, falloff, direction, state) in LIGHTS {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "border: 1px solid var(--border-subtle); border-radius: 6px; \
             margin-bottom: 6px; background: var(--surface-panel); overflow: hidden;",
        );

        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "display: flex; align-items: center; gap: 6px; padding: 6px 8px; \
             border-bottom: 1px solid var(--border-subtle);",
        );

        let swatch = document.create_element("div").unwrap();
        let sw_el: HtmlElement = swatch.clone().dyn_into().unwrap();
        sw_el.style().set_css_text(&format!(
            "width: 14px; height: 14px; border-radius: 50%; background: {}; \
             border: 1px solid var(--border-medium);",
            color,
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

        let type_badge = document.create_element("span").unwrap();
        type_badge.set_text_content(Some(ltype));
        let tb_el2: HtmlElement = type_badge.clone().dyn_into().unwrap();
        let type_color = match *ltype {
            "directional" => "rgba(255, 220, 100, 0.8)",
            "point" => "rgba(100, 200, 100, 0.8)",
            "spot" => "rgba(0, 200, 255, 0.8)",
            "ambient" => "rgba(200, 150, 255, 0.8)",
            _ => "var(--text-muted)",
        };
        tb_el2.style().set_css_text(&format!(
            "font-size: 8px; color: {}; font-family: var(--font-mono); font-weight: 600;",
            type_color,
        ));
        hdr.append_child(&type_badge).unwrap();

        let state_badge = document.create_element("span").unwrap();
        state_badge.set_text_content(Some(state));
        let sb_el: HtmlElement = state_badge.clone().dyn_into().unwrap();
        let state_color = if *state == "On" {
            "rgba(100, 200, 100, 0.8)"
        } else {
            "var(--text-muted)"
        };
        sb_el.style().set_css_text(&format!(
            "margin-left: auto; font-size: 8px; color: {}; font-family: var(--font-mono); \
             font-weight: 700;",
            state_color,
        ));
        hdr.append_child(&state_badge).unwrap();
        card.append_child(&hdr).unwrap();

        // Params
        let params = document.create_element("div").unwrap();
        let p_el: HtmlElement = params.clone().dyn_into().unwrap();
        p_el.style().set_css_text(
            "display: grid; grid-template-columns: repeat(2, 1fr); gap: 4px; padding: 6px 8px;",
        );

        let param_rows: Vec<(String, String)> = vec![
            ("Colour".into(), color.to_string()),
            ("Intensity".into(), format!("{:.2}", intensity)),
            ("Falloff".into(), format!("{:.1}m", falloff)),
            ("Direction / Pos".into(), direction.to_string()),
        ];

        for (label, value) in &param_rows {
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
            v_el.style().set_css_text(
                "font-size: 9px; color: var(--text-primary); font-family: var(--font-mono); \
                 font-weight: 600;",
            );
            row.append_child(&v).unwrap();
            p_el.append_child(&row).unwrap();
        }
        card.append_child(&params).unwrap();

        // Intensity bar
        let bar_area = document.create_element("div").unwrap();
        let ba_el: HtmlElement = bar_area.clone().dyn_into().unwrap();
        ba_el.style().set_css_text("padding: 0 8px 6px;");

        let bar_bg = document.create_element("div").unwrap();
        let bb_el: HtmlElement = bar_bg.clone().dyn_into().unwrap();
        bb_el.style().set_css_text(
            "height: 4px; background: var(--surface-bg); border-radius: 2px; position: relative;",
        );
        let bar_fill = document.create_element("div").unwrap();
        let bf_el: HtmlElement = bar_fill.clone().dyn_into().unwrap();
        bf_el.style().set_css_text(&format!(
            "position: absolute; left: 0; top: 0; bottom: 0; width: {}%; \
             background: {}; border-radius: 2px;",
            (intensity / 2.0 * 100.0).min(100.0),
            color,
        ));
        bar_bg.append_child(&bar_fill).unwrap();
        bar_area.append_child(&bar_bg).unwrap();
        card.append_child(&bar_area).unwrap();

        content.append_child(&card).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} lighting editor requires R3D-7 lighting engine.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
