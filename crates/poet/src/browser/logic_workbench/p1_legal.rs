//! P1 legal/governance panels: jural, STIT, causal, responsibility, capacity,
//! delegation, contract, consensus, meta-deontic, argumentation.

use super::helpers::{
    make_button, make_results_area, make_section_label, make_select, make_text_input,
    make_textarea, make_tool_panel, show_mock_results,
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlSelectElement, MouseEvent};

pub(super) fn build_jural_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "jural", false);

    panel
        .append_child(&make_section_label(
            document,
            "Jural Relations \u{2014} Hohfeldian positions (8 correlative/opposite pairs)",
        ))
        .unwrap();

    let hohfeld = [
        ("Right", "Duty", "No-Right", "Privilege"),
        ("Privilege", "No-Right", "Duty", "Right"),
        ("Power", "Liability", "Disability", "Immunity"),
        ("Immunity", "Disability", "Liability", "Power"),
    ];

    let table = document.create_element("div").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "display: grid; grid-template-columns: 1fr 1fr 1fr 1fr; gap: 4px; \
         font-family: var(--font-mono); font-size: 10px;",
    );

    for header in &[
        "Position",
        "Correlative",
        "Opposite",
        "Correlative of Opposite",
    ] {
        let h = document.create_element("div").unwrap();
        let h_el: HtmlElement = h.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "padding: 6px 8px; background: var(--surface-panel-elevated); \
             color: var(--accent-violet); font-weight: 700; text-align: center; \
             border-radius: var(--radius-xs);",
        );
        h.set_text_content(Some(header));
        table.append_child(&h).unwrap();
    }

    for (pos, corr, opp, corr_opp) in &hohfeld {
        for val in &[*pos, *corr, *opp, *corr_opp] {
            let cell = document.create_element("div").unwrap();
            let c_el: HtmlElement = cell.clone().dyn_into().unwrap();
            c_el.style().set_css_text(
                "padding: 6px 8px; background: var(--surface-panel); \
                 color: var(--text-secondary); text-align: center; \
                 border-radius: var(--radius-xs); border: 1px solid var(--border-subtle);",
            );
            cell.set_text_content(Some(val));
            table.append_child(&cell).unwrap();
        }
    }
    panel.append_child(&table).unwrap();

    panel
        .append_child(&make_section_label(
            document,
            "Select role to inspect jural positions:",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");
    row.append_child(&make_select(
        document,
        "jural-role",
        &[
            ("principal", "Principal"),
            ("agent", "Agent"),
            ("contributor", "Contributor"),
            ("reviewer", "Reviewer"),
            ("fiduciary", "Fiduciary"),
            ("custodian", "Custodian"),
        ],
    ))
    .unwrap();
    row.append_child(&make_button(
        document,
        "jural-analyze",
        "\u{2696} Analyze Positions",
        true,
    ))
    .unwrap();
    panel.append_child(&row).unwrap();

    panel
        .append_child(&make_results_area(
            document,
            "jural-results",
            "Scan the connected graph for the role's jural positions and unmet correlatives.",
        ))
        .unwrap();

    panel
}

pub(super) fn wire_jural_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("jural-analyze") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "jural-results", "jural");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_stit_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "stit", false);
    panel
        .append_child(&make_section_label(
            document,
            "STIT Agency \u{2014} \u{2018}\u{03B1} Sees To It That \u{03C6}\u{2019} agentive attribution",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_text_input(
        document,
        "stit-agent",
        "Agent DID (e.g. did:qualia:timothy_charles_holborn)",
    ))
    .unwrap();
    row.append_child(&make_text_input(
        document,
        "stit-action",
        "Action (e.g. deliverMilestone)",
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_section_label(
            document,
            "Context (obligations, duties, joint actions):",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "stit-context",
            "brought_about=true\ncould_do_otherwise=true\nmembers=[alice|bob]\njoint_acted=false",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "stit-evaluate",
            "\u{1F3AF} Evaluate Agency",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "stit-joint",
            "\u{1F91D} Joint Liability",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "stit-results",
            "Evaluate Chellas/deliberative agency and bounded joint liability.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_stit_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("stit-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "stit-results", "STIT");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("stit-joint") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "stit-results", "stit-joint");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_causal_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "causal", false);
    panel
        .append_child(&make_section_label(
            document,
            "Causal Liability \u{2014} but-for causation, overdetermination, dependency voidance",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "causal-editor",
            "target=missedDeadline\nedges=[alice:delay|delay:missedDeadline|bob:delay]",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "causal-evaluate",
            "\u{1F539} Trace Causation",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "causal-overdetermine",
            "\u{26A1} Check Overdetermination",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "causal-results",
            "Trace root causes and detect independent overdetermination in a bounded graph.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_causal_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("causal-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "causal-results", "causal");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("causal-overdetermine") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "causal-results", "causal-overdetermination");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_responsibility_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "responsibility", false);
    panel
        .append_child(&make_section_label(
            document,
            "Responsibility / Meta-Guard \u{2014} allegation \u{2192} adjudication, rule-of-law asymmetry",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "resp-editor",
            "confirmed=true\ndismissed=false\nharm_occurred=true\naccountable_person=false",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "resp-evaluate",
            "\u{2696} Adjudicate",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "resp-vacuum",
            "\u{1F9ED} Accountability Vacuum",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "resp-results",
            "Apply the allegation/adjudication gate and systemic accountability guard.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_responsibility_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("resp-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "resp-results", "responsibility");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("resp-vacuum") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "resp-results", "responsibility-vacuum");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_capacity_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "capacity", false);
    panel
        .append_child(&make_section_label(
            document,
            "Capacity Evaluator \u{2014} legal capacity, posthumous standing, stipulation binding",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_text_input(document, "capacity-agent", "Agent DID"))
        .unwrap();
    row.append_child(&make_select(
        document,
        "capacity-type",
        &[
            ("effective", "Effective Principal"),
            ("posthumous", "Posthumous Standing"),
            ("stipulation", "Stipulation Binding"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "capacity-context",
            "status=intact\ndependent=bob\nguardianship=false\ndeceased=false\nrepresentative=false\nimbalance=0.0\nexplicit_threat=false\nduress_threshold=0.7",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "capacity-evaluate",
            "\u{1F4CB} Evaluate Capacity",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "capacity-results",
            "Evaluate binding, duress/voidability, principal identity, and standing.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_capacity_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("capacity-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "capacity-results", "capacity");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_delegation_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "delegation", false);
    panel
        .append_child(&make_section_label(
            document,
            "Delegation Tracker \u{2014} chain validity, revocation, descendant scope",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "deleg-editor",
            "parent_domains=[projectX|reports]\nchild_domains=[projectX]\nrevoked_domains=[projectX]\nrequested_domain=projectX",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "deleg-evaluate",
            "\u{1F517} Trace Delegation",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "deleg-revoke",
            "\u{2702} Check Revocation",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "deleg-results",
            "Check attenuation and cascading domain revocation with the native capacity core.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_delegation_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("deleg-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "deleg-results", "delegation");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("deleg-revoke") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "deleg-results", "delegation-revocation");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_contract_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "contract", false);
    panel
        .append_child(&make_section_label(
            document,
            "Contract Formation \u{2014} offer, acceptance, consideration, incorporation by reference",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "contract-editor",
            "stipulated=true\naccepted=true\nofferor_capacity=intact\nacceptor_capacity=intact\ninstrument=standardTerms",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "contract-evaluate",
            "\u{1F4DD} Check Formation",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "contract-results",
            "Compute formation stage, capacity-gated binding, and incorporation by reference.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_contract_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("contract-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "contract-results", "contract");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_consensus_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "consensus", false);
    panel
        .append_child(&make_section_label(
            document,
            "Consensus / Partition \u{2014} M-of-N decisions, partition tolerance, transaction status",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "consensus-mode",
        &[
            ("global", "Global Validity"),
            ("partition", "Partition Tolerance"),
            ("joint", "Joint Formation"),
        ],
    ))
    .unwrap();
    row.append_child(&make_text_input(
        document,
        "consensus-threshold",
        "Threshold (e.g. 3-of-5)",
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "consensus-context",
            "partitioned=true",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "consensus-evaluate",
            "\u{2705} Evaluate Consensus",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "consensus-results",
            "Compute transaction status, BFT quorum, commit, and partition formation rules.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_consensus_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("consensus-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "consensus-results", "consensus");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_meta_deontic_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "meta_deontic", false);
    panel
        .append_child(&make_section_label(
            document,
            "Meta-Deontic Breach \u{2014} breach records, provenance, endorsement credentials",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "meta-deontic-editor",
            "actor=alice\naction=missedDeadline\ninstrument=witness_bob\nnow=1787865600",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "meta-deontic-evaluate",
            "\u{1F4DC} Build Breach Record",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "meta-deontic-endorse",
            "\u{1F4E8} Endorsement Credential",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "meta-deontic-results",
            "Build a provenance-anchored breach Quin; endorsement reports its signing requirement.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_meta_deontic_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("meta-deontic-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "meta-deontic-results", "meta-deontic");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("meta-deontic-endorse") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "meta-deontic-results", "meta-deontic-endorse");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_argumentation_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "argumentation", false);

    panel
        .append_child(&make_section_label(
            document,
            "Argumentation Framework \u{2014} Dung-style attack/defense graph",
        ))
        .unwrap();

    panel
        .append_child(&make_textarea(
            document,
            "arg-editor",
            "arguments=[a1|a2|a3]\nattacks=[a2:a1|a3:a2]",
            "160px",
        ))
        .unwrap();

    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");
    row.append_child(&{
        let lbl = document.create_element("span").unwrap();
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        lbl.set_text_content(Some("Semantics:"));
        lbl.into()
    })
    .unwrap();
    row.append_child(&make_select(
        document,
        "arg-semantics",
        &[
            ("grounded", "Grounded"),
            ("preferred", "Preferred"),
            ("stable", "Stable"),
            ("complete", "Complete"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();

    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "arg-evaluate",
            "\u{1F4AC} Compute Extension",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "arg-visualize",
            "\u{1F5FA} Graph Data",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();

    panel
        .append_child(&make_results_area(
            document,
            "arg-results",
            "Compute native Dung extensions and return bounded node/attack graph data.",
        ))
        .unwrap();

    panel
}

pub(super) fn wire_argumentation_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("arg-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let sem = doc
                .get_element_by_id("arg-semantics")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            show_mock_results(&doc, "arg-results", &format!("{}-extension", sem));
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("arg-visualize") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "arg-results", "argumentation-visualize");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
