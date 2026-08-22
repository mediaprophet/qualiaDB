//! Animation Timeline — keyframe/clip/track/mixer/player UI (§3.1, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TRACKS: &[(&str, &str, &str)] = &[
    ("Camera", "position", "3 keyframes"),
    ("House Model", "rotation", "5 keyframes"),
    ("House Model", "scale", "2 keyframes"),
    ("Sun Light", "intensity", "4 keyframes"),
    ("Terrain", "opacity", "2 keyframes"),
];

const KEYFRAMES: &[(&str, f64, &str, &str)] = &[
    ("Camera", 0.0, "(0, 5, 20)", "Linear"),
    ("Camera", 2.5, "(10, 8, 15)", "Hermite"),
    ("Camera", 5.0, "(0, 5, 20)", "Linear"),
    ("House Model", 0.0, "0\u{00B0}", "Step"),
    ("House Model", 1.0, "45\u{00B0}", "Linear"),
    ("House Model", 2.0, "90\u{00B0}", "Linear"),
    ("House Model", 3.0, "135\u{00B0}", "Linear"),
    ("House Model", 5.0, "180\u{00B0}", "Linear"),
    ("Sun Light", 0.0, "0.8", "Linear"),
    ("Sun Light", 1.5, "1.0", "Linear"),
    ("Sun Light", 3.0, "0.6", "Linear"),
    ("Sun Light", 5.0, "0.2", "Linear"),
];

const CLIPS: &[(&str, &str, &str, &str)] = &[
    ("Walkthrough", "Camera", "0.0 - 5.0s", "Loop"),
    ("Sun Arc", "Sun Light", "0.0 - 5.0s", "Once"),
    ("Rotate House", "House Model", "0.0 - 5.0s", "PingPong"),
];

