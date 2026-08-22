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
            "Abductive Reasoning \u{2014} ATMS, best explanation inference, probabilistic abduction",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "abductive-editor",
            "# Abductive context\nobserve(symptom(fever)).\nobserve(symptom(cough)).\nhypothesis(flu, explains([fever, cough])).\nhypothesis(cold, explains([cough])).\n\n# Query: best explanation for [fever, cough]?\n# Query: ATMS label for flu?",
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
            "Click \"Best Explanation\" to rank hypotheses (mock).",
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
            "Fuzzy Logic \u{2014} t-norm (G\u{00F6}del), quantifiers, type-2 fuzzy sets, fuzzy RDF schema",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "fuzzy-editor",
            "# Fuzzy context\nmembership(alice, contributor, 0.8).\nmembership(bob, contributor, 0.6).\nmembership(carol, contributor, 0.3).\n\n# Quantifiers: most, many, few, about_half\n# Query: t_norm_godel(0.8, 0.6)?\n# Query: type-2 fuzzy union of alice and bob?",
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
            "Click \"Evaluate Fuzzy\" to compute membership (mock).",
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
            "Probabilistic Reasoning \u{2014} Bayesian networks, threshold evaluation, evidence weighting",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "probabilistic-editor",
            "# Bayesian network\nnode(rain, [yes, no]).\nnode(sprinkler, [on, off]).\nnode(wetGrass, [true, false]).\np(rain=yes)=0.2.\np(sprinkler=on|rain=yes)=0.01.\np(sprinkler=on|rain=no)=0.4.\np(wetGrass=true|rain=yes,sprinkler=on)=0.99.\n\n# Query: P(rain=yes|wetGrass=true)?\n# Query: evaluate_threshold(0.5)?",
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
            "Click \"Evaluate Bayesian\" to compute posterior (mock).",
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
            "Graph Theory \u{2014} topology, centrality, communities, motifs, subgraph isomorphism",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "graph-theory-editor",
            "# Graph context\nedge(alice, bob).\nedge(bob, carol).\nedge(carol, alice).\nedge(alice, dave).\n\n# Query: bounded topology analysis (zero-heap, 128 nodes)?\n# Query: community spans?\n# Query: motif count (triangles)?\n# Query: subgraph pattern match?",
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
            "Click \"Analyze Topology\" to compute graph metrics (mock).",
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
            "# Interval context\ninterval(task1, [0, 10]).\ninterval(task2, [5, 15]).\ninterval(task3, [15, 25]).\n\n# Allen relations: Before, Meets, Overlaps, Starts, During, Finishes, Equals\n# Query: Allen relation between task1 and task2?\n# Query: Minkowski sum of [0,10] and [5,15]?",
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
            "Click \"Evaluate Intervals\" to compute Allen relations (mock).",
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
            "# 10D manifold coordinate\n# Dimensions: scale, attention_depth, epistemic_weight, topological_spin,\n#   temporal_decay, entropy_bias, spatial_phase, recurrence_frequency,\n#   density_threshold, manifold_curvature\n\nstate(s0, [1.0, 0.5, 0.8, 0.2, 0.9, 0.1, 0.3, 0.7, 0.4, 0.6]).\nstate(s1, [1.2, 0.6, 0.7, 0.3, 0.8, 0.2, 0.4, 0.6, 0.5, 0.5]).\n\n# Query: project 10D to quaternion?\n# Query: manifold LTL trace evaluation?\n# Query: manifold answer sets (ASP)?",
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
            "Click \"Project 10D\" to compute manifold state (mock).",
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
            "# Epistemic boundaries context\nclaim(alice, \"patient has condition X\").\nseverity(medical, high).\nseverity(legal, medium).\n\n# Socratic degradation: definitive -> probable -> possible -> speculative -> socratic\n# Referral domains: medical, legal, bio\n# Query: degrade claim to socratic?\n# Query: requires physiological quarantine?\n# Query: forbids definitive classification?",
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
            "Click \"Check Boundaries\" to evaluate safety (mock).",
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
            "# Modal logic context\nnecessary(data_integrity).\npossible(publication_delay).\nnecessary(necessary(access_control)).\n\n# Systems: K (no reflexivity), T (reflexive), S4 (reflexive+transitive), S5 (equivalence)\n# Query: evaluate in S5: necessary(data_integrity) -> possible(data_integrity)?\n# Query: evaluate in S4: necessary(necessary(p)) -> necessary(p)?",
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
            "Click \"Evaluate Modal\" to check modal formula (mock).",
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
