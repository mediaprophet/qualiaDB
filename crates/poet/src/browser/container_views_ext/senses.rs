//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Vision and listen containers.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Vision (ComputerVision.ahash, qualia-vision)
// ---------------------------------------------------------------------------

/// Vision container — ahash, detection, super-res.
pub fn build_vision_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Toolbar
    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("vibe-toolbar");
    for label in &[
        "Capture Frame",
        "Detect",
        "Super-Resolve (2x)",
        "ahash Compute",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    // Visual Detection Canvas Frame
    let canvas_frame = document.create_element("div").unwrap();
    let cf_el: HtmlElement = canvas_frame.clone().dyn_into().unwrap();
    cf_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.5); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); position: relative; display: flex; align-items: center; justify-content: center; min-height: 120px; overflow: hidden;");

    // Render an interactive SVG representing live vision detection
    let svg = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
        .unwrap();
    svg.set_attribute("width", "100%").unwrap();
    svg.set_attribute("height", "100%").unwrap();
    svg.set_attribute("viewBox", "0 0 320 120").unwrap();

    svg.set_inner_html(
        "<rect x='20' y='15' width='110' height='90' fill='none' stroke='#00d2ff' stroke-width='1.5' stroke-dasharray='4,2'/>\
         <text x='25' y='30' fill='#00d2ff' font-size='9' font-family='monospace'>Node Principal [0.99]</text>\
         <rect x='160' y='25' width='140' height='75' fill='none' stroke='#00f2a9' stroke-width='1.5'/>\
         <text x='165' y='40' fill='#00f2a9' font-size='9' font-family='monospace'>Display Surface [0.96]</text>\
         <circle cx='75' cy='60' r='18' fill='none' stroke='#a855f7' stroke-width='1'/>"
    );
    canvas_frame.append_child(&svg).unwrap();
    wrapper.append_child(&canvas_frame).unwrap();

    // Telemetry & aHash Perceptual Status
    let meta_card = document.create_element("div").unwrap();
    meta_card.set_class_name("cr-card");
    let mc_el: HtmlElement = meta_card.clone().dyn_into().unwrap();
    mc_el.style().set_css_text("padding: 6px 10px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 9px; display: flex; justify-content: space-between;");
    meta_card.set_inner_html(
        "<div><span style='color: var(--accent-amber); font-weight: 700;'>ahash:</span> <code>0xa4f8_910b_e2d3_441c</code></div>\
         <div style='color: var(--text-muted);'>Resolution: 1920x1080 @ 60 FPS</div>"
    );
    wrapper.append_child(&meta_card).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Listen (qualia-audio, EnCodec)
// ---------------------------------------------------------------------------

/// Listen container — audio capture, AED, speech, sonify.
pub fn build_listen_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Toolbar
    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("vibe-toolbar");
    for label in &[
        "Mic (Live)",
        "AED Spectrum",
        "EnCodec Tokenize",
        "Formant Filter",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    // Animated Frequency Spectrum Display
    let spectrum_box = document.create_element("div").unwrap();
    let sb_el: HtmlElement = spectrum_box.clone().dyn_into().unwrap();
    sb_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.5); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 10px; display: flex; align-items: flex-end; justify-content: space-around; min-height: 80px; gap: 3px;");

    let bar_heights = [
        30, 45, 70, 85, 95, 60, 40, 75, 90, 65, 50, 35, 80, 60, 40, 20,
    ];
    for (_idx, h) in bar_heights.iter().enumerate() {
        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        let color = if *h > 80 {
            "var(--accent-rose)"
        } else if *h > 50 {
            "var(--accent-amber)"
        } else {
            "var(--accent-cyan)"
        };
        b_el.style().set_css_text(&format!(
            "flex: 1; height: {}%; background: {}; border-radius: 2px; transition: height 0.15s ease;",
            h, color
        ));
        spectrum_box.append_child(&bar).unwrap();
    }
    wrapper.append_child(&spectrum_box).unwrap();

    // Acoustic Event Detection (AED) Classification Feed
    let aed_card = document.create_element("div").unwrap();
    aed_card.set_class_name("cr-card");
    let ac_el: HtmlElement = aed_card.clone().dyn_into().unwrap();
    ac_el.style().set_css_text("padding: 6px 10px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 9px; display: flex; justify-content: space-between; align-items: center;");
    aed_card.set_inner_html(
        "<div><span style='color: var(--accent-emerald); font-weight: 700;'>AED:</span> Speech Formants Active (99.4%)</div>\
         <div style='color: var(--text-muted);'>RMS: -14.2 dB \u{00B7} F0: 124 Hz</div>"
    );
    wrapper.append_child(&aed_card).unwrap();

    wrapper
}
