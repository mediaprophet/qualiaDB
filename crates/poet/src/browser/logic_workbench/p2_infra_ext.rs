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
            "# CRDT sync context\n# DelegatedAccess: principal_did[32], delegate_did[32], context_bound, expiration, proof[64]\n# SuspendedTransaction: agreement_id, threshold, collected_signatures, registers[16]\n# Queue depth: 32\n\n# Query: resolve LWW for key X?\n# Query: verify delegation?\n# Query: apply consensus token?\n# Query: list suspended transactions?",
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
            "Click \"Resolve\" to check CRDT state (mock).",
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
            "# Agency inspection\n# compute_scoped_merkle_root: subgraph \u{2192} BLAKE3 root\n# sign_agency_root: Ed25519 signature over root\n# verify_human_agency: checks human principal\n# stamp_fiduciary_metadata: adds fiduciary tag\n# derive_lane_key: PBKDF2, 310,000 iterations\n\n# Query: compute scoped Merkle root for subgraph X?\n# Query: verify human agency for did:qualia:...?",
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
            "Click \"Inspect Agency\" to verify agency state (mock).",
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
            "# Key vault operations\n# HomomorphicKeyManager: keys[8], KeyRotationPolicy\n# Operations: generate, rotate, revoke, list\n# Key types: BFV HE keys, Ed25519 signing keys\n\n# Query: list active keys?\n# Query: rotate key at index 0?\n# Query: generate new BFV key?",
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
            "Click \"Manage Keys\" to view vault state (mock).",
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
            "# Policy evaluation context\n# wellfair_evaluate_policy: subject, resource, action, sensitivity, epistemic_status\n# Sensitivity levels: public, internal, confidential, restricted, top_secret\n# Epistemic status: active, uncertain, skipped\n\n# Query: evaluate access for subject X to resource Y?\npolicy(subject=\"did:qualia:alice\", resource=\"doc:42\", action=\"read\", sensitivity=\"confidential\").",
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
            "Click \"Evaluate Policy\" to check access (mock).",
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
            "# Consent management context\n# wellfair_grant_consent: scope, subject, recipient, duration\n# wellfair_revoke_consent: consent_id\n# wellfair_list_consents: subject\n\n# Consent states: pending, granted, denied, revoked, expired",
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
            "Click \"Execute\" to manage consent (mock).",
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
            "# Carrier operations\n# extract_payload: carrier_quin \u{2192} payload bytes\n# media_tag: media_type, codec, duration, sample_rate\n# verify_binding: carrier_hash, payload_hash, signature\n\n# Query: extract payload from carrier X?\n# Query: verify binding integrity?",
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
            "Click \"Process Carrier\" to inspect media binding (mock).",
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
            "# Control feedback context\n# ControlState: setpoint, measured, error, integral, derivative\n# Bits: CONTROL_BIT, FEEDBACK_BIT, STABILIZATION_BIT\n# Operations: PID compute, state update, stabilization check\n\n# Query: compute PID output for setpoint=100, measured=95?\n# Query: check stabilization?",
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
            "Click \"Compute\" to evaluate control feedback (mock).",
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
            "# Likeliness context\n# Likeliness struct: value, confidence, evidence_count\n# Operations: combine, compare, rank\n# Used for: hypothesis ranking, evidence weighting\n\n# Query: compute likeliness of hypothesis H given evidence E1, E2, E3?\n# Query: rank hypotheses by likeliness?",
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
            "Click \"Evaluate\" to compute likeliness (mock).",
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
            "# QUBO compilation context\n# Input: cost matrix Q (n x n), variable count n\n# Output: compiled QUBO program for annealing\n# Used for: combinatorial optimization, scheduling, clustering\n\n# Query: compile QUBO for 10-variable problem?\n# Query: decompose into sub-QUBOs?",
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
            "Click \"Compile\" to build QUBO program (mock).",
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
            "# OWL conversion context\n# shacl_convert: OWL ontology \u{2192} SHACL shapes\n# materialize: OWL ontology \u{2192} inferred triples\n# classify: OWL DL classification (SROIQ)\n# consistency: OWL consistency checking\n\n# Input: OWL/Turtle ontology document\n\n@prefix ex: <http://example.org/> .\nex:Person a owl:Class ; rdfs:subClassOf ex:Agent .\nex:Student a owl:Class ; rdfs:subClassOf ex:Person .",
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
            "Click \"Convert\" to process OWL ontology (mock).",
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
