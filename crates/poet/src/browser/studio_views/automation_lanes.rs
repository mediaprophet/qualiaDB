//! Automation Lanes — per-channel parameter automation (§5.1, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const LANES: &[(&str, &str, usize, &str)] = &[
    ("Vocal", "Gain", 6, "Linear"),
    ("Vocal", "Pan", 4, "Linear"),
    ("Vocal", "EQ HI-MID Freq", 3, "Step"),
    ("Vocal", "Comp Threshold", 5, "Hermite"),
    ("Vocal", "Reverb Send", 4, "Linear"),
    ("Bass", "Gain", 3, "Linear"),
    ("Bass", "Comp Ratio", 2, "Step"),
    ("Guitar L", "Pan", 5, "Linear"),
    ("Guitar R", "Pan", 5, "Linear"),
    ("Master", "Gain", 2, "Linear"),
];

pub fn build_automation_lanes_view(document: &Document) -> Element {
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
    for label in &["+ Lane", "Read", "Write", "Touch", "Latch", "Clear All"] {
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
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px 8px;");

    // Time ruler
    let ruler = document.create_element("div").unwrap();
    let r_el: HtmlElement = ruler.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "display: flex; height: 14px; border-bottom: 1px solid var(--border-subtle); \
         margin-bottom: 4px;",
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
    content.append_child(&ruler).unwrap();

    // Automation lanes
    for (channel, param, kf_count, interp) in LANES {
        let lane_row = document.create_element("div").unwrap();
        let lr_el: HtmlElement = lane_row.clone().dyn_into().unwrap();
        lr_el.style().set_css_text(
            "display: flex; align-items: center; gap: 4px; padding: 2px 0; \
             border-bottom: 1px solid var(--border-subtle);",
        );

        // Label
        let label = document.create_element("div").unwrap();
        label.set_text_content(Some(&format!("{}.{}", channel, param)));
        let l_el: HtmlElement = label.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 8px; color: var(--text-primary); font-family: var(--font-mono); \
             min-width: 140px; white-space: nowrap;",
        );
        lane_row.append_child(&label).unwrap();

        // Lane
        let lane = document.create_element("div").unwrap();
        let ln_el: HtmlElement = lane.clone().dyn_into().unwrap();
        ln_el.style().set_css_text(
            "flex: 1; height: 18px; background: var(--surface-panel); \
             border-radius: 3px; position: relative;",
        );

        // Mock automation curve (dots at random-ish positions)
        for i in 0..*kf_count {
            let dot = document.create_element("div").unwrap();
            let d_el: HtmlElement = dot.clone().dyn_into().unwrap();
            let pct = (i as f64 / *kf_count.max(&1) as f64) * 90.0 + 5.0;
            let y_pos = 30.0 + ((i * 17) % 60) as f64;
            d_el.style().set_css_text(&format!(
                "position: absolute; left: {}%; top: {}%; \
                 width: 5px; height: 5px; border-radius: 50%; \
                 background: var(--accent-cyan); border: 1px solid var(--text-primary);",
                pct, y_pos,
            ));
            lane.append_child(&dot).unwrap();
        }

        // Interpolation mode badge
        let interp_badge = document.create_element("span").unwrap();
        interp_badge.set_text_content(Some(interp));
        let ib_el: HtmlElement = interp_badge.clone().dyn_into().unwrap();
        let interp_color = match *interp {
            "Linear" => "rgba(0, 200, 255, 0.6)",
            "Step" => "rgba(100, 200, 100, 0.6)",
            "Hermite" => "rgba(200, 150, 255, 0.6)",
            _ => "var(--text-muted)",
        };
        ib_el.style().set_css_text(&format!(
            "position: absolute; right: 4px; top: 50%; transform: translateY(-50%); \
             font-size: 6px; color: {}; font-family: var(--font-mono);",
            interp_color,
        ));
        lane.append_child(&interp_badge).unwrap();

        lane_row.append_child(&lane).unwrap();

        // Keyframe count
        let count = document.create_element("span").unwrap();
        count.set_text_content(Some(&format!("{} kf", kf_count)));
        let c_el2: HtmlElement = count.clone().dyn_into().unwrap();
        c_el2.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 40px; text-align: right;",
        );
        lane_row.append_child(&count).unwrap();

        content.append_child(&lane_row).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} automation lanes require AUD-11..AUD-12 engine.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
