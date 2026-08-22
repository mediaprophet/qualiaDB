//! HRTF Personalization — individualized HRTF profile editor (§6.2, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const HRTF_PROFILES: &[(&str, &str, &str, &str)] = &[
    ("Default MIT", "MIT KEMAR", "Mannequin", "Public domain"),
    (
        "Subjective A",
        "Custom measurement",
        "did:qualia:timothy_charles_holborn",
        "Selfhood",
    ),
    ("ARI HRTF", "ARI/ARIHRTF v2", "Database", "CC BY-NC 4.0"),
    ("SADIE", "SADIE Database", "Database", "CC BY 4.0"),
    ("CIPIC", "CIPIC v1", "Database", "CC BY-NC 4.0"),
];

const EAR_PARAMS: &[(&str, f64, &str)] = &[
    ("Head radius", 0.087, "m"),
    ("Pinna height", 0.062, "m"),
    ("Pinna width", 0.038, "m"),
    ("Ear canal length", 0.025, "m"),
    ("ITD max", 0.00062, "s"),
    ("Concha depth", 0.012, "m"),
    ("Cymba height", 0.008, "m"),
    ("Lobe height", 0.020, "m"),
];

const CALIBRATION: &[(&str, &str, &str)] = &[
    ("Front (0\u{00B0})", "Localised correctly", "Pass"),
    ("Right (90\u{00B0})", "Localised correctly", "Pass"),
    ("Rear (180\u{00B0})", "Slight front-back confusion", "Warn"),
    ("Left (270\u{00B0})", "Localised correctly", "Pass"),
    ("Above (90\u{00B0} elev)", "Localised correctly", "Pass"),
    (
        "Below (-45\u{00B0} elev)",
        "Poor elevation accuracy",
        "Fail",
    ),
];

pub fn build_hrtf_personalization_view(document: &Document) -> Element {
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
    for label in &["Measure HRTF", "Import SOFA", "Calibrate", "Export Profile"] {
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

    // Sensitivity warning
    let warning = document.create_element("div").unwrap();
    warning.set_text_content(Some(
        "\u{26A0} HRTF profiles are Selfhood sensitivity. Sharing requires explicit consent.",
    ));
    let w_el: HtmlElement = warning.clone().dyn_into().unwrap();
    w_el.style().set_css_text(
        "padding: 4px 8px; background: rgba(200, 150, 255, 0.1); border-radius: 4px; \
         margin-bottom: 8px; font-size: 8px; color: rgba(200, 150, 255, 0.8); \
         font-family: var(--font-mono);",
    );
    content.append_child(&warning).unwrap();

    // Active profile
    let active = document.create_element("div").unwrap();
    active.set_text_content(Some(
        "Active Profile: Subjective A (Custom)  |  Source: did:qualia:timothy_charles_holborn  |  \
         Created: 2026-08-12  |  Calibration: 4/6 pass",
    ));
    let a_el: HtmlElement = active.clone().dyn_into().unwrap();
    a_el.style().set_css_text(
        "padding: 6px 8px; background: var(--surface-panel); border-radius: 4px; \
         margin-bottom: 8px; font-size: 9px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    content.append_child(&active).unwrap();

    // Ear parameters
    let ear_header = document.create_element("div").unwrap();
    ear_header.set_text_content(Some("Anthropometric Parameters"));
    let eh_el: HtmlElement = ear_header.clone().dyn_into().unwrap();
    eh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&ear_header).unwrap();

    let ear_table = make_table(document, &["Parameter", "Value", "Unit"]);
    let ear_tbody = document.create_element("tbody").unwrap();
    for (name, value, unit) in EAR_PARAMS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![name.to_string(), format!("{:.5}", value), unit.to_string()];
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
        ear_tbody.append_child(&tr).unwrap();
    }
    ear_table.append_child(&ear_tbody).unwrap();
    content.append_child(&ear_table).unwrap();

    // Calibration results
    let cal_header = document.create_element("div").unwrap();
    cal_header.set_text_content(Some("Localisation Calibration"));
    let ch_el: HtmlElement = cal_header.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&cal_header).unwrap();

    let cal_table = make_table(document, &["Direction", "Result", "Status"]);
    let cal_tbody = document.create_element("tbody").unwrap();
    for (dir, result, status) in CALIBRATION {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![dir.to_string(), result.to_string(), status.to_string()];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match &**status {
                    "Pass" => "rgba(100, 200, 100, 0.8)",
                    "Warn" => "rgba(255, 165, 0, 0.8)",
                    "Fail" => "rgba(255, 0, 0, 0.8)",
                    _ => "var(--text-muted)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 700; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        cal_tbody.append_child(&tr).unwrap();
    }
    cal_table.append_child(&cal_tbody).unwrap();
    content.append_child(&cal_table).unwrap();

    // Profile library
    let lib_header = document.create_element("div").unwrap();
    lib_header.set_text_content(Some("HRTF Profile Library (5)"));
    let lh_el: HtmlElement = lib_header.clone().dyn_into().unwrap();
    lh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&lib_header).unwrap();

    let lib_table = make_table(document, &["Profile", "Source", "Type", "License"]);
    let lib_tbody = document.create_element("tbody").unwrap();
    for (name, source, ptype, license) in HRTF_PROFILES {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            source.to_string(),
            ptype.to_string(),
            license.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 0 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else if i == 2 {
                let color = if ptype.contains("did:") {
                    "rgba(200, 150, 255, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        lib_tbody.append_child(&tr).unwrap();
    }
    lib_table.append_child(&lib_tbody).unwrap();
    content.append_child(&lib_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} HRTF personalization requires AUD-17 HRTF engine + SOFA import.",
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
