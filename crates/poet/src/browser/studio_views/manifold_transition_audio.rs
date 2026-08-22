//! Manifold Transition Audio — crossfade / ambience between manifolds (§6.3, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TRANSITIONS: &[(&str, &str, &str, f64, &str)] = &[
    (
        "Research \u{2192} Studio",
        "Crossfade",
        "2.0s",
        0.5,
        "Active",
    ),
    (
        "Studio \u{2192} Health",
        "Ambience fade",
        "3.0s",
        0.0,
        "Idle",
    ),
    (
        "Health \u{2192} Knowledge",
        "Direct cut",
        "0.0s",
        0.0,
        "Idle",
    ),
    (
        "Knowledge \u{2192} Communications",
        "Crossfade",
        "1.5s",
        0.0,
        "Idle",
    ),
    (
        "Communications \u{2192} Social",
        "Ambience fade",
        "2.5s",
        0.0,
        "Idle",
    ),
    (
        "Social \u{2192} Studio",
        "Crossfade + reverb tail",
        "3.0s",
        0.0,
        "Idle",
    ),
    (
        "Studio \u{2192} Datasets",
        "Direct cut",
        "0.0s",
        0.0,
        "Idle",
    ),
    (
        "Datasets \u{2192} Sanctuary",
        "Long fade",
        "5.0s",
        0.0,
        "Idle",
    ),
];

const AMBIENCE: &[(&str, &str, f64, bool)] = &[
    ("Research", "Library ambience (low hum)", 0.15, true),
    ("Studio", "Studio room tone (silent)", 0.0, false),
    ("Health", "Clinical ambience (soft)", 0.08, true),
    ("Knowledge", "Library ambience (low hum)", 0.15, true),
    ("Communications", "Office ambience (muted)", 0.05, true),
    ("Social", "Cafe ambience (muted)", 0.05, true),
    ("Datasets", "Silent", 0.0, false),
    ("Sanctuary", "Nature sounds (soft)", 0.20, true),
];

pub fn build_manifold_transition_audio_view(document: &Document) -> Element {
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
        "+ Transition",
        "Test Transition",
        "Enable Ambience",
        "Export Config",
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

    // Current transition
    let current = document.create_element("div").unwrap();
    current.set_text_content(Some(
        "Current Transition: Research \u{2192} Studio  |  Type: Crossfade  |  Duration: 2.0s  |  \
         Progress: 50%  |  Audio: Fading out research ambience, fading in studio room tone",
    ));
    let cu_el: HtmlElement = current.clone().dyn_into().unwrap();
    cu_el.style().set_css_text(
        "padding: 6px 8px; background: var(--surface-panel); border-radius: 4px; \
         margin-bottom: 8px; font-size: 9px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    content.append_child(&current).unwrap();

    // Transition progress bar
    let bar_bg = document.create_element("div").unwrap();
    let bb_el: HtmlElement = bar_bg.clone().dyn_into().unwrap();
    bb_el.style().set_css_text(
        "height: 8px; background: var(--surface-bg); border-radius: 4px; \
         margin-bottom: 8px; position: relative; overflow: hidden;",
    );
    let bar_fill = document.create_element("div").unwrap();
    let bf_el: HtmlElement = bar_fill.clone().dyn_into().unwrap();
    bf_el.style().set_css_text(
        "position: absolute; left: 0; top: 0; bottom: 0; width: 50%; \
         background: linear-gradient(to right, rgba(0,200,255,0.4), rgba(100,200,100,0.4)); \
         border-radius: 4px;",
    );
    bar_bg.append_child(&bar_fill).unwrap();
    content.append_child(&bar_bg).unwrap();

    // Transitions table
    let trans_header = document.create_element("div").unwrap();
    trans_header.set_text_content(Some("Manifold Transitions (8)"));
    let th_el: HtmlElement = trans_header.clone().dyn_into().unwrap();
    th_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&trans_header).unwrap();

    let trans_table = make_table(
        document,
        &["Route", "Type", "Duration", "Progress", "Status"],
    );
    let trans_tbody = document.create_element("tbody").unwrap();
    for (route, ttype, duration, progress, status) in TRANSITIONS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            route.to_string(),
            ttype.to_string(),
            duration.to_string(),
            format!("{:.0}%", progress * 100.0),
            status.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = if *status == "Active" {
                    "rgba(0, 200, 255, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 1 {
                let color = match &**ttype {
                    "Crossfade" | "Crossfade + reverb tail" => "rgba(0, 200, 255, 0.6)",
                    "Ambience fade" | "Long fade" => "rgba(100, 200, 100, 0.6)",
                    "Direct cut" => "var(--text-muted)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-family: var(--font-mono);",
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
        trans_tbody.append_child(&tr).unwrap();
    }
    trans_table.append_child(&trans_tbody).unwrap();
    content.append_child(&trans_table).unwrap();

    // Ambience settings
    let amb_header = document.create_element("div").unwrap();
    amb_header.set_text_content(Some("Per-Manifold Ambience (8)"));
    let ah_el: HtmlElement = amb_header.clone().dyn_into().unwrap();
    ah_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&amb_header).unwrap();

    let amb_table = make_table(document, &["Manifold", "Ambience", "Volume", "Enabled"]);
    let amb_tbody = document.create_element("tbody").unwrap();
    for (manifold, desc, volume, enabled) in AMBIENCE {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            manifold.to_string(),
            desc.to_string(),
            format!("{:.2}", volume),
            if *enabled { "On" } else { "Off" }.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = if *enabled {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; text-align: center;",
                    color,
                ));
            } else if i == 2 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        amb_tbody.append_child(&tr).unwrap();
    }
    amb_table.append_child(&amb_tbody).unwrap();
    content.append_child(&amb_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} manifold transition audio requires AUD-18 engine.",
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
