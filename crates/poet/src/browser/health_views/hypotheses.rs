//! Hypotheses — diagnostic hypothesis list with evidence + disclosure tiers (§4, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("all", "All Hypotheses"),
    ("proposed", "Proposed"),
    ("supported", "Supported"),
    ("disputed", "Disputed"),
];

const HYPOTHESES: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "HYP-001",
        "Iron deficiency anaemia secondary to inadequate dietary intake",
        "engine",
        "Proposed",
        "0.72",
        "Self",
    ),
    (
        "HYP-002",
        "Vitamin D deficiency due to limited sun exposure",
        "engine",
        "Supported",
        "0.85",
        "Self",
    ),
    (
        "HYP-003",
        "Hypothyroidism adequately treated with current Levothyroxine dose",
        "clinician",
        "Supported",
        "0.90",
        "Self",
    ),
    (
        "HYP-004",
        "Seasonal affective pattern contributing to mood variability",
        "engine",
        "Proposed",
        "0.45",
        "Self",
    ),
    (
        "HYP-005",
        "Ferrous sulfate causing GI intolerance reducing adherence",
        "engine",
        "Disputed",
        "0.30",
        "Self",
    ),
];

const EVIDENCE: &[(&str, &str, &str, &str)] = &[
    (
        "HYP-001",
        "supporting",
        "Ferritin = 12 \u{00B5}g/L (low, ref 30-400)",
        "Lab Result: 2026-08-15",
    ),
    (
        "HYP-001",
        "supporting",
        "Iron = 8 \u{00B5}mol/L (low, ref 10-30)",
        "Lab Result: 2026-08-15",
    ),
    (
        "HYP-001",
        "contradicting",
        "Haemoglobin = 142 g/L (normal)",
        "Lab Result: 2026-08-15",
    ),
    (
        "HYP-002",
        "supporting",
        "25-OH Vitamin D = 28 nmol/L (low, ref 75-250)",
        "Lab Result: 2025-11-20",
    ),
    (
        "HYP-003",
        "supporting",
        "TSH = 2.1 mIU/L (normal, ref 0.4-4.0)",
        "Lab Result: 2026-06-01",
    ),
    (
        "HYP-003",
        "supporting",
        "Free T4 = 14 pmol/L (normal, ref 10-20)",
        "Lab Result: 2026-06-01",
    ),
];