pub fn build_animation_timeline_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    // Transport bar
    let transport = document.create_element("div").unwrap();
    let t_el: HtmlElement = transport.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "display: flex; align-items: center; gap: 6px; padding: 4px 8px; \
         border-bottom: 1px solid var(--border-subtle);",
    );

    for label in &[
        "\u{23F5} Play",
        "\u{23F8} Pause",
        "\u{23F9} Stop",
        "\u{23CF} Record",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 8px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 9px; font-family: var(--font-mono);",
        );
        transport.append_child(&btn).unwrap();
    }

    let time_display = document.create_element("span").unwrap();
    time_display.set_text_content(Some("00:02.350 / 00:05.000"));
    let td_el: HtmlElement = time_display.clone().dyn_into().unwrap();
    td_el.style().set_css_text(
        "font-size: 10px; color: var(--accent-cyan); font-family: var(--font-mono); \
         font-weight: 600; margin-left: auto;",
    );
    transport.append_child(&time_display).unwrap();

    let speed_sel = document.create_element("select").unwrap();
    let ss_el: HtmlElement = speed_sel.clone().dyn_into().unwrap();
    ss_el.style().set_css_text(
        "font-size: 9px; font-family: var(--font-mono); background: transparent; \
         color: var(--text-secondary); border: 1px solid var(--border-medium); \
         border-radius: 3px; padding: 1px 4px;",
    );
    for speed in &["0.25x", "0.5x", "1x", "2x", "4x"] {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", speed).unwrap();
        opt.set_text_content(Some(speed));
        speed_sel.append_child(&opt).unwrap();
    }
    transport.append_child(&speed_sel).unwrap();

    let loop_sel = document.create_element("select").unwrap();
    let ls_el: HtmlElement = loop_sel.clone().dyn_into().unwrap();
    ls_el.style().set_css_text(
        "font-size: 9px; font-family: var(--font-mono); background: transparent; \
         color: var(--text-secondary); border: 1px solid var(--border-medium); \
         border-radius: 3px; padding: 1px 4px;",
    );
    for mode in &["Once", "Loop", "PingPong"] {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", mode).unwrap();
        opt.set_text_content(Some(mode));
        loop_sel.append_child(&opt).unwrap();
    }
    transport.append_child(&loop_sel).unwrap();
    wrapper.append_child(&transport).unwrap();

    // Timeline tracks area
    let timeline_area = document.create_element("div").unwrap();
    let ta_el: HtmlElement = timeline_area.clone().dyn_into().unwrap();
    ta_el
        .style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px 8px;");

    // Time ruler
    let ruler = document.create_element("div").unwrap();
    let r_el: HtmlElement = ruler.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "display: flex; height: 16px; border-bottom: 1px solid var(--border-subtle); \
         margin-bottom: 4px; position: relative;",
    );
    for i in 0..=10 {
        let tick = document.create_element("div").unwrap();
        tick.set_text_content(Some(&format!("{}s", i as f64 * 0.5)));
        let tk_el: HtmlElement = tick.clone().dyn_into().unwrap();
        tk_el.style().set_css_text(
            "flex: 1; font-size: 7px; color: var(--text-muted); \
             font-family: var(--font-mono); text-align: left; \
             border-left: 1px solid var(--border-subtle); padding-left: 2px;",
        );
        ruler.append_child(&tick).unwrap();
    }
    timeline_area.append_child(&ruler).unwrap();

    // Playhead
    let playhead = document.create_element("div").unwrap();
    let ph_el: HtmlElement = playhead.clone().dyn_into().unwrap();
    ph_el.style().set_css_text(
        "position: absolute; left: 47%; top: 0; bottom: 0; width: 2px; \
         background: var(--accent-cyan); z-index: 10; pointer-events: none;",
    );

    // Tracks
    for (target, prop, kf_count) in TRACKS {
        let track_row = document.create_element("div").unwrap();
        let tr_el: HtmlElement = track_row.clone().dyn_into().unwrap();
        tr_el.style().set_css_text(
            "display: flex; align-items: center; gap: 4px; padding: 3px 0; \
             border-bottom: 1px solid var(--border-subtle);",
        );

        let label = document.create_element("div").unwrap();
        label.set_text_content(Some(&format!("{}.{}", target, prop)));
        let l_el: HtmlElement = label.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 8px; color: var(--text-primary); font-family: var(--font-mono); \
             min-width: 120px; white-space: nowrap;",
        );
        track_row.append_child(&label).unwrap();

        // Track lane
        let lane = document.create_element("div").unwrap();
        let ln_el: HtmlElement = lane.clone().dyn_into().unwrap();
        ln_el.style().set_css_text(
            "flex: 1; height: 20px; background: var(--surface-panel); \
             border-radius: 3px; position: relative;",
        );

        // Place keyframe dots
        for (kf_target, time, _, _) in KEYFRAMES {
            if *kf_target != *target {
                continue;
            }
            let dot = document.create_element("div").unwrap();
            let d_el: HtmlElement = dot.clone().dyn_into().unwrap();
            let pct = (time / 5.0) * 100.0;
            d_el.style().set_css_text(&format!(
                "position: absolute; left: {}%; top: 50%; transform: translate(-50%, -50%); \
                 width: 6px; height: 6px; border-radius: 50%; \
                 background: var(--accent-cyan); border: 1px solid var(--text-primary);",
                pct,
            ));
            lane.append_child(&dot).unwrap();
        }

        track_row.append_child(&lane).unwrap();

        let count = document.create_element("span").unwrap();
        count.set_text_content(Some(kf_count));
        let c_el: HtmlElement = count.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 60px; text-align: right;",
        );
        track_row.append_child(&count).unwrap();

        timeline_area.append_child(&track_row).unwrap();
    }

    timeline_area.append_child(&playhead).unwrap();
    wrapper.append_child(&timeline_area).unwrap();

    // Clips section
    let clips_header = document.create_element("div").unwrap();
    clips_header.set_text_content(Some("Clips"));
    let ch_el: HtmlElement = clips_header.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "font-size: 9px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); padding: 4px 8px 2px; \
         border-top: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&clips_header).unwrap();

    let clips_area = document.create_element("div").unwrap();
    let ca_el: HtmlElement = clips_area.clone().dyn_into().unwrap();
    ca_el
        .style()
        .set_css_text("padding: 0 8px 4px; max-height: 100px; overflow-y: auto;");

    let table = make_table(document, &["Clip", "Target", "Range", "Loop Mode"]);
    let tbody = document.create_element("tbody").unwrap();
    for (name, target, range, loop_mode) in CLIPS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, target, range, loop_mode].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "Loop" => "rgba(0, 200, 255, 0.8)",
                    "Once" => "var(--text-muted)",
                    "PingPong" => "rgba(200, 150, 255, 0.8)",
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
    clips_area.append_child(&table).unwrap();
    wrapper.append_child(&clips_area).unwrap();

    // Toolbar
    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-top: 1px solid var(--border-subtle);",
    );
    for label in &[
        "+ Keyframe",
        "+ Clip",
        "+ Track",
        "Export .10d",
        "Export glTF",
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

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} timeline requires ANI-1..ANI-6 engine + ICP commands.",
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
