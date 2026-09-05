//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Multi-modal document embedding chips.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Multi-Modal Document Embedding Chips (Subsystem 1.7)
// ---------------------------------------------------------------------------

/// Interactive audio waveform chip with play/pause and spectrogram bars.
pub fn build_multimodal_chip_audio(document: &Document, title: &str, duration: &str) -> Element {
    let chip = document.create_element("span").unwrap();
    chip.set_class_name("cml-chip cml-chip-audio");
    let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
    chip_el.style().set_css_text(
        "display: inline-flex; align-items: center; gap: 6px; padding: 2px 8px; \
         background: rgba(139, 92, 246, 0.12); border: 1px solid rgba(139, 92, 246, 0.35); \
         border-radius: 12px; font-size: 11px; color: #a78bfa; margin: 0 4px; vertical-align: middle; \
         cursor: pointer; user-select: none;"
    );

    let play_icon = document.create_element("span").unwrap();
    play_icon.set_text_content(Some("\u{25B6}")); // ▶
    chip.append_child(&play_icon).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some(title));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el.style().set_css_text("font-weight: 500;");
    chip.append_child(&label).unwrap();

    // Mini SVG waveform indicator
    let waves = document.create_element("span").unwrap();
    waves.set_text_content(Some(
        "\u{2582}\u{2585}\u{2588}\u{2586}\u{2583}\u{2587}\u{2584}",
    ));
    let waves_el: HtmlElement = waves.clone().dyn_into().unwrap();
    waves_el
        .style()
        .set_css_text("font-family: var(--font-mono); letter-spacing: -1px; opacity: 0.8;");
    chip.append_child(&waves).unwrap();

    let dur = document.create_element("span").unwrap();
    dur.set_text_content(Some(duration));
    let dur_el: HtmlElement = dur.clone().dyn_into().unwrap();
    dur_el.style().set_css_text("font-size: 9px; opacity: 0.6;");
    chip.append_child(&dur).unwrap();

    chip
}

/// 3D manifold / wireframe thumbnail chip.
pub fn build_multimodal_chip_3d(document: &Document, name: &str, poly_count: u32) -> Element {
    let chip = document.create_element("span").unwrap();
    chip.set_class_name("cml-chip cml-chip-3d");
    let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
    chip_el.style().set_css_text(
        "display: inline-flex; align-items: center; gap: 5px; padding: 2px 8px; \
         background: rgba(59, 130, 246, 0.12); border: 1px solid rgba(59, 130, 246, 0.35); \
         border-radius: 12px; font-size: 11px; color: #60a5fa; margin: 0 4px; vertical-align: middle; \
         cursor: pointer; user-select: none;"
    );

    let icon = document.create_element("span").unwrap();
    icon.set_text_content(Some("\u{25C7}")); // ◇
    chip.append_child(&icon).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some(name));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el.style().set_css_text("font-weight: 500;");
    chip.append_child(&label).unwrap();

    let count = document.create_element("span").unwrap();
    count.set_text_content(Some(&format!("{}p", poly_count)));
    let count_el: HtmlElement = count.clone().dyn_into().unwrap();
    count_el
        .style()
        .set_css_text("font-size: 9px; opacity: 0.6;");
    chip.append_child(&count).unwrap();

    chip
}

/// Embedded mini-spreadsheet chip with live preview cell.
pub fn build_multimodal_chip_sheet(
    document: &Document,
    sheet_ref: &str,
    summary_val: &str,
) -> Element {
    let chip = document.create_element("span").unwrap();
    chip.set_class_name("cml-chip cml-chip-sheet");
    let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
    chip_el.style().set_css_text(
        "display: inline-flex; align-items: center; gap: 5px; padding: 2px 8px; \
         background: rgba(16, 185, 129, 0.12); border: 1px solid rgba(16, 185, 129, 0.35); \
         border-radius: 12px; font-size: 11px; color: #34d399; margin: 0 4px; vertical-align: middle; \
         cursor: pointer; user-select: none;"
    );

    let icon = document.create_element("span").unwrap();
    icon.set_text_content(Some("\u{229E}")); // ⊞
    chip.append_child(&icon).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some(sheet_ref));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el.style().set_css_text("font-weight: 500;");
    chip.append_child(&label).unwrap();

    let val = document.create_element("span").unwrap();
    val.set_text_content(Some(&format!("= {}", summary_val)));
    let val_el: HtmlElement = val.clone().dyn_into().unwrap();
    val_el
        .style()
        .set_css_text("font-family: var(--font-mono); font-size: 10px; font-weight: 600;");
    chip.append_child(&val).unwrap();

    chip
}

/// Semantic provenance citation badge chip linking to W3C Verifiable Credentials / DIDs.
pub fn build_multimodal_chip_citation(
    document: &Document,
    did_short: &str,
    certainty_pct: u8,
) -> Element {
    let chip = document.create_element("span").unwrap();
    chip.set_class_name("cml-chip cml-chip-citation");
    let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
    chip_el.style().set_css_text(
        "display: inline-flex; align-items: center; gap: 4px; padding: 1px 6px; \
         background: rgba(245, 158, 11, 0.12); border: 1px solid rgba(245, 158, 11, 0.35); \
         border-radius: 8px; font-size: 10px; color: #fbbf24; margin: 0 3px; vertical-align: middle; \
         cursor: pointer; user-select: none;"
    );

    let icon = document.create_element("span").unwrap();
    icon.set_text_content(Some("\u{2713}")); // ✓
    chip.append_child(&icon).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some(did_short));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el
        .style()
        .set_css_text("font-family: var(--font-mono);");
    chip.append_child(&label).unwrap();

    let cert = document.create_element("span").unwrap();
    cert.set_text_content(Some(&format!("{}%", certainty_pct)));
    let cert_el: HtmlElement = cert.clone().dyn_into().unwrap();
    cert_el
        .style()
        .set_css_text("font-size: 9px; opacity: 0.75;");
    chip.append_child(&cert).unwrap();

    chip
}