pub fn build_hypotheses_view(document: &Document) -> Element {
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

    content
        .append_child(&build_hypotheses_tab(document, "all"))
        .unwrap();

    for (i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_hypotheses_tab(document, tab_id);
        let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
        p_el.style().set_css_text("display: none;");
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} hypotheses require DIAG-1..DIAG-6 HypothesisEngine + disclosure tier policy.",
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
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-hypotheses-tab", tab_id).unwrap();
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

fn build_hypotheses_tab(document: &Document, filter: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-hypotheses-panel", filter)
        .unwrap();

    for (id, text, generated_by, status, confidence, tier) in HYPOTHESES {
        let show = match filter {
            "all" => true,
            "proposed" => *status == "Proposed",
            "supported" => *status == "Supported",
            "disputed" => *status == "Disputed",
            _ => true,
        };
        if !show {
            continue;
        }

        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        let border = match *status {
            "Proposed" => "rgba(0, 200, 255, 0.3)",
            "Supported" => "rgba(100, 200, 100, 0.3)",
            "Disputed" => "rgba(255, 165, 0, 0.3)",
            "Rejected" => "var(--border-subtle)",
            _ => "var(--border-subtle)",
        };
        c_el.style().set_css_text(&format!(
            "border: 1px solid {}; border-radius: 6px; padding: 8px; \
             margin-bottom: 6px; background: var(--surface-panel);",
            border,
        ));

        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "display: flex; justify-content: space-between; align-items: center; \
             margin-bottom: 4px;",
        );

        let left = document.create_element("div").unwrap();
        left.set_text_content(Some(&format!("{} [{}]", id, generated_by)));
        let l_el: HtmlElement = left.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        hdr.append_child(&left).unwrap();

        let status_badge = document.create_element("span").unwrap();
        status_badge.set_text_content(Some(status));
        let sb_el: HtmlElement = status_badge.clone().dyn_into().unwrap();
        let status_color = match *status {
            "Proposed" => "rgba(0, 200, 255, 0.8)",
            "Supported" => "rgba(100, 200, 100, 0.8)",
            "Disputed" => "rgba(255, 165, 0, 0.8)",
            "Rejected" => "var(--text-muted)",
            _ => "var(--text-primary)",
        };
        sb_el.style().set_css_text(&format!(
            "font-size: 8px; color: {}; font-family: var(--font-mono); \
             font-weight: 600; text-transform: uppercase;",
            status_color,
        ));
        hdr.append_child(&status_badge).unwrap();
        card.append_child(&hdr).unwrap();

        let desc = document.create_element("div").unwrap();
        desc.set_text_content(Some(text));
        let d_el: HtmlElement = desc.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 10px; color: var(--text-primary); \
             font-family: var(--font-mono); margin-bottom: 4px;",
        );
        card.append_child(&desc).unwrap();

        let conf_row = document.create_element("div").unwrap();
        let cr_el: HtmlElement = conf_row.clone().dyn_into().unwrap();
        cr_el
            .style()
            .set_css_text("display: flex; align-items: center; gap: 6px; margin-bottom: 4px;");

        let conf_lbl = document.create_element("span").unwrap();
        conf_lbl.set_text_content(Some("Confidence:"));
        let cl_el: HtmlElement = conf_lbl.clone().dyn_into().unwrap();
        cl_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        conf_row.append_child(&conf_lbl).unwrap();

        let conf_bar_area = document.create_element("div").unwrap();
        let cba_el: HtmlElement = conf_bar_area.clone().dyn_into().unwrap();
        cba_el.style().set_css_text(
            "flex: 1; height: 6px; background: var(--surface-base); border-radius: 3px; \
             position: relative;",
        );
        let conf_f: f64 = confidence.parse().unwrap_or(0.0);
        let conf_bar = document.create_element("div").unwrap();
        let cb_el: HtmlElement = conf_bar.clone().dyn_into().unwrap();
        cb_el.style().set_css_text(&format!(
            "position: absolute; height: 100%; width: {:.0}%; \
             background: var(--accent-cyan); border-radius: 3px;",
            conf_f * 100.0,
        ));
        cba_el.append_child(&conf_bar).unwrap();
        conf_row.append_child(&conf_bar_area).unwrap();

        let conf_val = document.create_element("span").unwrap();
        conf_val.set_text_content(Some(confidence));
        let cv_el: HtmlElement = conf_val.clone().dyn_into().unwrap();
        cv_el.style().set_css_text(
            "font-size: 8px; color: var(--text-primary); font-family: var(--font-mono); \
             min-width: 30px; text-align: right;",
        );
        conf_row.append_child(&conf_val).unwrap();
        card.append_child(&conf_row).unwrap();

        let tier_row = document.create_element("div").unwrap();
        tier_row.set_text_content(Some(&format!("Disclosure tier: {}", tier)));
        let tr_el: HtmlElement = tier_row.clone().dyn_into().unwrap();
        tr_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
             margin-bottom: 4px;",
        );
        card.append_child(&tier_row).unwrap();

        let evidence_section = document.create_element("div").unwrap();
        let es_el: HtmlElement = evidence_section.clone().dyn_into().unwrap();
        es_el.style().set_css_text("margin-bottom: 4px;");

        for (ev_hyp, ev_type, ev_summary, ev_source) in EVIDENCE {
            if *ev_hyp != *id {
                continue;
            }
            let ev = document.create_element("div").unwrap();
            let ev_el: HtmlElement = ev.clone().dyn_into().unwrap();
            let ev_color = if *ev_type == "supporting" {
                "rgba(100, 200, 100, 0.8)"
            } else {
                "rgba(255, 100, 100, 0.8)"
            };
            ev.set_text_content(Some(&format!(
                "{} {}: {} ({})",
                if *ev_type == "supporting" { "+" } else { "-" },
                ev_type,
                ev_summary,
                ev_source,
            )));
            ev_el.style().set_css_text(&format!(
                "font-size: 8px; color: {}; font-family: var(--font-mono); \
                 padding: 1px 0;",
                ev_color,
            ));
            evidence_section.append_child(&ev).unwrap();
        }
        card.append_child(&evidence_section).unwrap();

        let actions = document.create_element("div").unwrap();
        let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
        a_el.style().set_css_text("display: flex; gap: 4px;");

        for label in &["Confirm", "Reject", "Dispute", "Set Tier"] {
            let btn = document.create_element("button").unwrap();
            btn.set_text_content(Some(label));
            let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
            b_el.style().set_css_text(
                "padding: 2px 6px; border: 1px solid var(--border-medium); \
                 background: transparent; color: var(--text-secondary); border-radius: 3px; \
                 cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
            );
            actions.append_child(&btn).unwrap();
        }
        card.append_child(&actions).unwrap();

        panel.append_child(&card).unwrap();
    }

    let gen_btn = document.create_element("button").unwrap();
    gen_btn.set_text_content(Some("+ Generate Hypotheses"));
    let gb_el: HtmlElement = gen_btn.clone().dyn_into().unwrap();
    gb_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 4px;",
    );
    panel.append_child(&gen_btn).unwrap();

    panel
}
