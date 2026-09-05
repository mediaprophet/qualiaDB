//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Health calculator route and anatomy container.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Health (Framingham, clinical, consent-gated)
// ---------------------------------------------------------------------------

/// Health container — consent-gated clinical calculators.
pub fn build_health_view(document: &Document) -> Element {
    crate::browser::health_views::calculators::build_health_calculators_view(document)
}

// ---------------------------------------------------------------------------
// Anatomy (10D, organ percepts)
// ---------------------------------------------------------------------------

/// Anatomy container — 10D body view, organ percepts, comorbidity.
pub fn build_anatomy_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Stratum Selector
    let stratum_bar = document.create_element("div").unwrap();
    stratum_bar.set_class_name("vibe-toolbar");
    for (idx, stratum) in [
        "10D Visceral",
        "Neural Connectome",
        "Musculoskeletal",
        "Vascular",
    ]
    .iter()
    .enumerate()
    {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
        if idx == 0 {
            btn_el.style().set_css_text(
                "background: var(--accent-rose); color: var(--text-inverse); font-weight: 700;",
            );
        }
        btn.set_text_content(Some(stratum));
        stratum_bar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&stratum_bar).unwrap();

    // 10D Anatomical Stratum Grid
    let organ_grid = document.create_element("div").unwrap();
    let og_el: HtmlElement = organ_grid.clone().dyn_into().unwrap();
    og_el
        .style()
        .set_css_text("display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px;");

    let organs = [
        (
            "Heart \u{00B7} Cor",
            "FMA:7088",
            "98.4% Normal",
            "var(--accent-emerald)",
            "HRV: 68ms \u{00B7} EF: 62%",
        ),
        (
            "Lungs \u{00B7} Pulmo",
            "FMA:7195",
            "99.1% Clear",
            "var(--accent-emerald)",
            "SpO2: 99% \u{00B7} FEV1: 3.8L",
        ),
        (
            "Liver \u{00B7} Hepar",
            "FMA:7203",
            "97.5% Normal",
            "var(--accent-emerald)",
            "ALT: 22 U/L \u{00B7} AST: 20",
        ),
        (
            "Brain \u{00B7} Encephalon",
            "FMA:50801",
            "99.8% Coherent",
            "var(--accent-violet)",
            "Gamma: 40Hz \u{00B7} Alpha: 10Hz",
        ),
    ];

    for (name, fma, status, col, telemetry) in organs {
        let card = document.create_element("div").unwrap();
        card.set_class_name("cr-card");
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text("padding: 8px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); display: flex; flex-direction: column; gap: 3px; font-size: 10px;");

        let h_row = document.create_element("div").unwrap();
        let h_row_el: HtmlElement = h_row.clone().dyn_into().unwrap();
        h_row_el
            .style()
            .set_css_text("display: flex; justify-content: space-between; align-items: center;");
        let t_el = document.create_element("span").unwrap();
        t_el.set_text_content(Some(name));
        t_el.set_attribute("style", "font-weight: 700; color: var(--text-primary);")
            .unwrap();
        h_row.append_child(&t_el).unwrap();

        let s_el = document.create_element("span").unwrap();
        s_el.set_text_content(Some(status));
        s_el.set_attribute(
            "style",
            &format!("font-size: 9px; color: {}; font-weight: 600;", col),
        )
        .unwrap();
        h_row.append_child(&s_el).unwrap();
        card.append_child(&h_row).unwrap();

        let sub = document.create_element("div").unwrap();
        sub.set_attribute("style", "display: flex; justify-content: space-between; color: var(--text-muted); font-size: 9px;").unwrap();
        sub.set_inner_html(&format!("<span>{}</span><span>{}</span>", fma, telemetry));
        card.append_child(&sub).unwrap();

        organ_grid.append_child(&card).unwrap();
    }
    wrapper.append_child(&organ_grid).unwrap();

    // Comorbidity Scorecard
    let comorb = document.create_element("div").unwrap();
    comorb.set_class_name("cr-card");
    let comorb_el: HtmlElement = comorb.clone().dyn_into().unwrap();
    comorb_el.style().set_css_text("padding: 8px 10px; background: rgba(0,0,0,0.25); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 10px; display: flex; justify-content: space-between; align-items: center;");
    comorb.set_inner_html(
        "<div><span style='color: var(--accent-cyan); font-weight: 700;'>Charlson Index:</span> <span style='color: var(--text-primary);'>0 (10-Yr Survival: 99%)</span></div>\
         <div style='color: var(--accent-emerald); font-weight: 600;'>\u{2713} Low Comorbidity</div>"
    );
    wrapper.append_child(&comorb).unwrap();

    // Action Toolbar
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &[
        "Mount 10D Manifold",
        "Comorbidity Risk",
        "Extract SNOMED",
        "Export FHIR",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}
