//! Logic workbench \u{2014} a modal overlay exposing the 42+ logic modalities
//! from the QualiaDB engine as interactive UI surfaces.
//!
//! P0 reasoning tools (core governance):
//! - Deontic rule editor, N3 Logic Studio, SHACL Validator
//! - RDF-Star Editor, Ontology Builder, Evaluate Modality, Symbolic Infer
//!
//! P1 legal/governance tools:
//! - Jural Relations, STIT Agency, Causal Liability, Responsibility
//! - Capacity, Delegation, Contract, Consensus, Meta-Deontic
//! - Value Flow, Interaction Governance, Identity Fabric, Capability Gap
//! - Legal Compose, Deontic Compose, Argumentation
//!
//! P1 formal/advanced logic tools:
//! - Epistemic, Paraconsistent, LTL, CTL, ASP, Defeasible
//! - Linear, Description Logic, Dialectical
//! - Abductive, Fuzzy, Probabilistic, Graph Theory, Interval
//! - Manifold 10D, Epistemic Boundaries, Modal
//!
//! All evaluation is structural/mock \u{2014} actual logic execution requires the
//! QualiaDB daemon backend (MCP evaluate_modality, evaluate_logic_rules,
//! symbolic_logic_infer tools; Tauri evaluate_logic_rules command).
//!
//! Source: `consult/20260818_logic-modalities-audit.md`
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

mod descriptions;
mod dispatch;
mod helpers;
mod p0_core;
mod p0_ext;
mod p1_advanced;
mod p1_governance;
mod p1_legal;
mod p1_logic;
mod p2_domain;
mod p2_extras;
mod p2_infra;
mod p2_infra_ext;
#[cfg(test)]
mod tests;

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, KeyboardEvent, MouseEvent};

// ---------------------------------------------------------------------------
// Modality catalog
// ---------------------------------------------------------------------------

const P0_TOOLS: &[(&str, &str, &str)] = &[
    ("deontic", "Deontic Editor", "\u{2696}"),
    ("n3", "N3 Logic Studio", "\u{1F9E9}"),
    ("shacl", "SHACL Validator", "\u{2705}"),
    ("rdfstar", "RDF-Star Editor", "\u{2B50}"),
    ("ontology", "Ontology Builder", "\u{1F4D6}"),
    ("modality", "Evaluate Modality", "\u{1F9E0}"),
    ("infer", "Symbolic Infer", "\u{1F50E}"),
    ("jural", "Jural Relations", "\u{2696}"),
    ("argumentation", "Argumentation", "\u{1F4AC}"),
];

const P1_LEGAL_TOOLS: &[(&str, &str, &str)] = &[
    ("stit", "STIT Agency", "\u{1F3AF}"),
    ("causal", "Causal Liability", "\u{1F539}"),
    ("responsibility", "Responsibility", "\u{2696}"),
    ("capacity", "Capacity", "\u{1F4CB}"),
    ("delegation", "Delegation", "\u{1F517}"),
    ("contract", "Contract Formation", "\u{1F4DD}"),
    ("consensus", "Consensus", "\u{2705}"),
    ("meta_deontic", "Meta-Deontic", "\u{1F4DC}"),
];

const P1_GOV_TOOLS: &[(&str, &str, &str)] = &[
    ("value_flow", "Value Flow", "\u{1F4B0}"),
    ("interaction_gov", "Interaction Gov", "\u{1F6E1}"),
    ("identity_fabric", "Identity Fabric", "\u{1F575}"),
    ("capability_gap", "Capability Gap", "\u{1F4CA}"),
    ("legal_compose", "Legal Compose", "\u{1F510}"),
    ("deontic_compose", "Deontic Compose", "\u{2696}"),
];

