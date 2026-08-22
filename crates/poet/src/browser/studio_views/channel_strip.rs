//! Channel Strip — EQ + comp + gate + delay/reverb + sends (§5.1, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const EQ_BANDS: &[(&str, f64, f64, f64, bool)] = &[
    ("LO", 80.0, 2.5, 0.7, true),
    ("LO-MID", 350.0, 0.0, 1.0, true),
    ("HI-MID", 2500.0, -1.5, 1.2, true),
    ("HI", 8000.0, 1.0, 0.8, false),
];

const COMP_PARAMS: &[(&str, f64, &str)] = &[
    ("Threshold", -20.0, "dB"),
    ("Ratio", 4.0, ":1"),
    ("Attack", 10.0, "ms"),
    ("Release", 100.0, "ms"),
    ("Knee", 3.0, ""),
];

const SENDS: &[(&str, f64, &str)] = &[
    ("Reverb Bus", 0.35, "post"),
    ("Delay Bus", 0.15, "post"),
    ("Comp Bus", 0.0, "pre"),
];

pub fn build_channel_strip_view(document: &Document) -> Element {
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
        "< Prev Channel",
        "Next Channel >",
        "Reset Channel",
        "Bypass All",
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

    // Channel header
    let hdr = document.create_element("div").unwrap();
    let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "padding: 6px 8px; background: var(--surface-panel); border-radius: 4px; \
         margin-bottom: 6px; display: flex; align-items: center; gap: 6px;",
    );

    let ch_name = document.create_element("div").unwrap();
    ch_name.set_text_content(Some("Channel: Vocal  |  Source: track  |  Colour: #b197fc"));
    let cn_el: HtmlElement = ch_name.clone().dyn_into().unwrap();
    cn_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    h_el.append_child(&ch_name).unwrap();
    content.append_child(&hdr).unwrap();

    // EQ Section
    content
        .append_child(&build_section_header(
            document,
            "EQ \u{2014} 4-Band Parametric",
        ))
        .unwrap();
    let eq_table = make_table(document, &["Band", "Freq (Hz)", "Gain (dB)", "Q", "On"]);
    let eq_tbody = document.create_element("tbody").unwrap();
    for (name, freq, gain, q, on) in EQ_BANDS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            format!("{:.0}", freq),
            format!("{:+.1}", gain),
            format!("{:.1}", q),
            if *on { "On" } else { "Off" }.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = if *on {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600;",
                    color,
                ));
            } else if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 9px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        eq_tbody.append_child(&tr).unwrap();
    }
    eq_table.append_child(&eq_tbody).unwrap();
    content.append_child(&eq_table).unwrap();

    // Compressor Section
    content
        .append_child(&build_section_header(document, "Compressor"))
        .unwrap();
    let comp_table = make_table(document, &["Param", "Value", "Unit"]);
    let comp_tbody = document.create_element("tbody").unwrap();
    for (name, value, unit) in COMP_PARAMS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![name.to_string(), format!("{:.1}", value), unit.to_string()];
        for (i, val) in vals.iter().enumerate() {
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
        comp_tbody.append_child(&tr).unwrap();
    }
    comp_table.append_child(&comp_tbody).unwrap();
    content.append_child(&comp_table).unwrap();

    // Gate/Expander
    content
        .append_child(&build_section_header(document, "Gate / Expander"))
        .unwrap();
    let gate_info = document.create_element("div").unwrap();
    gate_info.set_text_content(Some(
        "Threshold: -45 dB  |  Range: -30 dB  |  Attack: 5 ms  |  Release: 80 ms  |  Sidechain: Off",
    ));
    let gi_el: HtmlElement = gate_info.clone().dyn_into().unwrap();
    gi_el.style().set_css_text(
        "padding: 4px 8px; font-size: 9px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    content.append_child(&gate_info).unwrap();

    // Sends
    content
        .append_child(&build_section_header(document, "Sends"))
        .unwrap();
    let sends_table = make_table(document, &["Bus", "Level", "Tap"]);
    let sends_tbody = document.create_element("tbody").unwrap();
    for (bus, level, tap) in SENDS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![bus.to_string(), format!("{:.2}", level), tap.to_string()];
        for (i, val) in vals.iter().enumerate() {
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
        sends_tbody.append_child(&tr).unwrap();
    }
    sends_table.append_child(&sends_tbody).unwrap();
    content.append_child(&sends_table).unwrap();

    // Output
    content
        .append_child(&build_section_header(document, "Output"))
        .unwrap();
    let out_info = document.create_element("div").unwrap();
    out_info.set_text_content(Some(
        "Bus: Master  |  Gain: -2.0 dB  |  Pan: C  |  Mute: Off  |  Solo: On  |  Direct Out: Off",
    ));
    let oi_el: HtmlElement = out_info.clone().dyn_into().unwrap();
    oi_el.style().set_css_text(
        "padding: 4px 8px; font-size: 9px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    content.append_child(&out_info).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} channel strip requires AUD-6..AUD-10 engine plugins.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_section_header(document: &Document, title: &str) -> Element {
    let hdr = document.create_element("div").unwrap();
    hdr.set_text_content(Some(title));
    let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 8px; margin-bottom: 4px; \
         padding-bottom: 2px; border-bottom: 1px solid var(--border-subtle);",
    );
    hdr
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
