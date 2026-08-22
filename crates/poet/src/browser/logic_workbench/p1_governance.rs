//! P1 governance panels: value flow, interaction governance, identity fabric,
//! capability gap, legal compose, deontic compose.

use super::helpers::{
    make_button, make_results_area, make_section_label, make_select, make_text_input,
    make_textarea, make_tool_panel, show_logic_notification, show_mock_results,
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

pub(super) fn build_value_flow_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "value_flow", false);
    panel
        .append_child(&make_section_label(
            document,
            "Value Flow / Commons \u{2014} commons cost, royalty, outstanding obligations, pool state",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "value-flow-editor",
            "# Value flow context\ncontribution(alice, 100h, projectX).\ncontribution(bob, 50h, projectX).\ncommons_cost(projectX, 200h).\n\n# Query: outstanding obligations for alice?\n# Query: pool after alice contribution?",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "value-flow-evaluate",
            "\u{1F4B0} Compute Flow",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "value-flow-royalty",
            "\u{1F4B3} Royalty Split",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "value-flow-results",
            "Click \"Compute Flow\" to trace value contributions (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_value_flow_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("value-flow-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "value-flow-results", "value-flow");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("value-flow-royalty") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_logic_notification(&doc, "Royalty split: alice=66.7%, bob=33.3% (mock)");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_interaction_gov_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "interaction_gov", false);
    panel
        .append_child(&make_section_label(
            document,
            "Interaction Governance \u{2014} policy mode, governance verdict, permitted/forbidden interactions",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "interaction-gov-mode",
        &[
            ("permit", "Permit"),
            ("forbid", "Forbid"),
            ("obligate", "Obligate"),
            ("waive", "Waive"),
        ],
    ))
    .unwrap();
    row.append_child(&make_text_input(
        document,
        "interaction-gov-agent",
        "Agent DID",
    ))
    .unwrap();
    row.append_child(&make_text_input(
        document,
        "interaction-gov-action",
        "Action",
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "interaction-gov-context",
            "# Governance policies\npolicy(projectX, permit, contribute).\npolicy(projectX, forbid, deleteWithoutReview).\npolicy(projectX, obligate, signOffBeforeMerge).\n\n# Query: govern verdict for alice contributing to projectX?",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "interaction-gov-evaluate",
            "\u{1F6E1} Govern Verdict",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "interaction-gov-results",
            "Click \"Govern Verdict\" to evaluate policy (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_interaction_gov_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("interaction-gov-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "interaction-gov-results", "interaction-governance");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_identity_fabric_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "identity_fabric", false);
    panel
        .append_child(&make_section_label(
            document,
            "Identity Fabric \u{2014} identity survival, anchor recompute, identifier \u{2260} identity",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "identity-fabric-editor",
            "# Identity fabric\nanchor(alice, did:qualia:timothy_charles_holborn).\nanchor(alice, publicKey:ed25519:abc123).\nattest(bob, alice, \"isContributor\").\n\n# Query: surviving anchors after key rotation?\n# Query: does identity survive loss of publicKey:ed25519:abc123?",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "identity-fabric-evaluate",
            "\u{1F575} Recompute Fabric",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "identity-fabric-survive",
            "\u{1F501} Check Survival",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "identity-fabric-results",
            "Click \"Recompute Fabric\" to trace identity anchors (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_identity_fabric_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("identity-fabric-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "identity-fabric-results", "identity-fabric");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("identity-fabric-survive") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_logic_notification(&doc, "Identity survives: 2 surviving anchors (mock)");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_capability_gap_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "capability_gap", false);
    panel
        .append_child(&make_section_label(
            document,
            "Capability Gap Analyzer \u{2014} required vs available capabilities",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "capability-gap-editor",
            "# Capability requirements\nrequires(projectX, [rust, wasm, sparql]).\nhas(alice, [rust, wasm]).\nhas(bob, [sparql, rust, wasm]).\n\n# Query: capability gap for projectX with team {alice}?\n# Query: requirements met for projectX with team {alice, bob}?",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "capability-gap-evaluate",
            "\u{1F4CA} Analyze Gap",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "capability-gap-results",
            "Click \"Analyze Gap\" to find missing capabilities (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_capability_gap_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("capability-gap-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "capability-gap-results", "capability-gap");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_legal_compose_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "legal_compose", false);
    panel
        .append_child(&make_section_label(
            document,
            "Legal Compose \u{2014} selective disclosure, ZK eligibility, cross-jurisdictional compliance",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "legal-compose-editor",
            "# Legal compose context\njurisdiction(alice, au).\njurisdiction(bob, eu).\nrule(au, dataRetention, 7years).\nrule(eu, gdpr, rightToErasure).\n\n# Query: selective disclosure for alice in au?\n# Query: ZK eligibility for cross-jurisdictional transfer?",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "legal-compose-evaluate",
            "\u{1F510} Compose Legal",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "legal-compose-zk",
            "\u{1F50F} ZK Eligibility",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "legal-compose-results",
            "Click \"Compose Legal\" to evaluate cross-jurisdictional rules (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_legal_compose_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("legal-compose-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "legal-compose-results", "legal-compose");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("legal-compose-zk") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_logic_notification(&doc, "ZK eligibility: proof generated, eligible (mock)");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_deontic_compose_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "deontic_compose", false);
    panel
        .append_child(&make_section_label(
            document,
            "Deontic Compose \u{2014} mens rea classification, obligation lifecycle, multi-stakeholder norms",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "deontic-compose-editor",
            "# Deontic compose context\nobligation(alice, payTax, active).\nobligation(alice, deliverReport, discharged).\nknows(alice, deadlinePassed).\n\n# Query: classify mens rea for alice?\n# Query: obligation applies in jurisdiction au?\n# Query: obligation globally?",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "deontic-compose-evaluate",
            "\u{2696} Compose Norms",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "deontic-compose-mens",
            "\u{1F9E0} Mens Rea",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "deontic-compose-results",
            "Click \"Compose Norms\" to evaluate multi-stakeholder obligations (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_deontic_compose_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("deontic-compose-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "deontic-compose-results", "deontic-compose");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("deontic-compose-mens") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_logic_notification(&doc, "Mens rea: knowingly (mock)");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
