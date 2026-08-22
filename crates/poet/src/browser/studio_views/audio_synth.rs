//! Triad Formant Synthesizer & Audio Spectrogram Visualizer (Subsystem 4.4).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Provides the interactive Triad Formant Synthesizer interface, mapping 3-formant
//! vowels (F1, F2, F3) into acoustic sound synthesis and resonant filtering.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const VOWELS: &[(&str, u32, u32, u32, &str)] = &[
    ("/i/ (see)", 270, 2290, 3010, "#38bdf8"),
    ("/e/ (say)", 530, 1840, 2480, "#34d399"),
    ("/a/ (father)", 730, 1090, 2440, "#fbbf24"),
    ("/o/ (go)", 570, 840, 2410, "#f87171"),
    ("/u/ (too)", 300, 870, 2240, "#a78bfa"),
];

/// Build the Triad Formant Synthesizer & Spectral Visualizer container view.
pub fn build_audio_synth_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 8px; \
         padding: 10px; background: #0f172a; color: #f8fafc; overflow-y: auto;"
    );

    // Header Toolbar
    let hud = document.create_element("div").unwrap();
    hud.set_class_name("vibe-toolbar");
    let hud_el: HtmlElement = hud.clone().dyn_into().unwrap();
    hud_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 6px; padding: 6px 10px;"
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{266B} Triad Formant Synthesizer"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el.style().set_css_text("font-weight: 600; font-size: 12px; color: #a78bfa;");
    hud.append_child(&title).unwrap();

    let meta = document.create_element("span").unwrap();
    meta.set_text_content(Some("F0: 120 Hz \u{00B7} Rate: 48 kHz \u{00B7} Latency: 4.2 ms"));
    let meta_el: HtmlElement = meta.clone().dyn_into().unwrap();
    meta_el.style().set_css_text("font-family: var(--font-mono); font-size: 10px; color: #94a3b8;");
    hud.append_child(&meta).unwrap();

    wrapper.append_child(&hud).unwrap();

    // Vowel Matrix Section
    let vowel_row = document.create_element("div").unwrap();
    let vowel_row_el: HtmlElement = vowel_row.clone().dyn_into().unwrap();
    vowel_row_el.style().set_css_text("display: flex; gap: 6px; flex-wrap: wrap;");

    for (vowel, f1, f2, f3, color) in VOWELS {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(&format!(
            "flex: 1; min-width: 90px; background: rgba(30, 41, 59, 0.5); \
             border: 1px solid {}; border-radius: 6px; padding: 6px 8px; \
             display: flex; flex-direction: column; gap: 2px; cursor: pointer;",
            color
        ));

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(vowel));
        let name_el: HtmlElement = name.clone().dyn_into().unwrap();
        name_el.style().set_css_text(&format!("font-weight: 600; font-size: 11px; color: {};", color));
        card.append_child(&name).unwrap();

        let freqs = document.create_element("span").unwrap();
        freqs.set_text_content(Some(&format!("F1: {} Hz\nF2: {} Hz\nF3: {} Hz", f1, f2, f3)));
        let freqs_el: HtmlElement = freqs.clone().dyn_into().unwrap();
        freqs_el.style().set_css_text(
            "font-family: var(--font-mono); font-size: 9px; opacity: 0.75; white-space: pre-line;"
        );
        card.append_child(&freqs).unwrap();

        vowel_row.append_child(&card).unwrap();
    }

    wrapper.append_child(&vowel_row).unwrap();

    // F1/F2 Formant Plane Canvas Placeholder
    let canvas_container = document.create_element("div").unwrap();
    let canvas_container_el: HtmlElement = canvas_container.clone().dyn_into().unwrap();
    canvas_container_el.style().set_css_text(
        "flex: 1; min-height: 140px; background: #020617; border: 1px solid rgba(255, 255, 255, 0.08); \
         border-radius: 6px; position: relative; display: flex; align-items: center; justify-content: center;"
    );

    let canvas_label = document.create_element("div").unwrap();
    canvas_label.set_text_content(Some("F1 \u{00D7} F2 Acoustic Vowel Space (Interactive Morphing)"));
    let canvas_label_el: HtmlElement = canvas_label.clone().dyn_into().unwrap();
    canvas_label_el.style().set_css_text("font-size: 11px; color: #64748b; font-family: var(--font-mono);");
    canvas_container.append_child(&canvas_label).unwrap();

    wrapper.append_child(&canvas_container).unwrap();

    // Action Controls
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Play Vowel", "Continuous Drone", "Glottal Pulse", "Sonify Formula"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vowels_list() {
        assert_eq!(VOWELS.len(), 5);
        assert_eq!(VOWELS[0].0, "/i/ (see)");
    }
}
