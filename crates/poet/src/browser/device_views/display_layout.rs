//! Display Layout — multi-monitor display arrangement and container placement (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const DISPLAYS: &[(&str, &str, u32, u32, f32, bool, i32, i32)] = &[
    (
        "display-1",
        "Left Monitor (4K)",
        3840,
        2160,
        1.5,
        false,
        -3840,
        0,
    ),
    ("display-2", "Primary (4K)", 3840, 2160, 1.5, true, 0, 0),
    (
        "display-3",
        "Right Monitor (FHD)",
        1920,
        1080,
        1.0,
        false,
        3840,
        0,
    ),
    (
        "display-laptop",
        "Laptop Screen",
        2560,
        1600,
        2.0,
        false,
        0,
        2160,
    ),
];

const CONTAINER_PLACEMENTS: &[(&str, &str, &str)] = &[
    ("graph_canvas", "Semantic Graph", "display-2"),
    ("ontology_library", "Ontology Library", "display-1"),
    ("n3_editor", "N3 Editor", "display-1"),
    ("vocabulary_mapper", "Vocabulary Mapper", "display-3"),
    ("relation_builder", "Relation Builder", "display-3"),
    ("shacl_shapes", "SHACL Shapes", "display-2"),
    ("inspector", "Inspector", "display-laptop"),
    ("pulse-panel", "Pulse Stream", "display-laptop"),
];

pub fn build_display_layout_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         align-items: center; flex-wrap: wrap;",
    );

    for label in &[
        "Detect Displays",
        "Arrange Auto",
        "Save Layout",
        "Extend to Laptop",
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

    let spacer = document.create_element("div").unwrap();
    let sp_el: HtmlElement = spacer.clone().dyn_into().unwrap();
    sp_el.style().set_css_text("flex: 1;");
    toolbar.append_child(&spacer).unwrap();

    let info = document.create_element("span").unwrap();
    info.set_text_content(Some("4 displays | 12,128x3260 virtual | 3 devices"));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style()
        .set_css_text("font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);");
    toolbar.append_child(&info).unwrap();
    wrapper.append_child(&toolbar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Visual display map
    let map_label = document.create_element("div").unwrap();
    map_label.set_text_content(Some("Display Map"));
    let ml_el: HtmlElement = map_label.clone().dyn_into().unwrap();
    ml_el.style().set_css_text(
        "font-size: 8px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; margin-bottom: 4px;",
    );
    content.append_child(&map_label).unwrap();

    let map = document.create_element("div").unwrap();
    let m_el: HtmlElement = map.clone().dyn_into().unwrap();
    m_el.style().set_css_text(
        "position: relative; width: 100%; height: 120px; background: var(--surface-bg); \
         border: 1px solid var(--border-subtle); border-radius: 4px; margin-bottom: 8px; \
         overflow: hidden;",
    );

    // Virtual coordinate space: x from -3840 to 5760, y from 0 to 3760
    // Scale to fit: width 100%, height 120px
    let virt_w = 9600.0f32;
    let virt_h = 3760.0f32;

    for (did, label, w, h, scale, is_primary, vx, vy) in DISPLAYS {
        let disp = document.create_element("div").unwrap();
        let d_el: HtmlElement = disp.clone().dyn_into().unwrap();
        let left_pct = ((vx + 3840) as f32 / virt_w) * 100.0;
        let top_pct = (*vy as f32 / virt_h) * 100.0;
        let width_pct = (*w as f32 / virt_w) * 100.0;
        let height_pct = (*h as f32 / virt_h) * 100.0;
        let border = if *is_primary {
            "2px solid var(--accent-cyan)"
        } else {
            "1px solid var(--border-medium)"
        };
        let bg = if *is_primary {
            "rgba(0, 200, 255, 0.05)"
        } else {
            "var(--surface-panel)"
        };
        d_el.style().set_css_text(&format!(
            "position: absolute; left: {}%; top: {}%; width: {}%; height: {}%; \
             border: {}; background: {}; border-radius: 3px; display: flex; \
             flex-direction: column; justify-content: center; align-items: center; \
             overflow: hidden;",
            left_pct, top_pct, width_pct, height_pct, border, bg,
        ));

        let name = document.create_element("div").unwrap();
        name.set_text_content(Some(label));
        let n_el: HtmlElement = name.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 7px; color: var(--text-primary); font-family: var(--font-mono); \
             font-weight: 600; text-align: center; padding: 2px;",
        );
        disp.append_child(&name).unwrap();

        let res = document.create_element("div").unwrap();
        res.set_text_content(Some(&format!("{}x{} @ {}x", w, h, scale)));
        let r_el: HtmlElement = res.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "font-size: 6px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        disp.append_child(&res).unwrap();

        // Container count on this display
        let count = CONTAINER_PLACEMENTS
            .iter()
            .filter(|(_, _, d)| *d == *did)
            .count();
        if count > 0 {
            let count_div = document.create_element("div").unwrap();
            count_div.set_text_content(Some(&format!("{} containers", count)));
            let cn_el: HtmlElement = count_div.clone().dyn_into().unwrap();
            cn_el.style().set_css_text(
                "font-size: 6px; color: var(--accent-cyan); font-family: var(--font-mono);",
            );
            disp.append_child(&count_div).unwrap();
        }

        map.append_child(&disp).unwrap();
    }
    content.append_child(&map).unwrap();

    // Container placements table
    let placements_label = document.create_element("div").unwrap();
    placements_label.set_text_content(Some("Container Placements (8)"));
    let pl_el: HtmlElement = placements_label.clone().dyn_into().unwrap();
    pl_el.style().set_css_text(
        "font-size: 8px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; margin-top: 8px; margin-bottom: 4px;",
    );
    content.append_child(&placements_label).unwrap();

    let table = make_table(document, &["Container", "Display", "Status"]);
    let tbody = document.create_element("tbody").unwrap();
    for (ctype, title, display) in CONTAINER_PLACEMENTS {
        let tr = document.create_element("tr").unwrap();
        let vals = vec![
            format!("{}: {}", ctype, title),
            display.to_string(),
            "Placed".to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: rgba(100, 200, 100, 0.8); font-size: 7px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 8px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} display layout requires multi-window + virtual desktop API.",
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
