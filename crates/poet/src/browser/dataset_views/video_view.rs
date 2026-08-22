//! Video View — video player with timeline scrubber + annotation overlays (§4.2, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const VIDEO_INFO: &[(&str, &str)] = &[
    ("Title", "Field Recording \u{2014} Sydney Harbour"),
    ("Source", "DS-007"),
    ("Duration", "00:04:32.180"),
    ("Resolution", "3840 x 2160 (4K)"),
    ("Codec", "H.265 / HEVC"),
    ("Frame Rate", "24 fps"),
    ("Bit Rate", "45 Mbps"),
    ("Sensitivity", "Public"),
    ("Provenance", "did:qualia:timothy_charles_holborn"),
];

const TIMELINE_MARKERS: &[(&str, f64, &str)] = &[
    ("M1", 12.5, "Camera pan starts"),
    ("M2", 45.0, "Subject enters frame"),
    ("M3", 120.0, "Audio peak \u{2014} ferry horn"),
    ("M4", 210.0, "Zoom in"),
    ("M5", 320.0, "End of scene"),
];

pub fn build_video_view_view(document: &Document) -> Element {
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
        "Play",
        "Pause",
        "Stop",
        "+ Marker",
        "Export Frame",
        "Audio Waveform",
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

    // Video viewport placeholder
    let viewport = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport.clone().dyn_into().unwrap();
    vp_el.style().set_css_text(
        "height: 120px; background: var(--surface-panel); border-radius: 6px; \
         margin-bottom: 6px; display: flex; align-items: center; justify-content: center; \
         border: 1px solid var(--border-subtle); position: relative;",
    );
    let ph = document.create_element("div").unwrap();
    ph.set_text_content(Some("Video Player \u{2014} 4K H.265 (not wired)"));
    let p_el: HtmlElement = ph.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    viewport.append_child(&ph).unwrap();

    // Time display overlay
    let time_overlay = document.create_element("div").unwrap();
    time_overlay.set_text_content(Some("00:01:23.456 / 00:04:32.180"));
    let to_el: HtmlElement = time_overlay.clone().dyn_into().unwrap();
    to_el.style().set_css_text(
        "position: absolute; bottom: 4px; right: 8px; font-size: 9px; \
         color: var(--text-primary); font-family: var(--font-mono); \
         background: rgba(0,0,0,0.5); padding: 1px 4px; border-radius: 2px;",
    );
    viewport.append_child(&time_overlay).unwrap();
    content.append_child(&viewport).unwrap();

    // Scrub bar with markers
    let scrub_bg = document.create_element("div").unwrap();
    let sb_el: HtmlElement = scrub_bg.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "height: 20px; background: var(--surface-panel); border-radius: 3px; \
         margin-bottom: 6px; position: relative; border: 1px solid var(--border-subtle);",
    );

    // Playhead
    let playhead = document.create_element("div").unwrap();
    let ph_el: HtmlElement = playhead.clone().dyn_into().unwrap();
    ph_el.style().set_css_text(
        "position: absolute; left: 30%; top: 0; bottom: 0; width: 2px; \
         background: var(--accent-cyan);",
    );
    scrub_bg.append_child(&playhead).unwrap();

    // Markers
    for (_id, time, _label) in TIMELINE_MARKERS {
        let marker = document.create_element("div").unwrap();
        let m_el: HtmlElement = marker.clone().dyn_into().unwrap();
        let pct = time / 272.18 * 100.0;
        m_el.style().set_css_text(&format!(
            "position: absolute; left: {}%; top: 0; bottom: 0; width: 1px; \
             background: rgba(255, 165, 0, 0.6);",
            pct,
        ));
        scrub_bg.append_child(&marker).unwrap();
    }
    content.append_child(&scrub_bg).unwrap();

    // Audio waveform mock
    let wave_header = document.create_element("div").unwrap();
    wave_header.set_text_content(Some("Audio Waveform"));
    let wh_el: HtmlElement = wave_header.clone().dyn_into().unwrap();
    wh_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); \
         margin-bottom: 2px;",
    );
    content.append_child(&wave_header).unwrap();

    let wave_area = document.create_element("div").unwrap();
    let wa_el: HtmlElement = wave_area.clone().dyn_into().unwrap();
    wa_el.style().set_css_text(
        "height: 40px; background: var(--surface-panel); border-radius: 4px; \
         display: flex; align-items: center; gap: 1px; padding: 2px; \
         border: 1px solid var(--border-subtle); margin-bottom: 8px;",
    );
    for i in 0..80 {
        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        let h = 10.0 + ((i * 7 + 3) % 80) as f64 * 0.4;
        b_el.style().set_css_text(&format!(
            "flex: 1; height: {}%; background: rgba(0, 200, 255, 0.3); border-radius: 1px;",
            h,
        ));
        wa_el.append_child(&bar).unwrap();
    }
    content.append_child(&wave_area).unwrap();

    // Markers list
    let markers_header = document.create_element("div").unwrap();
    markers_header.set_text_content(Some("Timeline Markers (5)"));
    let mh_el: HtmlElement = markers_header.clone().dyn_into().unwrap();
    mh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&markers_header).unwrap();

    let markers_table = make_table(document, &["ID", "Time (s)", "Label"]);
    let markers_tbody = document.create_element("tbody").unwrap();
    for (id, time, label) in TIMELINE_MARKERS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![id.to_string(), format!("{:.1}", time), label.to_string()];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 0 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 8px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: rgba(255, 165, 0, 0.8); font-size: 9px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 9px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        markers_tbody.append_child(&tr).unwrap();
    }
    markers_table.append_child(&markers_tbody).unwrap();
    content.append_child(&markers_table).unwrap();

    // Video info
    let info_header = document.create_element("div").unwrap();
    info_header.set_text_content(Some("Video Info"));
    let ih_el: HtmlElement = info_header.clone().dyn_into().unwrap();
    ih_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&info_header).unwrap();

    let info_table = make_table(document, &["Field", "Value"]);
    let info_tbody = document.create_element("tbody").unwrap();
    for (field, value) in VIDEO_INFO {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![field.to_string(), value.to_string()];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        info_tbody.append_child(&tr).unwrap();
    }
    info_table.append_child(&info_tbody).unwrap();
    content.append_child(&info_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} video view requires DAT-28 media engine.",
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
