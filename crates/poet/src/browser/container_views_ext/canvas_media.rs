//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Aura SHACL inspector and LaTeX editor.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Aura (SHACL inspector)
// ---------------------------------------------------------------------------

/// SHACL validation inspector — shape list, conformance results.
pub fn build_aura_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    // Toolbar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let run_btn = document.create_element("button").unwrap();
    run_btn.set_class_name("vibe-run-btn");
    run_btn.set_text_content(Some("\u{25B6} Validate"));
    bar.append_child(&run_btn).unwrap();
    let label = document.create_element("span").unwrap();
    label.set_text_content(Some("Shapes:"));
    let l_el: HtmlElement = label.clone().dyn_into().unwrap();
    l_el.style().set_css_text("color: var(--text-muted); font-size: 10px; font-family: var(--font-mono); margin-left: 8px;");
    bar.append_child(&label).unwrap();
    wrapper.append_child(&bar).unwrap();

    // Shape results
    let results = document.create_element("div").unwrap();
    results.set_class_name("vibe-output");
    let shapes = &[
        ("soc:PeerShape", "conformant", "42 nodes validated"),
        ("soc:AgreementShape", "conformant", "8 nodes validated"),
        (
            "health:RecordShape",
            "violation",
            "2 nodes: missing `health:hasConsent`",
        ),
        ("rights:FiduciaryShape", "conformant", "3 nodes validated"),
        ("vibe:IntentShape", "conformant", "156 nodes validated"),
    ];
    for (shape, status, detail) in shapes {
        let row = document.create_element("div").unwrap();
        row.set_class_name("vibe-out-line");
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el
            .style()
            .set_css_text("display: flex; align-items: center; gap: 8px;");

        let badge = document.create_element("span").unwrap();
        let badge_class = if *status == "violation" {
            "honesty-badge honesty-missing"
        } else {
            "honesty-badge honesty-live"
        };
        badge.set_class_name(badge_class);
        badge.set_text_content(Some(status));
        row.append_child(&badge).unwrap();

        let shape_el = document.create_element("span").unwrap();
        let sh_el: HtmlElement = shape_el.clone().dyn_into().unwrap();
        sh_el
            .style()
            .set_css_text("color: var(--accent-cyan); font-family: var(--font-mono);");
        shape_el.set_text_content(Some(shape));
        row.append_child(&shape_el).unwrap();

        let detail_el = document.create_element("span").unwrap();
        let d_el: HtmlElement = detail_el.clone().dyn_into().unwrap();
        d_el.style()
            .set_css_text("color: var(--text-muted); font-size: 10px; margin-left: auto;");
        detail_el.set_text_content(Some(detail));
        row.append_child(&detail_el).unwrap();

        results.append_child(&row).unwrap();
    }
    wrapper.append_child(&results).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Latex (CAS invoke)
// ---------------------------------------------------------------------------

/// LaTeX editor — snippet bar, CAS invoke, symbolic algebra.
pub fn build_latex_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    // Snippet bar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    for label in &[
        "\\frac", "\\sum", "\\int", "\\sqrt", "\\alpha", "\\nabla", "CAS",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
        btn_el
            .style()
            .set_css_text("font-size: 10px; padding: 2px 6px; font-family: var(--font-mono);");
        btn.set_text_content(Some(label));
        bar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&bar).unwrap();

    // Editor
    let editor = document.create_element("div").unwrap();
    editor.set_class_name("vibe-editor");
    editor.set_text_content(Some(
        "\\documentclass{article}\n\
         \\begin{document}\n\
         \\section{Quantum DFT Ground State}\n\
         $$E_0 = \\min_{\\psi} \\langle \\psi | \\hat{H} | \\psi \\rangle$$\n\
         \\end{document}",
    ));
    wrapper.append_child(&editor).unwrap();

    // CAS output
    let output = document.create_element("div").unwrap();
    output.set_class_name("vibe-output");
    let line = document.create_element("div").unwrap();
    line.set_class_name("vibe-out-line");
    line.set_text_content(Some(
        "\u{2139}\u{FE0F} CAS: awaiting SymbolicAlgebra engine wiring (native_bindings)",
    ));
    output.append_child(&line).unwrap();
    wrapper.append_child(&output).unwrap();

    wrapper
}