const P1_LOGIC_TOOLS: &[(&str, &str, &str)] = &[
    ("epistemic", "Epistemic", "\u{1F9E0}"),
    ("paraconsistent", "Paraconsistent", "\u{26A1}"),
    ("ltl", "LTL", "\u{23F1}"),
    ("ctl", "CTL", "\u{1F534}"),
    ("asp", "ASP", "\u{1F4A1}"),
    ("defeasible", "Defeasible", "\u{2696}"),
    ("linear", "Linear", "\u{1F4B8}"),
    ("description", "Description Logic", "\u{1F50D}"),
    ("dialectical", "Dialectical", "\u{1F9ED}"),
];

const P1_ADVANCED_TOOLS: &[(&str, &str, &str)] = &[
    ("abductive", "Abductive", "\u{1F50D}"),
    ("fuzzy", "Fuzzy", "\u{1F4A0}"),
    ("probabilistic", "Probabilistic", "\u{1F4B9}"),
    ("graph_theory", "Graph Theory", "\u{1F5C2}"),
    ("interval", "Interval", "\u{23F1}"),
    ("manifold_10d", "Manifold 10D", "\u{1F300}"),
    ("epistemic_boundaries", "Epist. Boundaries", "\u{1F6E1}"),
    ("modal", "Modal", "\u{25FB}"),
];

const P2_DOMAIN_TOOLS: &[(&str, &str, &str)] = &[
    ("clinical_risk", "Clinical Risk", "\u{1F49A}"),
    ("dicom_viewer", "DICOM Viewer", "\u{1F5BC}"),
    ("comorbidity", "Comorbidity", "\u{1F9EA}"),
    ("chemistry", "Chemistry", "\u{1F9EA}"),
    ("physics", "Physics", "\u{269B}"),
    ("ode_solver", "ODE Solver", "\u{1F501}"),
    ("bioinformatics", "Bioinformatics", "\u{1F9EC}"),
    ("gbm_var", "GBM / VaR", "\u{1F4B9}"),
    ("diffusion", "Diffusion", "\u{1F300}"),
];

const P2_INFRA_TOOLS: &[(&str, &str, &str)] = &[
    ("bytecode_vm", "Bytecode/VM", "\u{1F50D}"),
    ("slg_arena", "SLG Arena", "\u{1F4CB}"),
    ("forge_compute", "Forge Compute", "\u{1F525}"),
    ("compute_profile", "Compute Profile", "\u{1F4BB}"),
    ("privacy", "Privacy/HE/DP", "\u{1F510}"),
    ("model_lifecycle", "Model Lifecycle", "\u{1F4E6}"),
    ("inference_monitor", "Inference Monitor", "\u{1F4CA}"),
    ("gguf_tokenizer", "GGUF Tokenizer", "\u{1F9F8}"),
    ("p64_weight", "P64 Weight", "\u{1F4E6}"),
];

const P2_INFRA_EXT_TOOLS: &[(&str, &str, &str)] = &[
    ("crdt_sync", "CRDT/Sync", "\u{1F501}"),
    ("agency_merkle", "Agency/Merkle", "\u{1F511}"),
    ("key_vault", "Key Vault", "\u{1F510}"),
    ("policy_evaluator", "Policy Evaluator", "\u{1F6E1}"),
    ("consent_manager", "Consent Manager", "\u{2705}"),
    ("carrier", "Carrier/Media", "\u{1F4E6}"),
    ("control_feedback", "Control Feedback", "\u{1F501}"),
    ("likeliness", "Likeliness", "\u{1F4CF}"),
    ("qubo", "QUBO", "\u{1F9EE}"),
    ("owl_converter", "OWL Converter", "\u{1F527}"),
];

const P2_EXTRAS_TOOLS: &[(&str, &str, &str)] = &[
    ("allen_rcc8", "Allen/RCC8", "\u{1F4D0}"),
    ("manifold_logic", "Manifold Logic", "\u{1F300}"),
    ("calculus", "Calculus", "\u{1F9EE}"),
];

