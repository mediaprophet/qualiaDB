//! P1 formal logic panels: epistemic, paraconsistent, LTL, CTL, ASP, defeasible,
//! linear, description, dialectical.

use super::helpers::{
    make_button, make_results_area, make_section_label, make_select, make_text_input,
    make_textarea, make_tool_panel, show_mock_results,
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

pub(super) fn build_epistemic_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "epistemic", false);
    panel
        .append_child(&make_section_label(
            document,
            "Epistemic Logic \u{2014} K(a,\u{03C6}) knows, B(a,\u{03C6}) affirms, common knowledge, introspection",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "epistemic-op",
        &[
            ("knows", "K(a,\u{03C6}) \u{2014} Knows"),
            ("affirms", "B(a,\u{03C6}) \u{2014} Affirms"),
            ("common", "C(\u{03C6}) \u{2014} Common Knowledge"),
            ("introspect_pos", "Positive Introspection"),
            ("introspect_neg", "Negative Introspection"),
        ],
    ))
    .unwrap();
    row.append_child(&make_text_input(document, "epistemic-agent", "Agent DID"))
        .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "epistemic-editor",
            "# Epistemic context\nknows(alice, deadlinePassed).\naffirms(bob, reportIsReady).\ncommon_knowledge(team, projectIsActive).\n\n# Certainty bands: KNOWS=255, AFFIRMS=230, AFFIRMS=200, RECOGNIZES=200,\n# CONSIDERS=128, SUPPOSES=100, SUSPECTS=80, SPECULATES=50, DOUBTS=20\n\n# Query: does alice know deadlinePassed?",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "epistemic-evaluate",
            "\u{1F9E0} Evaluate Frame",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "epistemic-results",
            "Evaluate the epistemic frame through the connected native logic engine.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_epistemic_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("epistemic-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "epistemic-results", "epistemic");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_paraconsistent_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "paraconsistent", false);
    panel
        .append_child(&make_section_label(
            document,
            "Paraconsistent Logic \u{2014} Belnap 4-valued, contradiction isolation, saturation",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "paraconsistent-editor",
            "# Route contradictions in the connected live graph.\n# Saturation is isolated / (consistent + isolated).\nthreshold=0.5",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "paraconsistent-evaluate",
            "\u{26A1} Route",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "paraconsistent-saturation",
            "\u{1F4CA} Saturation",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "paraconsistent-results",
            "Route contradictions through the native paraconsistent evaluator.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_paraconsistent_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("paraconsistent-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "paraconsistent-results", "paraconsistent");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("paraconsistent-saturation") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "paraconsistent-results", "paraconsistent-saturation");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_ltl_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "ltl", false);
    panel
        .append_child(&make_section_label(
            document,
            "LTL \u{2014} G (always), F (eventually), X (next), U (until), R (release)",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "ltl-editor",
            "# Operators: G, F, X, U, R. Trace items are predicate IRIs/names.\noperator=U\nleft=data_integrity\nright=publication_submitted\ntrace=[data_integrity|data_integrity|publication_submitted]\n\n# Safety Monitor uses G(invariant) over the same trace.\ninvariant=data_integrity",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "ltl-evaluate",
            "\u{23F1} Evaluate Trace",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "ltl-safety",
            "\u{1F6E1} Safety Monitor",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "ltl-results",
            "Evaluate a bounded temporal trace through the native LTL evaluator.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_ltl_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("ltl-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "ltl-results", "LTL");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("ltl-safety") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "ltl-results", "ltl-safety");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_ctl_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "ctl", false);
    panel
        .append_child(&make_section_label(
            document,
            "CTL \u{2014} branching-time: AG (all paths globally), EF (exists path eventually), AX, EX, AU, EU",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "ctl-editor",
            "# Operators: EF, AG, EX, AX, EU, AU, EG, AF.\noperator=EF\nstart=s0\nproposition=approved\n# `phi` is used by EU/AU.\nphi=review\ntransitions=[s0:s1|s1:s2|s1:s3|s2:s2|s3:s3]\nholds=[s0:init|s1:review|s2:approved|s3:rejected]",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "ctl-evaluate",
            "\u{1F534} Model Check",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "ctl-results",
            "Model-check a bounded branching-time frame with the native CTL evaluator.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_ctl_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("ctl-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "ctl-results", "CTL");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_asp_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "asp", false);
    panel
        .append_child(&make_section_label(
            document,
            "ASP \u{2014} Gelfond-Lifschitz reduct, stable models, weak constraints, brave/cautious",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "asp-editor",
            "# Bounded normal ASP (maximum 12 declared atoms).\natoms=[permit|forbid]\nrule=permit <- not forbid\nrule=forbid <- not permit\n# Weak constraints minimize incurred weight.\nweak=forbid : 1",
            "160px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "asp-evaluate",
            "\u{1F4A1} Compute Answer Sets",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "asp-optimal",
            "\u{1F3C6} Optimal Model",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "asp-results",
            "Enumerate bounded stable models through the native ASP evaluator.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_asp_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("asp-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "asp-results", "ASP");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("asp-optimal") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "asp-results", "asp-optimal");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_defeasible_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "defeasible", false);
    panel
        .append_child(&make_section_label(
            document,
            "Defeasible Logic \u{2014} strict/default/defeater conflict, superiority, and grounded justification",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "defeasible-editor",
            "literal=eligible\nrule_a=r1\nkind_a=defeasible\npositive_a=true\nrule_b=r2\nkind_b=defeasible\npositive_b=false\n# superior: r1, r2, or none\nsuperior=r1\nambiguity=blocking",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "defeasible-evaluate",
            "\u{2696} Evaluate Frame",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "defeasible-results",
            "Resolve the opposing rules and compute their grounded justification.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_defeasible_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("defeasible-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "defeasible-results", "defeasible");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_linear_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "linear", false);
    panel
        .append_child(&make_section_label(
            document,
            "Linear Logic \u{2014} tensor resource consumption and structural-rule licensing",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "linear-editor",
            "resource_a=budget\nresource_b=deliverable\nconsumed_a=false\nconsumed_b=false\nreusable_a=false\nreusable_b=false\n# weakening, contraction, or exchange\nstructural_rule=exchange",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "linear-evaluate",
            "\u{1F4B8} Check Resources",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "linear-results",
            "Attempt native tensor consumption and validate the selected structural rule.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_linear_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("linear-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "linear-results", "linear");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_description_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "description", false);
    panel
        .append_child(&make_section_label(
            document,
            "Description Logic \u{2014} SROIQ: subsumption, cardinality, disjointness, nominals",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "dl-editor",
            "# DL axioms\nSubClassOf(Contributor Agent).\nSubClassOf(Reviewer Contributor).\nDisjointClasses(Contributor Custodian).\nObjectMinCardinality(1 hasReview Reviewer).\n\n# Query: subsumes(Contributor, Reviewer)?\n# Query: concepts_disjoint(Contributor, Custodian)?\n# Query: min_cardinality_met(Reviewer, hasReview, 1)?",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "dl-evaluate",
            "\u{1F50D} Check Subsumption",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "dl-results",
            "Check class subsumption against the connected native TBox.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_description_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("dl-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "dl-results", "description-logic");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_dialectical_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "dialectical", false);
    panel
        .append_child(&make_section_label(
            document,
            "Dialectical Logic \u{2014} contradictory synthesis, IBIS support, and causal counterfactuals",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "dialectical-editor",
            "subject=architecture_decision\npredicate=preferred_stack\nthesis=rust\nantithesis=python\nsupporting=3\nobjecting=1\n\n# Counterfactual fields used by the secondary action.\ncausal_edges=[language:performance|performance:outcome]\nfactual_outcome=baseline\nintervention=language\nintervention_value=rust\ntarget=outcome",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "dialectical-evaluate",
            "\u{1F9ED} Synthesize",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "dialectical-counter",
            "\u{1F500} Counterfactual",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "dialectical-results",
            "Synthesize the contradiction or run the bounded causal counterfactual.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_dialectical_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("dialectical-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "dialectical-results", "dialectical");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("dialectical-counter") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "dialectical-results", "dialectical-counterfactual");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
