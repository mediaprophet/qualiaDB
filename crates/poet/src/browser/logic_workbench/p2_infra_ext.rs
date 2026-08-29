//! P2 infrastructure extension panels: CRDT/sync, agency/Merkle, key vault,
//! policy evaluator, consent manager, carrier/media, control feedback,
//! likeliness, QUBO compiler, OWL converter.

use super::helpers::{
    make_button, make_results_area, make_section_label, make_select, make_text_input,
    make_textarea, make_tool_panel, show_mock_results,
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

pub(super) fn append_panels(document: &Document, content: &Element) {
    content
        .append_child(&build_crdt_sync_panel(document))
        .unwrap();
    content
        .append_child(&build_agency_merkle_panel(document))
        .unwrap();
    content
        .append_child(&build_key_vault_panel(document))
        .unwrap();
    content
        .append_child(&build_policy_evaluator_panel(document))
        .unwrap();
    content
        .append_child(&build_consent_manager_panel(document))
        .unwrap();
    content
        .append_child(&build_carrier_panel(document))
        .unwrap();
    content
        .append_child(&build_control_feedback_panel(document))
        .unwrap();
    content
        .append_child(&build_likeliness_panel(document))
        .unwrap();
    content.append_child(&build_qubo_panel(document)).unwrap();
    content
        .append_child(&build_owl_converter_panel(document))
        .unwrap();
}

pub(super) fn wire_all(document: &Document) {
    wire_crdt_sync_panel(document);
    wire_agency_merkle_panel(document);
    wire_key_vault_panel(document);
    wire_policy_evaluator_panel(document);
    wire_consent_manager_panel(document);
    wire_carrier_panel(document);
    wire_control_feedback_panel(document);
    wire_likeliness_panel(document);
    wire_qubo_panel(document);
    wire_owl_converter_panel(document);
}

pub(super) fn build_crdt_sync_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "crdt_sync", false);
    panel
        .append_child(&make_section_label(
            document,
            "CRDT / Sync Dashboard \u{2014} LWW resolution, delegated access, suspended transactions",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "crdt-sync-input",
            "local_clock=1\nremote_clock=2\nlocal_object=10\nremote_object=20\nselfhood=false\nprincipal=did:alice\ndelegate=did:bob\ncontext=graph:1\nexpiry=100\nnow=1",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "crdt-sync-evaluate",
            "\u{1F501} Resolve",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "crdt-sync-results",
            "Resolve LWW by Lamport clock and verify the typed delegation window.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_crdt_sync_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("crdt-sync-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "crdt-sync-results", "crdt-sync");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_agency_merkle_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "agency_merkle", false);
    panel
        .append_child(&make_section_label(
            document,
            "Agency / Merkle Inspector \u{2014} scoped Merkle root, Ed25519 signing, fiduciary metadata",
        ))
        .unwrap();
    panel
        .append_child(&make_text_input(
            document,
            "agency-merkle-did",
            "Agent DID (did:qualia:...)",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "agency-merkle-input",
            "claims=[contribution|attestation|key-rotation]",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "agency-merkle-evaluate",
            "\u{1F511} Inspect Agency",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "agency-merkle-results",
            "Compute the author-scoped SHA-256 Merkle sub-root over the supplied claims.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_agency_merkle_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("agency-merkle-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "agency-merkle-results", "agency-merkle");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_key_vault_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "key_vault", false);
    panel
        .append_child(&make_section_label(
            document,
            "Key Vault Manager \u{2014} HE key management, rotation policy, 8 key slots",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "key-vault-input",
            "operation=register\nkey_id=key-0\ncreated_at=1\nexpires_at=1000\nnow=10",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "key-vault-evaluate",
            "\u{1F510} Manage Keys",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "key-vault-results",
            "Register HE key metadata in the 8-slot vault and drop expired entries. This does not generate BFV ciphertext keys.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_key_vault_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("key-vault-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "key-vault-results", "key-vault");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_policy_evaluator_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "policy_evaluator", false);
    panel
        .append_child(&make_section_label(
            document,
            "Policy Evaluator \u{2014} access policy with sensitivity + epistemic status",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "policy-evaluator-input",
            "subject=did:alice\nresource=doc:42\nclearance=restricted\nsensitivity=confidential\nepistemic=active",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "policy-evaluator-evaluate",
            "\u{1F6E1} Evaluate Policy",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "policy-evaluator-results",
            "Compare clearance to resource sensitivity and fail closed on uncertain epistemic status.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_policy_evaluator_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("policy-evaluator-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "policy-evaluator-results", "policy-evaluator");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_consent_manager_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "consent_manager", false);
    panel
        .append_child(&make_section_label(
            document,
            "Consent Manager \u{2014} grant, revoke, list consents per scope",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "consent-op",
        &[
            ("grant", "Grant Consent"),
            ("revoke", "Revoke Consent"),
            ("list", "List Consents"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_text_input(
            document,
            "consent-scope",
            "Scope (e.g. health:records)",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "consent-input",
            "expiry=100\nnow=10\nrevoked=false",
            "80px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "consent-evaluate",
            "\u{2705} Execute",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "consent-results",
            "Evaluate whether the scoped grant is in force at `now`, including revoke.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_consent_manager_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("consent-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "consent-results", "consent-manager");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_carrier_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "carrier", false);
    panel
        .append_child(&make_section_label(
            document,
            "Carrier / Media Binding \u{2014} payload extraction, media tags, binding verification",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "carrier-input",
            "payload=hello",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "carrier-evaluate",
            "\u{1F4E6} Process Carrier",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "carrier-results",
            "BLAKE3-tag the payload and verify the carrier binding.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_carrier_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("carrier-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "carrier-results", "carrier");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_control_feedback_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "control_feedback", false);
    panel
        .append_child(&make_section_label(
            document,
            "Control Feedback \u{2014} control state, stabilization, feedback loops",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "control-feedback-input",
            "setpoint=100\nmeasured=95\nt=1",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "control-feedback-evaluate",
            "\u{1F501} Compute",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "control-feedback-results",
            "Run one conservative PID step against the native control-feedback kernel.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_control_feedback_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("control-feedback-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "control-feedback-results", "control-feedback");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_likeliness_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "likeliness", false);
    panel
        .append_child(&make_section_label(
            document,
            "Likeliness \u{2014} likeliness algebra, plausibility assessment",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "likeliness-input",
            "premises=[2,1,-1]",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "likeliness-evaluate",
            "\u{1F4CF} Evaluate",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "likeliness-results",
            "Combine ordinal likeliness premises with the Kleene/De Morgan meet.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_likeliness_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("likeliness-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "likeliness-results", "likeliness");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_qubo_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "qubo", false);
    panel
        .append_child(&make_section_label(
            document,
            "QUBO Compiler \u{2014} Quadratic Unconstrained Binary Optimization compilation",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "qubo-input",
            "edges=[a:b|b:c]",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "qubo-evaluate",
            "\u{1F9EE} Compile",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "qubo-results",
            "Compile obligation edges into a QUBO and run the classical greedy solver.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_qubo_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("qubo-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "qubo-results", "qubo");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_owl_converter_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "owl_converter", false);
    panel
        .append_child(&make_section_label(
            document,
            "OWL Converter \u{2014} OWL\u{2192}SHACL conversion, OWL materialization",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "owl-op",
        &[
            ("to_shacl", "OWL \u{2192} SHACL"),
            ("materialize", "OWL Materialization"),
            ("classify", "OWL Classification"),
            ("consistency", "OWL Consistency Check"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "owl-input",
            "triples=[Student:subClassOf:Person|Person:subClassOf:Agent]",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "owl-evaluate",
            "\u{1F527} Convert",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "owl-results",
            "Run bounded OWL 2 RL materialization over subject:predicate:object axioms.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_owl_converter_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("owl-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "owl-results", "owl-converter");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