const ALL_MODALITIES: &[(&str, &str)] = &[
    ("deontic", "Deontic Logic"),
    ("epistemic", "Epistemic Logic"),
    ("paraconsistent", "Paraconsistent Logic"),
    ("ltl", "Linear Temporal Logic (LTL)"),
    ("ctl", "Computation Tree Logic (CTL)"),
    ("asp", "Answer Set Programming (ASP)"),
    ("defeasible", "Defeasible Logic"),
    ("linear", "Linear Logic"),
    ("description", "Description Logic (DL)"),
    ("dialectical", "Dialectical Logic"),
    ("argumentation", "Argumentation Framework"),
    ("abductive", "Abductive Reasoning"),
    ("fuzzy", "Fuzzy Logic"),
    ("probabilistic", "Probabilistic Reasoning"),
    ("graph_theory", "Graph Theory"),
    ("interval", "Interval Logic"),
    ("manifold_10d", "Manifold 10D Logic"),
    ("epistemic_boundaries", "Epistemic Boundaries"),
    ("modal", "Modal Logic"),
    ("jural", "Jural Relations (Hohfeld)"),
    ("stit", "STIT Agency"),
    ("causal", "Causal Liability"),
    ("responsibility", "Responsibility / Meta-Guard"),
    ("capacity", "Capacity Evaluator"),
    ("delegation", "Delegation Tracker"),
    ("contract", "Contract Formation"),
    ("consensus", "Consensus / Partition"),
    ("meta_deontic", "Meta-Deontic Breach"),
    ("value_flow", "Value Flow / Commons"),
    ("interaction_gov", "Interaction Governance"),
    ("identity_fabric", "Identity Fabric"),
    ("capability_gap", "Capability Gap Analyzer"),
    ("legal_compose", "Legal Compose"),
    ("deontic_compose", "Deontic Compose"),
    ("clinical_risk", "Clinical Risk Scorer"),
    ("dicom_viewer", "DICOM Viewer"),
    ("comorbidity", "Comorbidity Analyzer"),
    ("chemistry", "Chemistry Modeler"),
    ("physics", "Physics Simulator"),
    ("ode_solver", "ODE Solver"),
    ("bioinformatics", "Bioinformatics Lab"),
    ("gbm_var", "GBM / VaR Simulator"),
    ("diffusion", "Diffusion Controller"),
    ("bytecode_vm", "Bytecode / VM Inspector"),
    ("slg_arena", "SLG Arena Inspector"),
    ("forge_compute", "Forge Compute Probe"),
    ("compute_profile", "Compute Profile"),
    ("privacy", "Privacy / HE / DP"),
    ("model_lifecycle", "Model Lifecycle"),
    ("inference_monitor", "Inference Monitor"),
    ("gguf_tokenizer", "GGUF Tokenizer Inspector"),
    ("p64_weight", "P64 Weight Inspector"),
    ("crdt_sync", "CRDT / Sync Dashboard"),
    ("agency_merkle", "Agency / Merkle Inspector"),
    ("key_vault", "Key Vault Manager"),
    ("policy_evaluator", "Policy Evaluator"),
    ("consent_manager", "Consent Manager"),
    ("carrier", "Carrier / Media Binding"),
    ("control_feedback", "Control Feedback"),
    ("likeliness", "Likeliness"),
    ("qubo", "QUBO Compiler"),
    ("owl_converter", "OWL Converter"),
    ("allen_rcc8", "Allen Interval + RCC8"),
    ("manifold_logic", "Manifold Logic"),
    ("calculus", "Calculus"),
];

const DEONTIC_OPERATORS: &[(&str, &str)] = &[
    ("OBLIGATE", "OBLIGATE (\u{25CF}) \u{2014} must do"),
    ("PERMIT", "PERMIT (\u{25CB}) \u{2014} may do"),
    ("FORBID", "FORBID (\u{26D4}) \u{2014} must not do"),
    (
        "WAIVE",
        "WAIVE (\u{2300}) \u{2014} released from obligation",
    ),
];

