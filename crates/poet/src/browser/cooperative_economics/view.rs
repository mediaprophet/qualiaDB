//! Cooperative Systems & Ontological Economics viewport.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::live_welfare::build_live_welfare_panel;
use super::model::{AccessVerdict, OntologicalPricingEngine, PeerOntologyClass, TrueCostModel};

const CARD_CSS: &str = "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
     border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;";

/// Build the Cooperative Systems & Ontological Economics Viewport.
pub(super) fn build_cooperative_economics_view(
    document: &Document,
    cost_model: &TrueCostModel,
) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;",
    );
    let _ = root.set_attribute("data-cooperative-economics", "true");

    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px;",
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some(
        "Socially Defined Networking & Ontological Economics",
    ));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let cost_hud = document.create_element("span").unwrap();
    cost_hud.set_text_content(Some(&format!(
        "Local true-cost: ${:.3}/hr · Net ${:.3}/GB · Power ${:.3}/hr",
        cost_model.hardware_cost_per_hour(),
        cost_model.network_cost_per_gb(),
        cost_model.power_cost_per_hour()
    )));
    let cost_hud_el: HtmlElement = cost_hud.clone().dyn_into().unwrap();
    cost_hud_el
        .style()
        .set_css_text("font-size: 11px; font-family: var(--font-mono); color: #34d399;");
    header.append_child(&cost_hud).unwrap();
    root.append_child(&header).unwrap();

    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 10px;",
    );

    grid.append_child(&build_lanes_card(document)).unwrap();
    grid.append_child(&build_pricing_card(document)).unwrap();
    grid.append_child(&build_true_cost_card(document, cost_model))
        .unwrap();
    grid.append_child(&build_live_welfare_panel(document))
        .unwrap();

    root.append_child(&grid).unwrap();
    root
}

fn build_lanes_card(document: &Document) -> Element {
    let card = document.create_element("div").unwrap();
    let card_el: HtmlElement = card.clone().dyn_into().unwrap();
    card_el.style().set_css_text(CARD_CSS);
    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("SDN Permissive Routing Lanes"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    card.append_child(&title).unwrap();
    let lanes_text = document.create_element("pre").unwrap();
    lanes_text.set_text_content(Some(
        "• Lane::Commons: Public Open-Access\n\
         • Lane::Bilateral: 1-on-1 Projects\n\
         • Lane::Federated: Swarm Compute\n\
         • Lane::Commercial: Metered Transit",
    ));
    let lanes_text_el: HtmlElement = lanes_text.clone().dyn_into().unwrap();
    lanes_text_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; color: #94a3b8; margin: 4px 0 0 0; \
         background: rgba(0,0,0,0.3); padding: 6px; border-radius: 4px;",
    );
    card.append_child(&lanes_text).unwrap();
    card
}

