//! Desk Surface — audio mixing console channel strips + master bus (§5.2, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const CHANNELS: &[(&str, &str, &str, f64, &str)] = &[
    ("Kick", "track", "#ff6b6b", 0.0, "on"),
    ("Snare", "track", "#ffa94d", 0.0, "on"),
    ("Bass", "track", "#69db7c", -3.0, "on"),
    ("Guitar L", "track", "#4dabf7", -6.0, "on"),
    ("Guitar R", "track", "#4dabf7", -6.0, "on"),
    ("Vocal", "track", "#b197fc", -2.0, "solo"),
    ("Backing", "track", "#63e6be", -12.0, "mute"),
    ("Reverb Bus", "bus", "#ffa94d", -8.0, "on"),
    ("Master", "master", "#ffffff", 0.0, "on"),
];

pub fn build_desk_surface_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    // Toolbar
    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle);",
    );
    for label in &[
        "+ Channel",
        "+ Bus",
        "Save Patch",
        "Load Patch",
        "Bind Manifold",
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

    // Channel strips area (horizontal scroll)
    let strips_area = document.create_element("div").unwrap();
    let sa_el: HtmlElement = strips_area.clone().dyn_into().unwrap();
    sa_el.style().set_css_text(
        "flex: 1; overflow-x: auto; overflow-y: hidden; padding: 4px 8px; \
         display: flex; gap: 4px;",
    );

    for (name, source, color, gain_db, state) in CHANNELS {
        let strip = build_channel_strip(document, name, source, color, *gain_db, state);
        strips_area.append_child(&strip).unwrap();
    }

    wrapper.append_child(&strips_area).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} desk surface requires AUD-1..AUD-5 engine + AudioWorklet.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_channel_strip(
    document: &Document,
    name: &str,
    source: &str,
    color: &str,
    gain_db: f64,
    state: &str,
) -> Element {
    let strip = document.create_element("div").unwrap();
    let s_el: HtmlElement = strip.clone().dyn_into().unwrap();
    let width = if source == "master" { "80px" } else { "64px" };
    s_el.style().set_css_text(&format!(
        "display: flex; flex-direction: column; gap: 2px; padding: 4px; \
         background: var(--surface-panel); border-radius: 4px; \
         border: 1px solid var(--border-subtle); min-width: {}; \
         border-top: 3px solid {};",
        width, color,
    ));

    // Header
    let hdr = document.create_element("div").unwrap();
    hdr.set_text_content(Some(name));
    let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
    h_el.style().set_css_text(&format!(
        "font-size: 8px; font-weight: 700; color: {}; font-family: var(--font-mono); \
         text-align: center; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
        color,
    ));
    strip.append_child(&hdr).unwrap();

    let src_label = document.create_element("div").unwrap();
    src_label.set_text_content(Some(source));
    let sl_el: HtmlElement = src_label.clone().dyn_into().unwrap();
    sl_el.style().set_css_text(
        "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
         text-align: center; text-transform: uppercase;",
    );
    strip.append_child(&src_label).unwrap();

    // EQ section (4 bands, compact)
    let eq_label = document.create_element("div").unwrap();
    eq_label.set_text_content(Some("EQ"));
    let eql_el: HtmlElement = eq_label.clone().dyn_into().unwrap();
    eql_el.style().set_css_text(
        "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
         text-align: center; margin-top: 2px;",
    );
    strip.append_child(&eq_label).unwrap();

    for band in &["LO", "LO-MID", "HI-MID", "HI"] {
        let band_div = document.create_element("div").unwrap();
        band_div.set_text_content(Some(band));
        let bd_el: HtmlElement = band_div.clone().dyn_into().unwrap();
        bd_el.style().set_css_text(
            "font-size: 6px; color: var(--text-muted); font-family: var(--font-mono); \
             text-align: center; padding: 1px 0;",
        );
        strip.append_child(&band_div).unwrap();
    }

    // Dynamics (comp)
    let comp = document.create_element("div").unwrap();
    comp.set_text_content(Some("COMP"));
    let cp_el: HtmlElement = comp.clone().dyn_into().unwrap();
    cp_el.style().set_css_text(
        "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
         text-align: center; margin-top: 2px; border-top: 1px solid var(--border-subtle); \
         padding-top: 2px;",
    );
    strip.append_child(&comp).unwrap();

    // Fader (vertical bar)
    let fader_area = document.create_element("div").unwrap();
    let fa_el: HtmlElement = fader_area.clone().dyn_into().unwrap();
    fa_el.style().set_css_text(
        "height: 60px; background: var(--surface-bg); border-radius: 3px; \
         position: relative; margin-top: 4px; border: 1px solid var(--border-subtle);",
    );

    // Fader fill (0dB = 80%, -inf = 0%)
    let gain_pct = ((gain_db + 60.0) / 66.0 * 100.0).max(0.0).min(100.0);
    let fader_fill = document.create_element("div").unwrap();
    let ff_el: HtmlElement = fader_fill.clone().dyn_into().unwrap();
    ff_el.style().set_css_text(&format!(
        "position: absolute; bottom: 0; left: 0; right: 0; height: {}%; \
         background: linear-gradient(to top, {}, rgba(0,0,0,0)); border-radius: 2px;",
        gain_pct, color,
    ));
    fa_el.append_child(&fader_fill).unwrap();

    // 0dB line
    let zero_line = document.create_element("div").unwrap();
    let zl_el: HtmlElement = zero_line.clone().dyn_into().unwrap();
    zl_el.style().set_css_text(
        "position: absolute; bottom: 80%; left: 0; right: 0; height: 1px; \
         background: rgba(100, 200, 100, 0.4);",
    );
    fa_el.append_child(&zero_line).unwrap();

    strip.append_child(&fader_area).unwrap();

    // Gain readout
    let gain_text = document.create_element("div").unwrap();
    gain_text.set_text_content(Some(&format!("{:+.0} dB", gain_db)));
    let gt_el: HtmlElement = gain_text.clone().dyn_into().unwrap();
    gt_el.style().set_css_text(
        "font-size: 7px; color: var(--text-primary); font-family: var(--font-mono); \
         text-align: center; margin-top: 2px;",
    );
    strip.append_child(&gain_text).unwrap();

    // Mute / Solo buttons
    let btn_row = document.create_element("div").unwrap();
    let br_el: HtmlElement = btn_row.clone().dyn_into().unwrap();
    br_el
        .style()
        .set_css_text("display: flex; gap: 2px; margin-top: 2px;");

    let mute_btn = document.create_element("button").unwrap();
    mute_btn.set_text_content(Some("M"));
    let mb_el: HtmlElement = mute_btn.clone().dyn_into().unwrap();
    let mute_color = if state == "mute" {
        "rgba(255, 0, 0, 0.8)"
    } else {
        "var(--text-muted)"
    };
    mb_el.style().set_css_text(&format!(
        "flex: 1; padding: 1px; border: 1px solid var(--border-medium); \
         background: transparent; color: {}; border-radius: 2px; \
         cursor: pointer; font-size: 7px; font-family: var(--font-mono); font-weight: 700;",
        mute_color,
    ));
    btn_row.append_child(&mute_btn).unwrap();

    let solo_btn = document.create_element("button").unwrap();
    solo_btn.set_text_content(Some("S"));
    let sb_el: HtmlElement = solo_btn.clone().dyn_into().unwrap();
    let solo_color = if state == "solo" {
        "rgba(255, 165, 0, 0.8)"
    } else {
        "var(--text-muted)"
    };
    sb_el.style().set_css_text(&format!(
        "flex: 1; padding: 1px; border: 1px solid var(--border-medium); \
         background: transparent; color: {}; border-radius: 2px; \
         cursor: pointer; font-size: 7px; font-family: var(--font-mono); font-weight: 700;",
        solo_color,
    ));
    btn_row.append_child(&solo_btn).unwrap();
    strip.append_child(&btn_row).unwrap();

    strip
}
