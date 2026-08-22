//! Ragdoll / Skin — skeleton + skin weights + physics joints (§3.2, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const BONES: &[(&str, &str, &str, f64)] = &[
    ("Root", "root", "(0, 0, 0)", 0.0),
    ("Spine", "bone", "(0, 0.4, 0)", 0.0),
    ("Chest", "bone", "(0, 0.7, 0)", 0.0),
    ("Head", "bone", "(0, 1.0, 0)", 0.0),
    ("L Shoulder", "bone", "(0.2, 0.7, 0)", 15.0),
    ("L Upper Arm", "bone", "(0.4, 0.7, 0)", 25.0),
    ("L Forearm", "bone", "(0.6, 0.7, 0)", 20.0),
    ("R Shoulder", "bone", "(-0.2, 0.7, 0)", 15.0),
    ("R Upper Arm", "bone", "(-0.4, 0.7, 0)", 25.0),
    ("R Forearm", "bone", "(-0.6, 0.7, 0)", 20.0),
    ("L Thigh", "bone", "(0.15, 0, 0)", 30.0),
    ("L Shin", "bone", "(0.15, -0.5, 0)", 25.0),
    ("R Thigh", "bone", "(-0.15, 0, 0)", 30.0),
    ("R Shin", "bone", "(-0.15, -0.5, 0)", 25.0),
];

const JOINTS: &[(&str, &str, &str, &str)] = &[
    ("Spine", "slide", "Z-axis", "0.0 - 0.8m"),
    ("Chest", "spin", "Y-axis", "0\u{00B0} - 45\u{00B0}"),
    ("Neck", "spin", "all-axis", "0\u{00B0} - 60\u{00B0}"),
    ("L Shoulder", "ball", "3DOF", "0\u{00B0} - 90\u{00B0}"),
    ("R Shoulder", "ball", "3DOF", "0\u{00B0} - 90\u{00B0}"),
    ("L Elbow", "spin", "X-axis", "0\u{00B0} - 135\u{00B0}"),
    ("R Elbow", "spin", "X-axis", "0\u{00B0} - 135\u{00B0}"),
    ("L Hip", "ball", "3DOF", "0\u{00B0} - 120\u{00B0}"),
    ("R Hip", "ball", "3DOF", "0\u{00B0} - 120\u{00B0}"),
    ("L Knee", "spin", "X-axis", "0\u{00B0} - 150\u{00B0}"),
    ("R Knee", "spin", "X-axis", "0\u{00B0} - 150\u{00B0}"),
];

pub fn build_ragdoll_skin_view(document: &Document) -> Element {
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
        "+ Bone",
        "+ Joint",
        "Paint Weights",
        "Simulate",
        "Reset Pose",
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

    // Skeleton viewport placeholder
    let viewport = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport.clone().dyn_into().unwrap();
    vp_el.style().set_css_text(
        "height: 100px; background: var(--surface-panel); border-radius: 6px; \
         margin-bottom: 8px; display: flex; align-items: center; justify-content: center; \
         border: 1px solid var(--border-subtle);",
    );
    let ph = document.create_element("div").unwrap();
    ph.set_text_content(Some(
        "Skeleton Viewport \u{2014} bone hierarchy + skin weight heatmap (not wired)",
    ));
    let p_el: HtmlElement = ph.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);");
    viewport.append_child(&ph).unwrap();
    content.append_child(&viewport).unwrap();

    // Bones table
    let bones_header = document.create_element("div").unwrap();
    bones_header.set_text_content(Some("Skeleton Bones (14)"));
    let bh_el: HtmlElement = bones_header.clone().dyn_into().unwrap();
    bh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&bones_header).unwrap();

    let bones_table = make_table(document, &["Bone", "Type", "Position", "Weight (verts)"]);
    let bones_tbody = document.create_element("tbody").unwrap();
    for (name, btype, pos, weight) in BONES {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            btype.to_string(),
            pos.to_string(),
            format!("{:.0}", weight),
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
            } else if i == 3 {
                let color = if *weight > 0.0 {
                    "rgba(255, 165, 0, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 9px; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        bones_tbody.append_child(&tr).unwrap();
    }
    bones_table.append_child(&bones_tbody).unwrap();
    content.append_child(&bones_table).unwrap();

    // Joints table
    let joints_header = document.create_element("div").unwrap();
    joints_header.set_text_content(Some("Physics Joints (11)"));
    let jh_el: HtmlElement = joints_header.clone().dyn_into().unwrap();
    jh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&joints_header).unwrap();

    let joints_table = make_table(document, &["Joint", "Type", "Axis", "Range"]);
    let joints_tbody = document.create_element("tbody").unwrap();
    for (name, jtype, axis, range) in JOINTS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            jtype.to_string(),
            axis.to_string(),
            range.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                let color = match &**jtype {
                    "ball" => "rgba(255, 100, 200, 0.6)",
                    "spin" => "rgba(0, 200, 255, 0.6)",
                    "slide" => "rgba(100, 200, 100, 0.6)",
                    _ => "var(--text-muted)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        joints_tbody.append_child(&tr).unwrap();
    }
    joints_table.append_child(&joints_tbody).unwrap();
    content.append_child(&joints_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} ragdoll/skin requires ANI-9 skeleton + physics joint engine.",
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