const SHACL_CONSTRAINTS: &[(&str, &str)] = &[
    ("sh:minCount", "minCount"),
    ("sh:maxCount", "maxCount"),
    ("sh:minLength", "minLength"),
    ("sh:maxLength", "maxLength"),
    ("sh:pattern", "pattern (regex)"),
    ("sh:nodeKind", "nodeKind"),
    ("sh:datatype", "datatype"),
    ("sh:class", "class"),
    ("sh:in", "in (enumeration)"),
    ("sh:hasValue", "hasValue"),
    ("sh:lessThan", "lessThan"),
    ("sh:lessThanOrEquals", "lessThanOrEquals"),
    ("sh:qualifiedValueShape", "qualifiedValueShape"),
];

const RDFSTAR_ROLES: &[(&str, &str)] = &[
    ("subject", "Subject"),
    ("predicate", "Predicate"),
    ("object", "Object"),
    ("quoted-subject", "Quoted Subject (RDF-Star)"),
    ("quoted-object", "Quoted Object (RDF-Star)"),
];

// ---------------------------------------------------------------------------
// Build the logic workbench overlay
// ---------------------------------------------------------------------------

pub fn build_logic_workbench(document: &Document) -> Element {
    let overlay = document.create_element("div").unwrap();
    overlay.set_id("logic-workbench");
    let ov_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    ov_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.7); z-index: 10001; display: none; \
         align-items: flex-start; justify-content: center; padding-top: 40px;",
    );

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("logic-workbench-panel");
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "width: 920px; max-height: 720px; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px); \
         box-shadow: 0 12px 48px rgba(0,0,0,0.5); overflow: hidden; \
         display: flex; flex-direction: column;",
    );

    // Header
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 10px 16px; border-bottom: 1px solid var(--border-subtle);",
    );

    let title = document.create_element("span").unwrap();
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 13px; font-weight: 700; color: var(--accent-violet); \
         text-transform: uppercase; letter-spacing: 0.5px; font-family: var(--font-mono);",
    );
    title.set_text_content(Some("\u{1F9E0} Logic Workbench"));
    header.append_child(&title).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));
    let cb_el: HtmlElement = close_btn.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "background: transparent; border: none; color: var(--text-muted); \
         cursor: pointer; font-size: 16px; padding: 4px;",
    );
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    // Tool tabs (scrollable horizontal)
    let tabs = document.create_element("div").unwrap();
    tabs.set_class_name("logic-tool-tabs");
    let tabs_el: HtmlElement = tabs.clone().dyn_into().unwrap();
    tabs_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto; scrollbar-width: thin;",
    );

    let all_tools: Vec<(&str, &str, &str)> = P0_TOOLS
        .iter()
        .chain(P1_LEGAL_TOOLS.iter())
        .chain(P1_GOV_TOOLS.iter())
        .chain(P1_LOGIC_TOOLS.iter())
        .chain(P1_ADVANCED_TOOLS.iter())
        .chain(P2_DOMAIN_TOOLS.iter())
        .chain(P2_INFRA_TOOLS.iter())
        .chain(P2_INFRA_EXT_TOOLS.iter())
        .chain(P2_EXTRAS_TOOLS.iter())
        .copied()
        .collect();

    for (i, (tool_id, label, icon)) in all_tools.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_class_name("logic-tool-tab");
        tab.set_attribute("data-tool", tool_id).unwrap();
        if i == 0 {
            tab.class_list().add_1("active").unwrap();
        }
        let tab_el: HtmlElement = tab.clone().dyn_into().unwrap();
        tab_el.style().set_css_text(&format!(
            "padding: 8px 14px; background: transparent; border: none; \
             border-bottom: 2px solid {}; \
             color: {}; font-size: 11px; font-family: var(--font-mono); \
             cursor: pointer; display: flex; align-items: center; gap: 6px; \
             white-space: nowrap; transition: var(--trans-fast);",
            if i == 0 {
                "var(--accent-violet)"
            } else {
                "transparent"
            },
            if i == 0 {
                "var(--text-primary)"
            } else {
                "var(--text-muted)"
            },
        ));
        tab.set_text_content(Some(&format!("{} {}", icon, label)));
        tabs.append_child(&tab).unwrap();
    }
    panel.append_child(&tabs).unwrap();

    // Content area
    let content = document.create_element("div").unwrap();
    content.set_id("logic-workbench-content");
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style().set_css_text(
        "flex: 1; overflow-y: auto; padding: 16px; display: flex; flex-direction: column; gap: 12px;",
    );

    // P0 panels
    content
        .append_child(&p0_core::build_deontic_panel(document))
        .unwrap();
    content
        .append_child(&p0_core::build_n3_panel(document))
        .unwrap();
    content
        .append_child(&p0_core::build_shacl_panel(document))
        .unwrap();
    content
        .append_child(&p0_ext::build_rdfstar_panel(document))
        .unwrap();
    content
        .append_child(&p0_ext::build_ontology_builder_panel(document))
        .unwrap();
    content
        .append_child(&p0_ext::build_modality_panel(document))
        .unwrap();
    content
        .append_child(&p0_ext::build_infer_panel(document))
        .unwrap();

    // P1 legal panels
    content
        .append_child(&p1_legal::build_jural_panel(document))
        .unwrap();
    content
        .append_child(&p1_legal::build_stit_panel(document))
        .unwrap();
    content
        .append_child(&p1_legal::build_causal_panel(document))
        .unwrap();
    content
        .append_child(&p1_legal::build_responsibility_panel(document))
        .unwrap();
    content
        .append_child(&p1_legal::build_capacity_panel(document))
        .unwrap();
    content
        .append_child(&p1_legal::build_delegation_panel(document))
        .unwrap();
    content
        .append_child(&p1_legal::build_contract_panel(document))
        .unwrap();
    content
        .append_child(&p1_legal::build_consensus_panel(document))
        .unwrap();
    content
        .append_child(&p1_legal::build_meta_deontic_panel(document))
        .unwrap();
    content
        .append_child(&p1_legal::build_argumentation_panel(document))
        .unwrap();

    // P1 governance panels
    content
        .append_child(&p1_governance::build_value_flow_panel(document))
        .unwrap();
    content
        .append_child(&p1_governance::build_interaction_gov_panel(document))
        .unwrap();
    content
        .append_child(&p1_governance::build_identity_fabric_panel(document))
        .unwrap();
    content
        .append_child(&p1_governance::build_capability_gap_panel(document))
        .unwrap();
    content
        .append_child(&p1_governance::build_legal_compose_panel(document))
        .unwrap();
    content
        .append_child(&p1_governance::build_deontic_compose_panel(document))
        .unwrap();

    // P1 logic panels
    content
        .append_child(&p1_logic::build_epistemic_panel(document))
        .unwrap();
    content
        .append_child(&p1_logic::build_paraconsistent_panel(document))
        .unwrap();
    content
        .append_child(&p1_logic::build_ltl_panel(document))
        .unwrap();
    content
        .append_child(&p1_logic::build_ctl_panel(document))
        .unwrap();
    content
        .append_child(&p1_logic::build_asp_panel(document))
        .unwrap();
    content
        .append_child(&p1_logic::build_defeasible_panel(document))
        .unwrap();
    content
        .append_child(&p1_logic::build_linear_panel(document))
        .unwrap();
    content
        .append_child(&p1_logic::build_description_panel(document))
        .unwrap();
    content
        .append_child(&p1_logic::build_dialectical_panel(document))
        .unwrap();

    // P1 advanced panels
    content
        .append_child(&p1_advanced::build_abductive_panel(document))
        .unwrap();
    content
        .append_child(&p1_advanced::build_fuzzy_panel(document))
        .unwrap();
    content
        .append_child(&p1_advanced::build_probabilistic_panel(document))
        .unwrap();
    content
        .append_child(&p1_advanced::build_graph_theory_panel(document))
        .unwrap();
    content
        .append_child(&p1_advanced::build_interval_panel(document))
        .unwrap();
    content
        .append_child(&p1_advanced::build_manifold_10d_panel(document))
        .unwrap();
    content
        .append_child(&p1_advanced::build_epistemic_boundaries_panel(document))
        .unwrap();
    content
        .append_child(&p1_advanced::build_modal_panel(document))
        .unwrap();

    // P2 domain computational panels
    p2_domain::append_panels(document, &content);

    // P2 infrastructure panels
    p2_infra::append_panels(document, &content);
    p2_infra_ext::append_panels(document, &content);

    // P2 extra modality panels
    p2_extras::append_panels(document, &content);

    // Show only the first panel
    show_tool_panel(document, "deontic");

    panel.append_child(&content).unwrap();

    // Honesty footer
    let footer = document.create_element("div").unwrap();
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "padding: 6px 16px; border-top: 1px solid var(--border-subtle); \
         font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
    );
    footer.set_text_content(Some(
        "\u{1F4A1} Rule/norm/shape construction is live. \
         Logic evaluation requires QualiaDB daemon (MCP evaluate_modality, evaluate_logic_rules, \
         symbolic_logic_infer) \u{2014} results are structural mocks.",
    ));
    panel.append_child(&footer).unwrap();

    overlay.append_child(&panel).unwrap();

    // Wire close button
    let ov_clone = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let ov: HtmlElement = ov_clone.clone().dyn_into().unwrap();
        ov.style().set_property("display", "none").unwrap();
    }) as Box<dyn FnMut(MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();

    // Wire tool tabs
    wire_tool_tabs(document);

    // Wire all panels
    p0_core::wire_deontic_editor(document);
    p0_core::wire_n3_editor(document);
    p0_core::wire_shacl_validator(document);
    p0_ext::wire_rdfstar_editor(document);
    p0_ext::wire_ontology_builder(document);
    p0_ext::wire_modality_panel(document);
    p0_ext::wire_infer_panel(document);
    p1_legal::wire_jural_panel(document);
    p1_legal::wire_stit_panel(document);
    p1_legal::wire_causal_panel(document);
    p1_legal::wire_responsibility_panel(document);
    p1_legal::wire_capacity_panel(document);
    p1_legal::wire_delegation_panel(document);
    p1_legal::wire_contract_panel(document);
    p1_legal::wire_consensus_panel(document);
    p1_legal::wire_meta_deontic_panel(document);
    p1_legal::wire_argumentation_panel(document);
    p1_governance::wire_value_flow_panel(document);
    p1_governance::wire_interaction_gov_panel(document);
    p1_governance::wire_identity_fabric_panel(document);
    p1_governance::wire_capability_gap_panel(document);
    p1_governance::wire_legal_compose_panel(document);
    p1_governance::wire_deontic_compose_panel(document);
    p1_logic::wire_epistemic_panel(document);
    p1_logic::wire_paraconsistent_panel(document);
    p1_logic::wire_ltl_panel(document);
    p1_logic::wire_ctl_panel(document);
    p1_logic::wire_asp_panel(document);
    p1_logic::wire_defeasible_panel(document);
    p1_logic::wire_linear_panel(document);
    p1_logic::wire_description_panel(document);
    p1_logic::wire_dialectical_panel(document);
    p1_advanced::wire_abductive_panel(document);
    p1_advanced::wire_fuzzy_panel(document);
    p1_advanced::wire_probabilistic_panel(document);
    p1_advanced::wire_graph_theory_panel(document);
    p1_advanced::wire_interval_panel(document);
    p1_advanced::wire_manifold_10d_panel(document);
    p1_advanced::wire_epistemic_boundaries_panel(document);
    p1_advanced::wire_modal_panel(document);

    // P2 wiring
    p2_domain::wire_all(document);
    p2_infra::wire_all(document);
    p2_infra_ext::wire_all(document);
    p2_extras::wire_all(document);

    overlay
}