fn build_pricing_card(document: &Document) -> Element {
    let card = document.create_element("div").unwrap();
    let card_el: HtmlElement = card.clone().dyn_into().unwrap();
    card_el.style().set_css_text(CARD_CSS);
    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("Ontological Pricing Matrix"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    card.append_child(&title).unwrap();

    let matrix_text = document.create_element("pre").unwrap();
    matrix_text.set_text_content(Some(
        "• Person: 25GB Free Commons Quota\n\
         • ResearchLab: Reciprocal Barter\n\
         • Corporation: $0.15/GB + $0.05/GPU-s\n\
         • Anonymous: Gated Challenge",
    ));
    let matrix_text_el: HtmlElement = matrix_text.clone().dyn_into().unwrap();
    matrix_text_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; color: #94a3b8; margin: 4px 0 0 0; \
         background: rgba(0,0,0,0.3); padding: 6px; border-radius: 4px;",
    );
    card.append_child(&matrix_text).unwrap();

    let eval_row = document.create_element("div").unwrap();
    let eval_row_el: HtmlElement = eval_row.clone().dyn_into().unwrap();
    eval_row_el
        .style()
        .set_css_text("display: flex; gap: 6px; align-items: center; margin-top: 4px;");

    let peer_select = document.create_element("select").unwrap();
    let ps_el: HtmlElement = peer_select.clone().dyn_into().unwrap();
    ps_el.style().set_css_text(
        "flex: 1; font-family: var(--font-mono); font-size: 10px; background: rgba(0,0,0,0.4); \
         color: #cbd5e1; border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 2px 4px;",
    );
    for (val, label) in &[
        ("human", "Natural Person (Verified)"),
        ("unverified", "Natural Person (Unverified)"),
        ("lab", "Research Collective (Lab)"),
        ("corp", "Commercial Corporation"),
        ("anon", "Anonymous / Unverified"),
    ] {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", val).unwrap();
        opt.set_text_content(Some(label));
        peer_select.append_child(&opt).unwrap();
    }
    eval_row.append_child(&peer_select).unwrap();

    let eval_btn = document.create_element("button").unwrap();
    eval_btn.set_class_name("vibe-run-btn");
    eval_btn.set_text_content(Some("Evaluate"));
    let eb_el: HtmlElement = eval_btn.clone().dyn_into().unwrap();
    eb_el.style().set_css_text(
        "background: var(--accent-emerald, #00f2a9); color: #020617; font-weight: 700; \
         font-size: 10px; padding: 3px 8px; border-radius: 4px; border: none; cursor: pointer;",
    );
    eval_row.append_child(&eval_btn).unwrap();
    card.append_child(&eval_row).unwrap();

    let verdict_display = document.create_element("div").unwrap();
    let vd_el: HtmlElement = verdict_display.clone().dyn_into().unwrap();
    vd_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; color: var(--accent-emerald); \
         background: rgba(0, 242, 169, 0.08); padding: 4px 6px; border-radius: 4px; \
         border: 1px solid rgba(0, 242, 169, 0.2);",
    );
    verdict_display
        .set_text_content(Some("Verdict: PermitFree (25GB Commons Quota) — local policy"));
    card.append_child(&verdict_display).unwrap();

    let ps_clone = peer_select.clone();
    let vd_clone = verdict_display.clone();
    let eval_closure =
        wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            let select_el: web_sys::HtmlSelectElement = ps_clone.clone().dyn_into().unwrap();
            let val = select_el.value();
            let peer_class = match val.as_str() {
                "human" => PeerOntologyClass::NaturalPerson {
                    is_human_verified: true,
                },
                "unverified" => PeerOntologyClass::NaturalPerson {
                    is_human_verified: false,
                },
                "lab" => PeerOntologyClass::ResearchCollective {
                    lab_name: "OpenAnatomyLab".into(),
                },
                "corp" => PeerOntologyClass::Corporation {
                    company_name: "Acme Corp".into(),
                    tax_id: None,
                },
                _ => PeerOntologyClass::AnonymousOrUnverified,
            };
            let verdict = OntologicalPricingEngine::evaluate_peer(&peer_class);
            let text = match verdict {
                AccessVerdict::PermitFree {
                    free_bandwidth_gb,
                    reason,
                } => format!("PermitFree ({free_bandwidth_gb}GB Quota) — {reason}"),
                AccessVerdict::ReciprocalBarter {
                    allowed_storage_gb,
                    required_return,
                } => format!(
                    "ReciprocalBarter ({allowed_storage_gb}GB Storage) — Return: {required_return}"
                ),
                AccessVerdict::MeteredPayment {
                    rate_per_gb_cents,
                    rate_per_gpu_sec_cents,
                } => format!(
                    "MeteredPayment: ${:.2}/GB, ${:.2}/GPU-s",
                    rate_per_gb_cents as f64 / 100.0,
                    rate_per_gpu_sec_cents as f64 / 100.0
                ),
                AccessVerdict::Deny { reason } => format!("Deny: {reason}"),
            };
            vd_clone.set_text_content(Some(&text));
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    eval_btn
        .add_event_listener_with_callback("click", eval_closure.as_ref().unchecked_ref())
        .unwrap();
    eval_closure.forget();

    card
}

fn build_true_cost_card(document: &Document, cost_model: &TrueCostModel) -> Element {
    let card = document.create_element("div").unwrap();
    let card_el: HtmlElement = card.clone().dyn_into().unwrap();
    card_el.style().set_css_text(CARD_CSS);
    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("True-Cost Personal Unit Economics"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    card.append_child(&title).unwrap();

    let honesty = document.create_element("div").unwrap();
    honesty.set_text_content(Some(
        "Local model from your rates — not a live market quote and not an Econ.* invoke.",
    ));
    let honesty_el: HtmlElement = honesty.clone().dyn_into().unwrap();
    honesty_el
        .style()
        .set_css_text("font-size: 10px; color: #94a3b8;");
    card.append_child(&honesty).unwrap();

    let cost_breakdown = document.create_element("pre").unwrap();
    cost_breakdown.set_text_content(Some(&format!(
        "Hardware: ${:.4} / hr\n\
         Bandwidth: ${:.4} / GB\n\
         Electricity: ${:.4} / hr\n\
         Sample 1hr + 10GB Job: ${:.4} AUD",
        cost_model.hardware_cost_per_hour(),
        cost_model.network_cost_per_gb(),
        cost_model.power_cost_per_hour(),
        cost_model.total_job_cost(1.0, 10.0)
    )));
    let cost_breakdown_el: HtmlElement = cost_breakdown.clone().dyn_into().unwrap();
    cost_breakdown_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; color: #34d399; margin: 4px 0 0 0; \
         background: rgba(0,0,0,0.3); padding: 6px; border-radius: 4px;",
    );
    card.append_child(&cost_breakdown).unwrap();
    card
}
