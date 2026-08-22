//! Meter Bridge — per-channel + per-bus peak/RMS/LUFS/true-peak meters (§5.1, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const METERS: &[(&str, f64, f64, f64, f64, &str)] = &[
    ("Kick", -3.0, -6.0, -9.0, -2.5, "on"),
    ("Snare", -6.0, -10.0, -12.0, -5.0, "on"),
    ("Bass", -4.0, -7.0, -10.0, -3.0, "on"),
    ("Guitar L", -8.0, -12.0, -14.0, -7.0, "on"),
    ("Guitar R", -8.0, -12.0, -14.0, -7.0, "on"),
    ("Vocal", -2.0, -5.0, -8.0, -1.0, "solo"),
    ("Backing", -60.0, -60.0, -60.0, -60.0, "mute"),
    ("Reverb Bus", -10.0, -14.0, -16.0, -9.0, "on"),
    ("Master", -1.5, -4.0, -7.0, -0.5, "on"),
];

pub fn build_meter_bridge_view(document: &Document) -> Element {
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
        "Peak Hold",
        "LUFS Integrated",
        "True Peak",
        "Phase Correlation",
        "K-Meter",
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

    // Meter table
    let table = make_table(
        document,
        &["Channel", "Peak", "RMS", "LUFS", "True Peak", "Meter"],
    );
    let tbody = document.create_element("tbody").unwrap();

    for (name, peak, rms, lufs, true_peak, state) in METERS {
        let tr = document.create_element("tr").unwrap();

        // Channel name
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(name));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        let state_color = if *state == "mute" {
            "var(--text-muted)"
        } else if *state == "solo" {
            "rgba(255, 165, 0, 0.8)"
        } else {
            "var(--text-primary)"
        };
        td_el.style().set_css_text(&format!(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: {}; font-size: 9px; font-weight: 600; font-family: var(--font-mono);",
            state_color,
        ));
        tr.append_child(&td).unwrap();

        // Numeric values
        for val in &[*peak, *rms, *lufs, *true_peak] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(&format!("{:+.1}", val)));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            let val_color = if *val > 0.0 {
                "rgba(255, 0, 0, 0.8)"
            } else if *val > -6.0 {
                "rgba(255, 165, 0, 0.8)"
            } else if *val > -60.0 {
                "rgba(100, 200, 100, 0.8)"
            } else {
                "var(--text-muted)"
            };
            td_el.style().set_css_text(&format!(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                 color: {}; font-size: 9px; font-family: var(--font-mono); font-weight: 600;",
                val_color,
            ));
            tr.append_child(&td).unwrap();
        }

        // Visual meter bar
        let td = document.create_element("td").unwrap();
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); min-width: 100px;",
        );

        let bar_bg = document.create_element("div").unwrap();
        let bb_el: HtmlElement = bar_bg.clone().dyn_into().unwrap();
        bb_el.style().set_css_text(
            "height: 10px; background: var(--surface-bg); border-radius: 2px; \
             position: relative; overflow: hidden;",
        );

        // Scale: -60 dB to +6 dB
        let pct = ((peak + 60.0) / 66.0 * 100.0).max(0.0).min(100.0);
        let bar_fill = document.create_element("div").unwrap();
        let bf_el: HtmlElement = bar_fill.clone().dyn_into().unwrap();
        let bar_color = if *peak > 0.0 {
            "linear-gradient(to right, rgba(100,200,100,0.6), rgba(255,165,0,0.6), rgba(255,0,0,0.8))"
        } else if *peak > -6.0 {
            "linear-gradient(to right, rgba(100,200,100,0.6), rgba(255,165,0,0.6))"
        } else {
            "rgba(100, 200, 100, 0.5)"
        };
        bf_el.style().set_css_text(&format!(
            "position: absolute; left: 0; top: 0; bottom: 0; width: {}%; \
             background: {}; border-radius: 2px;",
            pct, bar_color,
        ));
        bar_bg.append_child(&bar_fill).unwrap();

        // 0 dB reference line
        let zero_line = document.create_element("div").unwrap();
        let zl_el: HtmlElement = zero_line.clone().dyn_into().unwrap();
        zl_el.style().set_css_text(
            "position: absolute; left: 90.9%; top: 0; bottom: 0; width: 1px; \
             background: rgba(255, 255, 255, 0.3);",
        );
        bar_bg.append_child(&zero_line).unwrap();

        td.append_child(&bar_bg).unwrap();
        tr.append_child(&td).unwrap();

        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();

    // Spectrum analyzer placeholder
    let spec_header = document.create_element("div").unwrap();
    spec_header.set_text_content(Some("Spectrum Analyzer"));
    let sh_el: HtmlElement = spec_header.clone().dyn_into().unwrap();
    sh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&spec_header).unwrap();

    let spec_area = document.create_element("div").unwrap();
    let sa_el: HtmlElement = spec_area.clone().dyn_into().unwrap();
    sa_el.style().set_css_text(
        "height: 60px; background: var(--surface-panel); border-radius: 4px; \
         display: flex; align-items: flex-end; gap: 1px; padding: 4px; \
         border: 1px solid var(--border-subtle);",
    );

    // Mock spectrum bars
    for i in 0..48 {
        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        let height = 20.0 + ((i * 13 + 7) % 70) as f64 * 0.6;
        let color = if i < 8 {
            "rgba(100, 200, 100, 0.5)"
        } else if i < 24 {
            "rgba(0, 200, 255, 0.4)"
        } else if i < 36 {
            "rgba(255, 165, 0, 0.4)"
        } else {
            "rgba(255, 100, 100, 0.3)"
        };
        b_el.style().set_css_text(&format!(
            "flex: 1; height: {}%; background: {}; border-radius: 1px;",
            height, color,
        ));
        sa_el.append_child(&bar).unwrap();
    }
    content.append_child(&spec_area).unwrap();

    // Phase correlation
    let phase_header = document.create_element("div").unwrap();
    phase_header.set_text_content(Some("Phase Correlation: +0.82 (good)"));
    let fh_el: HtmlElement = phase_header.clone().dyn_into().unwrap();
    fh_el.style().set_css_text(
        "font-size: 9px; color: rgba(100, 200, 100, 0.8); font-family: var(--font-mono); \
         margin-top: 6px;",
    );
    content.append_child(&phase_header).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} meter bridge requires AUD-13..AUD-15 engine.",
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
            "text-align: left; padding: 4px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