// ---------------------------------------------------------------------------
// Mode switching
// ---------------------------------------------------------------------------

fn show_tool_panel(document: &Document, tool: &str) {
    let panels = document.query_selector_all(".logic-tool-panel").unwrap();
    for i in 0..panels.length() {
        let p = panels.get(i).unwrap();
        let p_el: Element = p.dyn_into().unwrap();
        let p_tool = p_el.get_attribute("data-tool").unwrap_or_default();
        let html: HtmlElement = p_el.clone().dyn_into().unwrap();
        if p_tool == tool {
            html.style().set_property("display", "flex").unwrap();
        } else {
            html.style().set_property("display", "none").unwrap();
        }
    }
}

fn wire_tool_tabs(document: &Document) {
    let tabs = document.query_selector_all(".logic-tool-tab").unwrap();
    for i in 0..tabs.length() {
        let tab = tabs.get(i).unwrap();
        let tab_el: Element = tab.clone().dyn_into().unwrap();
        let tool = tab_el.get_attribute("data-tool").unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let all_tabs = doc.query_selector_all(".logic-tool-tab").unwrap();
            for j in 0..all_tabs.length() {
                let t = all_tabs.get(j).unwrap();
                let t_el: Element = t.dyn_into().unwrap();
                let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
                t_html
                    .style()
                    .set_property("border-bottom", "2px solid transparent")
                    .unwrap();
                t_html
                    .style()
                    .set_property("color", "var(--text-muted)")
                    .unwrap();
            }
            if let Ok(Some(clicked)) = doc.query_selector(&format!("[data-tool=\"{}\"]", tool)) {
                let c_el: HtmlElement = clicked.clone().dyn_into().unwrap();
                c_el.style()
                    .set_property("border-bottom", "2px solid var(--accent-violet)")
                    .unwrap();
                c_el.style()
                    .set_property("color", "var(--text-primary)")
                    .unwrap();
            }
            show_tool_panel(&doc, &tool);
        }) as Box<dyn FnMut(MouseEvent)>);
        tab_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn toggle_logic_workbench(document: &Document) {
    if let Some(wb) = document.get_element_by_id("logic-workbench") {
        let wb_el: HtmlElement = wb.dyn_into().unwrap();
        let display = wb_el
            .style()
            .get_property_value("display")
            .unwrap_or_default();
        if display == "none" {
            wb_el.style().set_property("display", "flex").unwrap();
        } else {
            wb_el.style().set_property("display", "none").unwrap();
        }
    }
}

