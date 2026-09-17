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
            "# Allen: a=[start,end] b=[start,end]\na=[0,10]\nb=[5,15]\n\n# RCC8 / RCC8 points: flattened polygons\na_id=1\nb_id=2\na_points=[0,0,2,0,2,2,0,2]\nb_points=[1,1,3,1,3,3,1,3]\n\n# Spatial index: query AABB + flattened boxes\nquery=[0,0,2,2]\nboxes=[0,0,1,1,5,5,6,6]\n\n# Minkowski / causal\ndt=1\ndx=0\ndy=0\ndz=0\nc=1\n\n# Heat equation step\nu=[0,1,0]\nalpha=0.1\ndt=0.1\ndx=1",
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
            "Evaluate the selected Allen, RCC8, spatial-index, Minkowski, or heat-equation contract.",
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
            "# operation=wave_eval|integrate_abs|continuous_to_fact\noperation=continuous_to_fact\nsamples=[0.4,0.8,1.2]\nthreshold=0.5\nfact_id=7\n\n# wave_eval coordinates\nx=0\ny=0\nz=0\nt=0.5\nf=1\na=1\nphi=0",
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
            "Evaluate wave field, absolute integration, or continuous-to-fact against the native manifold-logic kernel.",
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
            ("large_grid", "Large-grid SIMD Integration"),
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
            "# Enter the function above and the selected operation's parameters\n# RK4: y0=1.0, dt=0.01, steps=100\n# Simpson/trapezoidal: a=0, b=1, n=1000\n# Adaptive: a=0, b=1, tol=1e-8, max_evaluations=10000\n# Large grid: a=0, b=1, n=100000\n# SIMD width requires no parameters",
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
            "Enter a supported symbolic expression and run the bounded native calculus operation.",
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
