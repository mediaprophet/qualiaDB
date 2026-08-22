//! P2 extra modality panels: Allen Interval + RCC8 spatial, manifold logic,
//! calculus (RK4 / Simpson's / trapezoidal / adaptive / GPU integration).

use super::helpers::{
    make_button, make_results_area, make_section_label, make_select, make_text_input,
    make_textarea, make_tool_panel, show_mock_results,
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlSelectElement, MouseEvent};

pub(super) fn append_panels(document: &Document, content: &Element) {
    content
        .append_child(&build_allen_rcc8_panel(document))
        .unwrap();
    content
        .append_child(&build_manifold_logic_panel(document))
        .unwrap();
    content
        .append_child(&build_calculus_panel(document))
        .unwrap();
}

pub(super) fn wire_all(document: &Document) {
    wire_allen_rcc8_panel(document);
    wire_manifold_logic_panel(document);
    wire_calculus_panel(document);
}

pub(super) fn build_allen_rcc8_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "allen_rcc8", false);
    panel
        .append_child(&make_section_label(
            document,
            "Allen Interval Algebra + RCC8 \u{2014} temporal relations, spatial regions, Minkowski",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "allen-rcc8-op",
        &[
            ("allen", "Allen Interval Relation"),
            ("rcc8", "RCC8 Spatial Relation"),
            ("rcc8_points", "RCC8 from Points (zero-heap)"),
            ("spatial_index", "Spatial Index Query"),
            ("minkowski", "Minkowski Interval"),
            ("causally_connectable", "Causally Connectable"),
            ("heat_equation", "Heat Equation Step"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "allen-rcc8-input",
            "# Allen Interval Algebra (7 relations):\n# Before, Meets, Overlaps, Starts, During, Finishes, Equals\n#\n# RCC8 Spatial Relations (8):\n# Disconnected, ExternallyConnected, PartiallyOverlapping,\n# TangentiallyProperPart, TangentiallyProperPartInverse,\n# NonTangentialProperPart, NonTangentialProperPartInverse, Equal\n\n# Example: interval A=[0,10], B=[5,15]\n# Query: what Allen relation holds between A and B?\n# Query: RCC8 relation between region R1 and R2?\n# Query: spatial index query for point (3,4)?",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "allen-rcc8-evaluate",
            "\u{1F4D0} Evaluate",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "allen-rcc8-results",
            "Click \"Evaluate\" to compute spatial/temporal relation (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_allen_rcc8_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("allen-rcc8-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let op = doc
                .get_element_by_id("allen-rcc8-op")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            show_mock_results(&doc, "allen-rcc8-results", &format!("allen-rcc8-{}", op));
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_manifold_logic_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "manifold_logic", false);
    panel
        .append_child(&make_section_label(
            document,
            "Manifold Logic \u{2014} continuous-to-fact, wave valuation, integration",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "manifold-logic-input",
            "# Manifold logic context\n# continuous_to_fact: continuous value \u{2192} discrete Quin fact\n# integrate_abs: absolute value integration over manifold\n# wave_val: WaveCoord valuation\n# WaveCoord: amplitude, frequency, phase, damping\n\n# Query: convert continuous signal to fact?\n# Query: integrate |f(x)| over [0, 1]?\n# Query: compute wave value at t=0.5?",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "manifold-logic-evaluate",
            "\u{1F300} Evaluate",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "manifold-logic-results",
            "Click \"Evaluate\" to compute manifold logic (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_manifold_logic_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("manifold-logic-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "manifold-logic-results", "manifold-logic");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_calculus_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "calculus", false);
    panel
        .append_child(&make_section_label(
            document,
            "Calculus \u{2014} RK4 step, Simpson's integration, trapezoidal, adaptive, GPU integration",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "calculus-op",
        &[
            ("rk4_step", "RK4 Step"),
            ("simpsons", "Simpson's Integration"),
            ("trapezoidal", "Trapezoidal Integration"),
            ("adaptive", "Adaptive Step"),
            ("gpu_integration", "GPU Integration"),
            ("simd_width", "Detect SIMD Width"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_text_input(
            document,
            "calculus-fn",
            "Function (e.g. sin(x) * exp(-x))",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "calculus-input",
            "# Calculus parameters (operation-dependent)\n# RK4: dy/dt=f(y,t), y0=1.0, dt=0.01, steps=100\n# Simpson's: f(x), a=0, b=1, n=1000\n# Trapezoidal: f(x), a=0, b=1, n=1000\n# Adaptive: f(x), a=0, b=1, tol=1e-6\n# GPU: f(x), a=0, b=1, n=1000000\n# SIMD: detect AVX2/NEON width",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "calculus-evaluate",
            "\u{1F9EE} Compute",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "calculus-results",
            "Click \"Compute\" to evaluate calculus operation (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_calculus_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("calculus-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let op = doc
                .get_element_by_id("calculus-op")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            show_mock_results(&doc, "calculus-results", &format!("calculus-{}", op));
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