pub fn open_to_tool(document: &Document, tool: &str) {
    if let Some(wb) = document.get_element_by_id("logic-workbench") {
        let wb_el: HtmlElement = wb.dyn_into().unwrap();
        wb_el.style().set_property("display", "flex").unwrap();
    }
    let tabs = document.query_selector_all(".logic-tool-tab").unwrap();
    for i in 0..tabs.length() {
        let t = tabs.get(i).unwrap();
        let t_el: Element = t.dyn_into().unwrap();
        let t_tool = t_el.get_attribute("data-tool").unwrap_or_default();
        if t_tool == tool {
            t_el.class_list().add_1("active").unwrap();
            let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
            t_html
                .style()
                .set_property("border-bottom", "2px solid var(--accent-violet)")
                .unwrap();
            t_html
                .style()
                .set_property("color", "var(--text-primary)")
                .unwrap();
        } else {
            t_el.class_list().remove_1("active").unwrap();
            let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
            t_html
                .style()
                .set_property("border-bottom", "2px solid transparent")
                .unwrap();
            t_html
                .style()
                .set_property("color", "var(--text-muted)")
                .unwrap();
        }
    }
    show_tool_panel(document, tool);
}

pub fn dispatch_command(document: &Document, label: &str) -> bool {
    dispatch::dispatch_command(document, label)
}

pub fn wire_logic_workbench_shortcut(document: &Document) {
    let closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        if e.key() == "L" && e.shift_key() && (e.ctrl_key() || e.meta_key()) {
            e.prevent_default();
            let doc = web_sys::window().unwrap().document().unwrap();
            toggle_logic_workbench(&doc);
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);

    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}
