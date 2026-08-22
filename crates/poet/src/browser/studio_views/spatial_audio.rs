//! Spatial Audio — 3D sound source placement + room + listener editor (§6, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("scene", "Scene"),
    ("room", "Room"),
    ("listener", "Listener"),
];

const SOURCES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "SRC-001",
        "Field Recording A",
        "point",
        "(5, 2, -3)",
        "omni",
    ),
    ("SRC-002", "Ambient Bed", "ambient", "(0, 5, 0)", "omni"),
    (
        "SRC-003",
        "Voiceover",
        "directional",
        "(-3, 1, 2)",
        "cardioid",
    ),
    ("SRC-004", "Music Bed", "ambient", "(0, 3, 0)", "omni"),
    (
        "SRC-005",
        "Tensor Sonification",
        "point",
        "(8, 0, 5)",
        "figure-8",
    ),
];

pub fn build_spatial_audio_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    // Toolbar
    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         flex-wrap: wrap;",
    );
    for label in &[
        "+ Point Source",
        "+ Directional",
        "+ Ambient",
        "+ Ambisonic",
        "Set Listener",
        "Sonify Tensor",
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

    // Render mode selector
    let render_sel = document.create_element("select").unwrap();
    let rs_el: HtmlElement = render_sel.clone().dyn_into().unwrap();
    rs_el.style().set_css_text(
        "font-size: 8px; font-family: var(--font-mono); background: transparent; \
         color: var(--text-secondary); border: 1px solid var(--border-medium); \
         border-radius: 3px; padding: 1px 4px; margin-left: auto;",
    );
    for mode in &[
        "Binaural (HRTF)",
        "Ambisonic (HOA)",
        "MultiChannel 5.1",
        "MultiChannel 7.1",
    ] {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", mode).unwrap();
        opt.set_text_content(Some(mode));
        render_sel.append_child(&opt).unwrap();
    }
    toolbar.append_child(&render_sel).unwrap();
    wrapper.append_child(&toolbar).unwrap();

    // Tab bar
    let tab_bar = build_tab_bar(document);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Scene tab
    let scene_panel = build_scene_tab(document);
    content.append_child(&scene_panel).unwrap();

    // Room tab
    let room_panel = build_room_tab(document);
    let rp_el: HtmlElement = room_panel.clone().dyn_into().unwrap();
    rp_el.style().set_css_text("display: none;");
    content.append_child(&room_panel).unwrap();

    // Listener tab
    let listener_panel = build_listener_tab(document);
    let lp_el: HtmlElement = listener_panel.clone().dyn_into().unwrap();
    lp_el.style().set_css_text("display: none;");
    content.append_child(&listener_panel).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} spatial audio requires IMM-1..IMM-4 engine + HRTF/Ambisonic decoder.",
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
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-spatial-tab", tab_id).unwrap();
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

fn build_scene_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-spatial-panel", "scene").unwrap();

    // 3D viewport placeholder
    let viewport = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport.clone().dyn_into().unwrap();
    vp_el.style().set_css_text(
        "height: 120px; background: var(--surface-panel); border-radius: 6px; \
         display: flex; align-items: center; justify-content: center; \
         margin-bottom: 8px; position: relative;",
    );
    let placeholder = document.create_element("div").unwrap();
    placeholder.set_text_content(Some(
        "3D Viewport \u{2014} source markers + listener head icon\n\
         (reuses 3D container wgpu surface)",
    ));
    let p_el: HtmlElement = placeholder.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "text-align: center; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); white-space: pre-wrap;",
    );
    viewport.append_child(&placeholder).unwrap();
    panel.append_child(&viewport).unwrap();

    // Source list
    let table = make_table(
        document,
        &["ID", "Name", "Kind", "Position (x,y,z)", "Directivity"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, name, kind, pos, directivity) in SOURCES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, name, kind, pos, directivity].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "point" => "rgba(0, 200, 255, 0.8)",
                    "directional" => "rgba(255, 165, 0, 0.8)",
                    "ambient" => "rgba(100, 200, 100, 0.8)",
                    "ambisonic" => "rgba(200, 150, 255, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 9px; font-weight: 600;",
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
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    panel
}

fn build_room_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-spatial-panel", "room").unwrap();

    let fields: &[(&str, &str, &str)] = &[
        (
            "Room Kind",
            "Sabine",
            "select: Sabine / RayTraced / Analytic",
        ),
        ("Width", "8.0 m", "numeric"),
        ("Length", "10.0 m", "numeric"),
        ("Height", "3.5 m", "numeric"),
        ("RT60 (reverb time)", "0.45 s", "derived/manual"),
        ("Early Reflections", "On (12 reflections)", "toggle + count"),
        (
            "Late Reverb",
            "Convolution (RIR)",
            "select: Convolution / Sabine",
        ),
        ("Absorption (125 Hz)", "0.12", "per-octave-band"),
        ("Absorption (500 Hz)", "0.18", "per-octave-band"),
        ("Absorption (2 kHz)", "0.25", "per-octave-band"),
    ];

    let table = make_table(document, &["Field", "Value", "Type"]);
    let tbody = document.create_element("tbody").unwrap();
    for (field, value, ftype) in fields {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [field, value, ftype].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-weight: 600; \
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
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    panel
}

fn build_listener_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-spatial-panel", "listener")
        .unwrap();

    let fields: &[(&str, &str, &str)] = &[
        ("Position", "(0, 1.7, 0)", "x, y, z in manifold space"),
        ("Orientation (Yaw)", "0\u{00B0}", "degrees"),
        ("Orientation (Pitch)", "0\u{00B0}", "degrees"),
        ("Orientation (Roll)", "0\u{00B0}", "degrees"),
        ("Head Tracking", "Off", "toggle: device sensor / manual"),
        (
            "HRTF Set",
            "MIT KEMAR (default)",
            "select: default / personalized",
        ),
        (
            "Output Config",
            "Stereo",
            "select: Stereo / 5.1 / 7.1 / 22.2 / Ambisonic AmbiX/FuMa",
        ),
    ];

    let table = make_table(document, &["Field", "Value", "Notes"]);
    let tbody = document.create_element("tbody").unwrap();
    for (field, value, notes) in fields {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [field, value, notes].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-weight: 600; \
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
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    panel
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
