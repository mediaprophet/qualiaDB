//! Rights & Agreements tab views — agreements, deontic norms, jural relations,
//! breach log, consents.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

pub fn build_agreements_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-rights-panel", "agreements")
        .unwrap();

    let agreements: &[(&str, &str, &str, &str, &str)] = &[
        (
            "Data Sharing Agreement",
            "did:qualia:alice \u{2194} did:qualia:timothy_charles_holborn",
            "2-of-3",
            "active",
            "2026-08-01",
        ),
        (
            "Fiduciary Obligation",
            "did:qualia:guardian \u{2192} did:qualia:timothy_charles_holborn",
            "1-of-1",
            "active",
            "2026-08-01",
        ),
        (
            "Software License Accord",
            "did:qualia:corp_01 \u{2194} did:qualia:timothy_charles_holborn",
            "3-of-5",
            "pending",
            "2026-08-10",
        ),
    ];

    for (title, parties, threshold, status, date) in agreements {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "background: var(--surface-panel); border: 1px solid var(--border-subtle); \
             border-radius: 4px; padding: 8px 10px; margin-bottom: 6px;",
        );

        let header = document.create_element("div").unwrap();
        let h_el: HtmlElement = header.clone().dyn_into().unwrap();
        h_el.style()
            .set_css_text("display: flex; justify-content: space-between; margin-bottom: 4px;");

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(title));
        let n_el: HtmlElement = name.clone().dyn_into().unwrap();
        n_el.style()
            .set_css_text("font-size: 11px; font-weight: 600; color: var(--text-primary);");
        header.append_child(&name).unwrap();

        let badge = document.create_element("span").unwrap();
        badge.set_class_name(&format!("honesty-badge honesty-{}", status));
        badge.set_text_content(Some(status));
        header.append_child(&badge).unwrap();
        card.append_child(&header).unwrap();

        let meta = document.create_element("div").unwrap();
        meta.set_text_content(Some(&format!(
            "Parties: {} \u{00B7} Threshold: {} \u{00B7} Signed: {}",
            parties, threshold, date
        )));
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&meta).unwrap();

        panel.append_child(&card).unwrap();
    }

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ New Agreement"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();

    panel
}

pub fn build_deontic_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-rights-panel", "deontic").unwrap();

    let norms: &[(&str, &str, &str, &str)] = &[
        (
            "OBLIGATE",
            "contributor:attribute_effort",
            "active",
            "COP-R4",
        ),
        ("PERMIT", "consumer:replay_artefact", "active", "COP-R4"),
        ("FORBID", "corporation:enclose_commons", "active", "COP-R4"),
        ("OBLIGATE", "reviewer:sign_decision", "active", "LOG-1"),
        ("FORBID", "agent:publish_selfhood", "active", "COP-M1"),
    ];

    let table = make_table(document, &["Modality", "Norm", "Status", "Source"]);
    let tbody = document.create_element("tbody").unwrap();
    for (modality, norm, status, source) in norms {
        let tr = document.create_element("tr").unwrap();
        let mod_color = match *modality {
            "FORBID" => "rgba(255, 99, 71, 0.8)",
            "OBLIGATE" => "rgba(255, 165, 0, 0.8)",
            "PERMIT" => "rgba(100, 200, 100, 0.8)",
            _ => "var(--text-primary)",
        };
        for (i, val) in [modality, norm, status, source].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            let cell_color = if i == 0 {
                mod_color
            } else {
                "var(--text-primary)"
            };
            td_el.style().set_css_text(&format!(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-family: var(--font-mono);",
                cell_color
            ));
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let eval_btn = document.create_element("button").unwrap();
    eval_btn.set_text_content(Some("\u{2696} Evaluate Deontic Contract"));
    let eb_el: HtmlElement = eval_btn.clone().dyn_into().unwrap();
    eb_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&eval_btn).unwrap();

    panel
}

