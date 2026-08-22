//! Automation — VibeScript trigger rules: event, condition, action (§2.9.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[("rules", "Trigger Rules"), ("log", "Execution Log")];

const RULES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "RULE-001",
        "work_item.moved_to(InReview)",
        "reviewer.is_assigned",
        "notify_reviewer + assign_review",
        "active",
    ),
    (
        "RULE-002",
        "contribution.committed",
        "fair_value > 0",
        "mint_tokens(CRB, fair_value * 0.1)",
        "active",
    ),
    (
        "RULE-003",
        "bounty.completed",
        "evidence.is_verified",
        "payout_from_escrow + notify_sponsor",
        "active",
    ),
    (
        "RULE-004",
        "milestone.reached",
        "governance.approves",
        "mint_tokens(QLA, 5000) + announce",
        "active",
    ),
    (
        "RULE-005",
        "dispute.filed",
        "urgency == high",
        "notify_governance + escalate_to_mediation",
        "active",
    ),
    (
        "RULE-006",
        "agreement.signed",
        "threshold_met",
        "activate_agreement + compile_norms",
        "draft",
    ),
];

const LOG: &[(&str, &str, &str, &str, &str)] = &[
    (
        "RULE-001",
        "2026-08-18 10:32",
        "TASK-005 moved to InReview",
        "success",
        "notified did:qualia:contributor_02",
    ),
    (
        "RULE-002",
        "2026-08-18 09:15",
        "Contribution by did:qualia:timothy_charles_holborn",
        "success",
        "minted 600 CRB",
    ),
    (
        "RULE-003",
        "2026-08-17 14:30",
        "BNT-005 completed by did:qualia:contributor_02",
        "success",
        "paid 1,500 XEC from escrow",
    ),
    (
        "RULE-005",
        "2026-08-17 16:20",
        "DSP-001 filed with high urgency",
        "success",
        "notified governance body",
    ),
    (
        "RULE-002",
        "2026-08-17 14:30",
        "Contribution by did:qualia:contributor_03",
        "success",
        "minted 200 CRB",
    ),
    (
        "RULE-004",
        "2026-08-03 12:00",
        "Milestone: Ontology Spec v3 approved",
        "success",
        "minted 5,000 QLA + announced",
    ),
];

pub fn build_automation_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = build_tab_bar(document);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content.append_child(&build_rules_tab(document)).unwrap();

    for (i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_tab_panel(document, tab_id);
        if i > 0 {
            let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
            p_el.style().set_css_text("display: none;");
        }
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} automation requires VibeScript engine + IntentBus + capability gating.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_tab_bar(document: &Document) -> Element {
    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-automation-tab", tab_id).unwrap();
        tab.set_text_content(Some(tab_label));
        let t_el: HtmlElement = tab.clone().dyn_into().unwrap();
        t_el.style().set_css_text(&format!(
            "padding: 4px 10px; border: none; border-bottom: 2px solid {}; \
             background: transparent; color: {}; font-size: 10px; \
             font-family: var(--font-mono); cursor: pointer; white-space: nowrap;",
            if i == 0 {
                "var(--accent-cyan)"
            } else {
                "transparent"
            },
            if i == 0 {
                "var(--text-primary)"
            } else {
                "var(--text-muted)"
            },
        ));
        tab_bar.append_child(&tab).unwrap();
    }
    tab_bar
}

fn build_rules_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-automation-panel", "rules")
        .unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Trigger rules: event \u{2192} condition \u{2192} action. \
         Actions are gated by VibeScript capability resolution.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    for (id, event, condition, action, status) in RULES {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        let border = match *status {
            "active" => "rgba(100, 200, 100, 0.3)",
            "draft" => "rgba(255, 165, 0, 0.3)",
            _ => "var(--border-subtle)",
        };
        c_el.style().set_css_text(&format!(
            "border: 1px solid {}; border-radius: 4px; padding: 6px 8px; \
             margin-bottom: 4px; background: var(--surface-panel);",
            border,
        ));

        let hdr = document.create_element("div").unwrap();
        hdr.set_text_content(Some(&format!("{}  [{}]", id, status)));
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        let status_color = match *status {
            "active" => "rgba(100, 200, 100, 0.8)",
            "draft" => "rgba(255, 165, 0, 0.8)",
            _ => "var(--text-primary)",
        };
        h_el.style().set_css_text(&format!(
            "font-size: 10px; font-weight: 600; color: {}; \
             font-family: var(--font-mono); margin-bottom: 4px;",
            status_color,
        ));
        card.append_child(&hdr).unwrap();

        let flow = document.create_element("div").unwrap();
        flow.set_text_content(Some(&format!(
            "WHEN {}  IF {}  DO {}",
            event, condition, action,
        )));
        let f_el: HtmlElement = flow.clone().dyn_into().unwrap();
        f_el.style().set_css_text(
            "font-size: 9px; color: var(--text-secondary); \
             font-family: var(--font-mono);",
        );
        card.append_child(&flow).unwrap();

        panel.append_child(&card).unwrap();
    }

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ New Rule"));
    let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 6px;",
    );
    panel.append_child(&btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-automation-panel", tab_id)
        .unwrap();

    match tab_id {
        "log" => {
            let table = make_table(
                document,
                &["Rule", "Timestamp", "Trigger", "Result", "Detail"],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (rule, ts, trigger, result, detail) in LOG {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [rule, ts, trigger, result, detail].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 3 {
                        let color = match **val {
                            "success" => "rgba(100, 200, 100, 0.8)",
                            "failed" => "rgba(255, 100, 100, 0.8)",
                            _ => "var(--text-primary)",
                        };
                        td_el.style().set_css_text(&format!(
                            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                             color: {}; font-size: 10px; font-weight: 600;",
                            color,
                        ));
                    } else {
                        td_el.style().set_css_text(
                            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                             color: var(--text-primary); font-size: 10px; \
                             font-family: var(--font-mono);",
                        );
                    }
                    tr.append_child(&td).unwrap();
                }
                tbody.append_child(&tr).unwrap();
            }
            table.append_child(&tbody).unwrap();
            panel.append_child(&table).unwrap();
        }
        _ => {}
    }

    panel
}

fn make_table(document: &Document, headers: &[&str]) -> Element {
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 10px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 4px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
