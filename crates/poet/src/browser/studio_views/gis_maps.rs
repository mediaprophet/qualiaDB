//! GIS Maps — topographic GIS terrain mesh + map tiles (§2.1, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const MAP_LAYERS: &[(&str, &str, bool)] = &[
    ("Terrain DEM", "Elevation heightmap mesh", true),
    ("Satellite Tiles", "WMS/XYZ tile service", true),
    ("Road Network", "GeoSPARQL line features", false),
    ("Building Footprints", "OSM polygon extrusions", true),
    ("Water Bodies", "Hydrology polygons", true),
    ("Contour Lines", "Isoline overlay (10m interval)", false),
    ("Coordinate Grid", "Lat/Lon grid lines", true),
    ("Place Labels", "Gazetteer point labels", true),
];

const MAP_INFO: &[(&str, &str)] = &[
    ("Projection", "Web Mercator (EPSG:3857)"),
    ("Centre", "-33.8688, 151.2093 (Sydney)"),
    ("Zoom", "14"),
    ("Tile Size", "256px"),
    ("DEM Resolution", "30m (SRTM)"),
    ("GeoSPARQL Endpoint", "qualia-db/sparql"),
    ("Sensitivity", "Public"),
];

pub fn build_gis_maps_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         flex-wrap: wrap;",
    );
    for label in &[
        "Import DEM",
        "Add Tile Layer",
        "GeoSPARQL Query",
        "Export GeoJSON",
        "3D Extrude",
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

    // Map viewport placeholder
    let viewport = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport.clone().dyn_into().unwrap();
    vp_el.style().set_css_text(
        "height: 140px; background: var(--surface-panel); border-radius: 6px; \
         margin-bottom: 8px; display: flex; align-items: center; justify-content: center; \
         border: 1px solid var(--border-subtle); position: relative;",
    );
    let placeholder = document.create_element("div").unwrap();
    placeholder.set_text_content(Some(
        "GIS Viewport\n\
         Terrain mesh + map tiles (wgpu)\n\
         Centre: -33.87, 151.21  |  Zoom: 14",
    ));
    let p_el: HtmlElement = placeholder.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "text-align: center; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); white-space: pre-wrap; line-height: 1.6;",
    );
    viewport.append_child(&placeholder).unwrap();

    // Zoom controls
    let zoom_in = document.create_element("div").unwrap();
    zoom_in.set_text_content(Some("+"));
    let zi_el: HtmlElement = zoom_in.clone().dyn_into().unwrap();
    zi_el.style().set_css_text(
        "position: absolute; right: 6px; top: 6px; width: 20px; height: 20px; \
         background: var(--surface-bg); border-radius: 3px; display: flex; \
         align-items: center; justify-content: center; font-size: 14px; \
         color: var(--text-secondary); cursor: pointer; border: 1px solid var(--border-medium);",
    );
    viewport.append_child(&zoom_in).unwrap();

    let zoom_out = document.create_element("div").unwrap();
    zoom_out.set_text_content(Some("\u{2212}"));
    let zo_el: HtmlElement = zoom_out.clone().dyn_into().unwrap();
    zo_el.style().set_css_text(
        "position: absolute; right: 6px; top: 30px; width: 20px; height: 20px; \
         background: var(--surface-bg); border-radius: 3px; display: flex; \
         align-items: center; justify-content: center; font-size: 14px; \
         color: var(--text-secondary); cursor: pointer; border: 1px solid var(--border-medium);",
    );
    viewport.append_child(&zoom_out).unwrap();
    content.append_child(&viewport).unwrap();

    // Map info
    let info_header = document.create_element("div").unwrap();
    info_header.set_text_content(Some("Map Info"));
    let ih_el: HtmlElement = info_header.clone().dyn_into().unwrap();
    ih_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&info_header).unwrap();

    let info_table = make_table(document, &["Field", "Value"]);
    let info_tbody = document.create_element("tbody").unwrap();
    for (field, value) in MAP_INFO {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![field.to_string(), value.to_string()];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        info_tbody.append_child(&tr).unwrap();
    }
    info_table.append_child(&info_tbody).unwrap();
    content.append_child(&info_table).unwrap();

    // Layers
    let layers_header = document.create_element("div").unwrap();
    layers_header.set_text_content(Some("Map Layers"));
    let lh_el: HtmlElement = layers_header.clone().dyn_into().unwrap();
    lh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&layers_header).unwrap();

    let layers_table = make_table(document, &["Layer", "Description", "Visible"]);
    let layers_tbody = document.create_element("tbody").unwrap();
    for (name, desc, visible) in MAP_LAYERS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            desc.to_string(),
            if *visible { "On" } else { "Off" }.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = if *visible {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 9px; font-weight: 600; text-align: center;",
                    color,
                ));
            } else if i == 0 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 9px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        layers_tbody.append_child(&tr).unwrap();
    }
    layers_table.append_child(&layers_tbody).unwrap();
    content.append_child(&layers_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} GIS maps require R3D-14..16 GIS engine + GeoSPARQL endpoint.",
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