pub fn build_jural_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-rights-panel", "jural").unwrap();

    let positions: &[(&str, &str, &str, &str)] = &[
        ("Right", "Duty", "Privilege", "No-Right"),
        ("Power", "Liability", "Immunity", "Disability"),
        ("Claim", "Obligation", "Freedom", "No-Claim"),
        ("Authority", "Responsibility", "Exemption", "No-Authority"),
    ];

    let table = make_table(
        document,
        &[
            "Role Position",
            "Correlative",
            "Opposite",
            "Opposite Correlative",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (pos, corr, opp, opp_corr) in positions {
        let tr = document.create_element("tr").unwrap();
        for val in &[pos, corr, opp, opp_corr] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                 color: var(--text-primary); font-size: 10px; font-family: var(--font-mono);",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Hohfeldian eight-term analytic \u{2014} LOG-2 jural.rs\n\
         Highlight unmet correlatives in the live graph.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "margin-top: 6px; padding: 6px 8px; font-size: 10px; color: var(--text-muted); \
         font-family: var(--font-mono); background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    panel
}

pub fn build_breach_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-rights-panel", "breach").unwrap();

    let breaches: &[(&str, &str, &str, &str, &str)] = &[
        (
            "norm:obligate_attribute",
            "breached",
            "OBLIGATE",
            "defeasible",
            "2026-08-15",
        ),
        (
            "norm:forbid_enclose",
            "defeated",
            "FORBID",
            "priority",
            "2026-08-10",
        ),
        (
            "norm:permit_replay",
            "active",
            "PERMIT",
            "none",
            "2026-08-01",
        ),
    ];

    let table = make_table(
        document,
        &["Norm", "Status", "Opcode", "Defeat Kind", "Date"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (norm, status, opcode, defeat, date) in breaches {
        let tr = document.create_element("tr").unwrap();
        let status_color = match *status {
            "breached" => "rgba(255, 99, 71, 0.8)",
            "defeated" => "rgba(255, 165, 0, 0.8)",
            _ => "rgba(100, 200, 100, 0.8)",
        };
        for (i, val) in [norm, status, opcode, defeat, date].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    status_color
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Meta-deontic breach log \u{2014} LOG-6 meta_deontic.rs\n\
         WAL audit trail records each breach/defeat with provenance.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "margin-top: 6px; padding: 6px 8px; font-size: 10px; color: var(--text-muted); \
         font-family: var(--font-mono); background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    panel
}

pub fn build_consents_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-rights-panel", "consents")
        .unwrap();

    let consents: &[(&str, &str, &str, &str, &str)] = &[
        (
            "did:qualia:reviewer_01",
            "data_access",
            "granted",
            "revocable",
            "2026-08-01",
        ),
        (
            "did:qualia:health_provider",
            "medical_records",
            "granted",
            "revocable",
            "2026-08-05",
        ),
        (
            "did:qualia:corp_01",
            "commercial_use",
            "denied",
            "irrevocable",
            "2026-08-10",
        ),
        (
            "did:qualia:researcher_01",
            "publication",
            "granted",
            "revocable",
            "2026-08-12",
        ),
    ];

    let table = make_table(
        document,
        &["Grantee", "Scope", "Status", "Revocability", "Date"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (grantee, scope, status, revocability, date) in consents {
        let tr = document.create_element("tr").unwrap();
        let status_color = match *status {
            "granted" => "rgba(100, 200, 100, 0.8)",
            _ => "rgba(255, 99, 71, 0.8)",
        };
        for (i, val) in [grantee, scope, status, revocability, date]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    status_color
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let btn_row = document.create_element("div").unwrap();
    let br_el: HtmlElement = btn_row.clone().dyn_into().unwrap();
    br_el
        .style()
        .set_css_text("display: flex; gap: 6px; margin-top: 6px;");

    let grant_btn = document.create_element("button").unwrap();
    grant_btn.set_text_content(Some("+ Grant Consent"));
    let g_el: HtmlElement = grant_btn.clone().dyn_into().unwrap();
    g_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    btn_row.append_child(&grant_btn).unwrap();

    let revoke_btn = document.create_element("button").unwrap();
    revoke_btn.set_text_content(Some("Revoke Selected"));
    let r_el: HtmlElement = revoke_btn.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    btn_row.append_child(&revoke_btn).unwrap();

    panel.append_child(&btn_row).unwrap();

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
