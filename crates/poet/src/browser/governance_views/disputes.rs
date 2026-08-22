//! Disputes — dispute registry & lifecycle (§8e.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("open", "Open Disputes"),
    ("history", "History"),
    ("parties", "Party Statements"),
    ("resolution", "Resolution"),
];

const DISPUTES: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "DSP-001",
        "contribution",
        "Fair value dispute: contributor_03 vs reviewer_01",
        "high",
        "under_review",
        "2026-08-14",
    ),
    (
        "DSP-002",
        "rights",
        "License terms dispute: CC-BY-NC vs commercial use",
        "normal",
        "mediation",
        "2026-08-10",
    ),
    (
        "DSP-003",
        "factual",
        "Disputed claim: benchmark methodology accuracy",
        "low",
        "open",
        "2026-08-16",
    ),
];

const RESOLVED: &[(&str, &str, &str, &str, &str)] = &[
    (
        "DSP-001",
        "contribution",
        "resolved_by_agreement",
        "2026-08-15",
        "Parties agreed on adjusted fair value",
    ),
    (
        "DSP-000",
        "decision",
        "resolved_by_governance",
        "2026-08-01",
        "Consensus vote: 3-of-3 approved scope change",
    ),
];

const PARTY_STATEMENTS: &[(&str, &str, &str, &str)] = &[
    (
        "DSP-001",
        "did:qualia:contributor_03",
        "Fair value should include expertise premium for NLP specialization",
        "2026-08-14",
    ),
    (
        "DSP-001",
        "did:qualia:reviewer_01",
        "Premium not justified — standard skill multiplier applies",
        "2026-08-14",
    ),
    (
        "DSP-001",
        "did:qualia:reviewer_02",
        "Mediator: suggest 1.5x premium based on market comparison",
        "2026-08-15",
    ),
    (
        "DSP-002",
        "did:qualia:timothy_charles_holborn",
        "CC-BY-NC was selected for research paper only, not for project deliverables",
        "2026-08-10",
    ),
];

pub fn build_disputes_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = build_tab_bar(document, "disputes-tab", TABS);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content.append_child(&build_open_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} dispute resolution requires COP-A3 governance engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_open_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-disputes-panel", "open").unwrap();

    let table = make_table(
        document,
        &["ID", "Category", "Subject", "Urgency", "Status", "Filed"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, category, subject, urgency, status, date) in DISPUTES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, category, subject, urgency, status, date]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "critical" => "rgba(255, 0, 0, 0.9)",
                    "high" => "rgba(255, 100, 100, 0.8)",
                    "normal" => "rgba(255, 165, 0, 0.8)",
                    "low" => "var(--text-muted)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color,
                ));
            } else if i == 4 {
                let color = match **val {
                    "open" => "rgba(255, 100, 100, 0.8)",
                    "under_review" => "rgba(255, 165, 0, 0.8)",
                    "mediation" => "rgba(0, 200, 255, 0.8)",
                    "resolution" => "rgba(100, 200, 100, 0.8)",
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

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ File Dispute"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-disputes-panel", tab_id).unwrap();

    match tab_id {
        "history" => build_history_tab(document, &panel),
        "parties" => build_parties_tab(document, &panel),
        "resolution" => build_resolution_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_history_tab(document: &Document, panel: &Element) {
    let table = make_table(
        document,
        &["ID", "Category", "Resolution Type", "Date", "Summary"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, category, res_type, date, summary) in RESOLVED {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, category, res_type, date, summary].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "resolved_by_agreement" => "rgba(100, 200, 100, 0.8)",
                    "resolved_by_mediation" => "rgba(0, 200, 255, 0.8)",
                    "resolved_by_arbitration" => "rgba(255, 165, 0, 0.8)",
                    "resolved_by_governance" => "rgba(100, 200, 100, 0.8)",
                    "withdrawn" => "var(--text-muted)",
                    "stale" => "var(--text-muted)",
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

fn build_parties_tab(document: &Document, panel: &Element) {
    let table = make_table(document, &["Dispute", "Party", "Statement", "Date"]);
    let tbody = document.create_element("tbody").unwrap();
    for (id, party, statement, date) in PARTY_STATEMENTS {
        let tr = document.create_element("tr").unwrap();
        for val in [id, party, statement, date].iter() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                 color: var(--text-primary); font-size: 10px; \
                 font-family: var(--font-mono);",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();
}

fn build_resolution_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Resolution types: by agreement, by mediation, by arbitration, by governance, withdrawn, stale. \
         Each transition is append-only with provenance.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let lifecycle = &[
        ("open", "Open", "Dispute filed, awaiting review"),
        ("under_review", "Under Review", "Evidence being assessed"),
        (
            "mediation",
            "Mediation",
            "Mediator assigned, parties in dialogue",
        ),
        (
            "resolution",
            "Resolution",
            "Resolution proposed, pending acceptance",
        ),
        ("closed", "Closed", "Dispute resolved or withdrawn"),
    ];

    for (i, (_id, label, desc)) in lifecycle.iter().enumerate() {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        let is_current = i == 1;
        let (bg, border_c, text_c) = if is_current {
            (
                "rgba(255, 165, 0, 0.08)",
                "rgba(255, 165, 0, 0.4)",
                "rgba(255, 165, 0, 0.9)",
            )
        } else {
            ("transparent", "var(--border-subtle)", "var(--text-muted)")
        };
        r_el.style().set_css_text(&format!(
            "display: flex; align-items: center; gap: 8px; padding: 6px 8px; \
             margin-bottom: 4px; border: 1px solid {}; border-radius: 4px; \
             background: {};",
            border_c, bg,
        ));

        let num = document.create_element("span").unwrap();
        num.set_text_content(Some(&format!("{}", i + 1)));
        let n_el: HtmlElement = num.clone().dyn_into().unwrap();
        n_el.style().set_css_text(&format!(
            "width: 20px; height: 20px; border-radius: 50%; display: flex; \
             align-items: center; justify-content: center; font-size: 9px; \
             font-family: var(--font-mono); border: 1px solid {}; color: {};",
            border_c, text_c,
        ));
        row.append_child(&num).unwrap();

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(label));
        let nm_el: HtmlElement = name.clone().dyn_into().unwrap();
        nm_el.style().set_css_text(&format!(
            "font-size: 10px; font-weight: 600; color: {}; min-width: 120px;",
            text_c
        ));
        row.append_child(&name).unwrap();

        let d = document.create_element("span").unwrap();
        d.set_text_content(Some(desc));
        let d_el: HtmlElement = d.clone().dyn_into().unwrap();
        d_el.style()
            .set_css_text("font-size: 9px; color: var(--text-muted); flex: 1;");
        row.append_child(&d).unwrap();

        panel.append_child(&row).unwrap();
    }
}

pub fn build_tab_bar(document: &Document, data_attr: &str, tabs: &[(&str, &str)]) -> Element {
    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in tabs.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute(&format!("data-{}", data_attr), tab_id)
            .unwrap();
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

pub fn make_table(document: &Document, headers: &[&str]) -> Element {
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
