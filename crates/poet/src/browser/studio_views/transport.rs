//! Transport — play/pause/stop/record/scrub/time-display (§5.1, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

pub fn build_transport_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    // Main transport controls
    let controls = document.create_element("div").unwrap();
    let c_el: HtmlElement = controls.clone().dyn_into().unwrap();
    c_el.style().set_css_text(
        "display: flex; align-items: center; gap: 8px; padding: 8px; \
         background: var(--surface-panel); border-radius: 6px; margin: 4px 8px;",
    );

    for (label, color) in &[
        ("\u{23F9} Stop", "var(--text-secondary)"),
        ("\u{23F5} Play", "rgba(100, 200, 100, 0.8)"),
        ("\u{23F8} Pause", "var(--text-secondary)"),
        ("\u{23CF} Record", "rgba(255, 0, 0, 0.8)"),
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(&format!(
            "padding: 4px 12px; border: 1px solid var(--border-medium); \
             background: transparent; color: {}; border-radius: 4px; \
             cursor: pointer; font-size: 12px; font-family: var(--font-mono); font-weight: 600;",
            color,
        ));
        controls.append_child(&btn).unwrap();
    }

    // Time display
    let time_div = document.create_element("div").unwrap();
    let t_el: HtmlElement = time_div.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "margin-left: auto; display: flex; flex-direction: column; align-items: flex-end;",
    );

    let time_main = document.create_element("div").unwrap();
    time_main.set_text_content(Some("00:02:15.340"));
    let tm_el: HtmlElement = time_main.clone().dyn_into().unwrap();
    tm_el.style().set_css_text(
        "font-size: 18px; font-weight: 700; color: var(--accent-cyan); \
         font-family: var(--font-mono);",
    );
    t_el.append_child(&time_main).unwrap();

    let time_sub = document.create_element("div").unwrap();
    time_sub.set_text_content(Some("BAR 3.2.1 | 120 BPM | 4/4"));
    let ts_el: HtmlElement = time_sub.clone().dyn_into().unwrap();
    ts_el
        .style()
        .set_css_text("font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);");
    t_el.append_child(&time_sub).unwrap();
    controls.append_child(&time_div).unwrap();

    wrapper.append_child(&controls).unwrap();

    // Scrub bar
    let scrub_area = document.create_element("div").unwrap();
    let sc_el: HtmlElement = scrub_area.clone().dyn_into().unwrap();
    sc_el.style().set_css_text("padding: 0 8px;");

    let scrub_label = document.create_element("div").unwrap();
    scrub_label.set_text_content(Some("Scrub"));
    let sl_el: HtmlElement = scrub_label.clone().dyn_into().unwrap();
    sl_el.style().set_css_text(
        "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
         margin-bottom: 2px;",
    );
    scrub_area.append_child(&scrub_label).unwrap();

    let scrub_bar = document.create_element("div").unwrap();
    let sb_el: HtmlElement = scrub_bar.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "height: 24px; background: var(--surface-panel); border-radius: 4px; \
         position: relative; border: 1px solid var(--border-subtle);",
    );

    // Playhead position (45%)
    let playhead = document.create_element("div").unwrap();
    let ph_el: HtmlElement = playhead.clone().dyn_into().unwrap();
    ph_el.style().set_css_text(
        "position: absolute; left: 45%; top: 0; bottom: 0; width: 3px; \
         background: var(--accent-cyan); border-radius: 1px; cursor: pointer;",
    );
    scrub_bar.append_child(&playhead).unwrap();

    // Waveform mock (bars)
    for i in 0..40 {
        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        let height = 30.0 + ((i * 7) % 60) as f64 * 0.5;
        b_el.style().set_css_text(&format!(
            "position: absolute; left: {}%; top: 50%; transform: translateY(-50%); \
             width: 1.5%; height: {}%; background: rgba(0, 200, 255, 0.2);",
            i as f64 * 2.5,
            height,
        ));
        scrub_bar.append_child(&bar).unwrap();
    }

    scrub_area.append_child(&scrub_bar).unwrap();
    wrapper.append_child(&scrub_area).unwrap();

    // Info grid
    let info = document.create_element("div").unwrap();
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; \
         padding: 4px 8px;",
    );

    for (label, value) in &[
        ("Sample Rate", "48 kHz"),
        ("Bit Depth", "24-bit"),
        ("Buffer", "256 samples"),
        ("Output", "Stereo"),
    ] {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "text-align: center; padding: 4px; background: var(--surface-panel); \
             border-radius: 4px;",
        );
        let v = document.create_element("div").unwrap();
        v.set_text_content(Some(value));
        let v_el: HtmlElement = v.clone().dyn_into().unwrap();
        v_el.style().set_css_text(
            "font-size: 12px; font-weight: 700; color: var(--text-primary); \
             font-family: var(--font-mono);",
        );
        card.append_child(&v).unwrap();
        let l = document.create_element("div").unwrap();
        l.set_text_content(Some(label));
        let l_el: HtmlElement = l.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&l).unwrap();
        i_el.append_child(&card).unwrap();
    }
    wrapper.append_child(&info).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} transport requires AUD-5 engine + capture.rs.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
