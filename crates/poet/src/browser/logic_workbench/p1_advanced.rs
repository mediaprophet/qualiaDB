//! P1 advanced panels: abductive, fuzzy, probabilistic, graph theory, interval,
//! manifold 10D, epistemic boundaries, modal.

use super::helpers::{
    make_button, make_results_area, make_section_label, make_textarea, make_tool_panel,
    show_mock_results,
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

pub(super) fn build_abductive_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "abductive", false);
    panel
        .append_child(&make_section_label(
            document,
            "Abductive Reasoning \u{2014} bounded Bayesian best-explanation ranking",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "abductive-editor",
            "# id:prior:P(observation|id)\nhypotheses=[flu:0.2:0.9|cold:0.8:0.1]",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "abductive-evaluate",
            "\u{1F50D} Best Explanation",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "abductive-results",
            "Rank 1\u{2013}32 hypotheses using the native Bayesian abduction evaluator.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_abductive_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("abductive-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "abductive-results", "abductive");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_fuzzy_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "fuzzy", false);
    panel
        .append_child(&make_section_label(
            document,
            "Fuzzy Logic \u{2014} G\u{00F6}del, \u{0141}ukasiewicz, product, and drastic t-norm/t-conorm families",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "fuzzy-editor",
            "# operations: godel_and, godel_or, lukasiewicz_and, lukasiewicz_or,\n# product_and, product_or, drastic_and, drastic_or\noperation=godel_and\na=0.8\nb=0.6",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "fuzzy-evaluate",
            "\u{1F4A0} Evaluate Fuzzy",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "fuzzy-results",
            "Evaluate two truth degrees with a native fuzzy operator.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_fuzzy_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("fuzzy-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "fuzzy-results", "fuzzy");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_probabilistic_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "probabilistic", false);
    panel
        .append_child(&make_section_label(
            document,
            "Probabilistic Reasoning \u{2014} Bayesian evidence update and posterior thresholding",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "probabilistic-editor",
            "# P(hypothesis), P(evidence|hypothesis), P(evidence|not hypothesis)\nprior=0.2\nlikelihood_true=0.9\nlikelihood_false=0.1\nthreshold=0.5",
            "160px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "probabilistic-evaluate",
            "\u{1F4B9} Evaluate Bayesian",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "probabilistic-results",
            "Compute a two-state Bayesian posterior and threshold decision.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_probabilistic_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("probabilistic-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "probabilistic-results", "probabilistic");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_graph_theory_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "graph_theory", false);
    panel
        .append_child(&make_section_label(
            document,
            "Graph Theory \u{2014} bounded topology, centrality, communities, and motifs",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "graph-theory-editor",
            "# directed from:to pairs (maximum 128 edges)\nedges=[alice:bob|bob:carol|carol:alice|alice:dave]",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "graph-theory-evaluate",
            "\u{1F5C2} Analyze Topology",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "graph-theory-results",
            "Analyze the bounded graph with the native topology engine.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_graph_theory_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("graph-theory-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "graph-theory-results", "graph-theory");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_interval_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "interval", false);
    panel
        .append_child(&make_section_label(
            document,
            "Interval Reasoning \u{2014} Allen's relations, temporal intervals, Minkowski sums",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "interval-editor",
            "# closed integer intervals\na=[0,10]\nb=[5,15]\n# Returns the full 13-relation Allen classification, sum, and intersection.",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "interval-evaluate",
            "\u{23F1} Evaluate Intervals",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "interval-results",
            "Classify two intervals and compute their Minkowski sum.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_interval_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("interval-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "interval-results", "interval");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_manifold_10d_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "manifold_10d", false);
    panel
        .append_child(&make_section_label(
            document,
            "Manifold 10D \u{2014} 10-dimensional coordinates, LTL projection, ASP topology, quaternion",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "manifold-10d-editor",
            "# Upper triangle of a symmetric 4x4 matrix (10 finite values).\n# The native fixed Lanczos solver returns its smallest-eigenvector quaternion.\nparameters=[1.0,0.5,0.8,0.2,0.9,0.1,0.3,0.7,0.4,0.6]",
            "160px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "manifold-10d-evaluate",
            "\u{1F300} Project 10D",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "manifold-10d-results",
            "Project 10D parameters to a normalized quaternion with the native manifold solver.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_manifold_10d_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("manifold-10d-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "manifold-10d-results", "manifold-10d");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_epistemic_boundaries_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "epistemic_boundaries", false);
    panel
        .append_child(&make_section_label(
            document,
            "Epistemic Boundaries \u{2014} Socratic degradation, referral triggers, physiological quarantine",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "epistemic-boundaries-editor",
            "# Known predicates include q42:medicalDiagnosis, q42:genomicAlignment,\n# q42:legalVerdict, q42:governancePolicy, and q42:investmentDirective.\nsubject=alice\npredicate=q42:medicalDiagnosis\nseverity=200",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "epistemic-boundaries-evaluate",
            "\u{1F6E1} Check Boundaries",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "epistemic-boundaries-results",
            "Apply structural refusal, quarantine, Socratic degradation, and referral gates.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_epistemic_boundaries_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("epistemic-boundaries-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "epistemic-boundaries-results", "epistemic-boundaries");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_modal_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "modal", false);
    panel
        .append_child(&make_section_label(
            document,
            "Modal Logic \u{2014} \u{25FB} necessity, \u{25CB} possibility, K/T/S4/S5 systems",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "modal-editor",
            "# Systems: K, T, D, B, S4, S5. Operators: necessary, possible.\nsystem=K\noperator=necessary\nworld=w0\nproposition=data_integrity\nworlds=[w0|w1]\naccesses=[w0:w1]\nholds_in=[w1]",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "modal-evaluate",
            "\u{25FB} Evaluate Modal",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "modal-results",
            "Evaluate a Kripke frame and verify its selected normal modal system.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_modal_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("modal-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "modal-results", "modal");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
