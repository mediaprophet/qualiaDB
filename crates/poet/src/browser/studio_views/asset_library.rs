//! Asset Library — imported meshes + .10d containers + tensors (§2.2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const ASSETS: &[(&str, &str, &str, &str, &str, &str)] = &[
    ("house.glb", "mesh", "GLB", "12.4 MB", "LOD 2", "Public"),
    ("terrain.dem", "mesh", "GIS", "45.2 MB", "LOD 1", "Public"),
    (
        "tensor_field_a.10d",
        "tensor",
        "10D",
        "128 MB",
        "LOD 0",
        "Restricted",
    ),
    ("character.fbx", "mesh", "FBX", "8.7 MB", "LOD 1", "Public"),
    (
        "particles.10d",
        "particles",
        "10D",
        "2.1 MB",
        "LOD 0",
        "Public",
    ),
    (
        "scan_volume.dcm",
        "volume",
        "DICOM",
        "256 MB",
        "LOD 0",
        "Restricted",
    ),
    ("cad_model.step", "cad", "STEP", "3.5 MB", "LOD 0", "Public"),
    (
        "animation_walk.glb",
        "animation",
        "GLB",
        "1.8 MB",
        "N/A",
        "Public",
    ),
];

pub fn build_asset_library_view(document: &Document) -> Element {
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
    for label in &["Import", "Search", "Sort by Name", "Sort by Size"] {
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

    // Asset grid
    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style()
        .set_css_text("display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px;");

    for (name, atype, fmt, size, lod, sens) in ASSETS {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "padding: 8px; background: var(--surface-panel); border-radius: 6px; \
             border: 1px solid var(--border-subtle); cursor: pointer;",
        );

        // Icon area
        let icon_area = document.create_element("div").unwrap();
        let icon = match *atype {
            "mesh" => "\u{1F4D8}",
            "tensor" => "\u{1F321}",
            "particles" => "\u{2728}",
            "volume" => "\u{1F4CD}",
            "cad" => "\u{1F4D0}",
            "animation" => "\u{23F1}",
            _ => "\u{1F4C1}",
        };
        icon_area.set_text_content(Some(icon));
        let ia_el: HtmlElement = icon_area.clone().dyn_into().unwrap();
        ia_el
            .style()
            .set_css_text("font-size: 20px; text-align: center; margin-bottom: 4px;");
        card.append_child(&icon_area).unwrap();

        // Name
        let name_div = document.create_element("div").unwrap();
        name_div.set_text_content(Some(name));
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 9px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono); text-align: center; \
             white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
        );
        card.append_child(&name_div).unwrap();

        // Type badge
        let type_div = document.create_element("div").unwrap();
        type_div.set_text_content(Some(atype));
        let t_el: HtmlElement = type_div.clone().dyn_into().unwrap();
        let type_color = match *atype {
            "mesh" => "rgba(100, 200, 100, 0.6)",
            "tensor" => "rgba(255, 165, 0, 0.6)",
            "particles" => "rgba(255, 100, 200, 0.6)",
            "volume" => "rgba(255, 100, 100, 0.6)",
            "cad" => "rgba(0, 200, 255, 0.6)",
            "animation" => "rgba(200, 150, 255, 0.6)",
            _ => "var(--text-muted)",
        };
        t_el.style().set_css_text(&format!(
            "font-size: 7px; color: {}; font-family: var(--font-mono); \
             text-align: center; text-transform: uppercase; margin-top: 2px;",
            type_color,
        ));
        card.append_child(&type_div).unwrap();

        // Size + LOD
        let info = document.create_element("div").unwrap();
        info.set_text_content(Some(&format!("{}  |  {}", size, lod)));
        let i_el: HtmlElement = info.clone().dyn_into().unwrap();
        i_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             text-align: center; margin-top: 2px;",
        );
        card.append_child(&info).unwrap();

        // Sensitivity badge
        let sens_div = document.create_element("div").unwrap();
        sens_div.set_text_content(Some(sens));
        let s_el: HtmlElement = sens_div.clone().dyn_into().unwrap();
        let sens_color = match *sens {
            "Public" => "rgba(100, 200, 100, 0.8)",
            "Restricted" => "rgba(255, 165, 0, 0.8)",
            _ => "var(--text-muted)",
        };
        s_el.style().set_css_text(&format!(
            "font-size: 7px; color: {}; font-family: var(--font-mono); \
             text-align: center; margin-top: 2px; font-weight: 600;",
            sens_color,
        ));
        card.append_child(&sens_div).unwrap();

        g_el.append_child(&card).unwrap();
    }
    content.append_child(&grid).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} asset library requires render/assets.rs import engine.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
