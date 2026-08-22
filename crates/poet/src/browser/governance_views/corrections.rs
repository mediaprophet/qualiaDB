//! Corrections — correction log & chain viewer (§8e.3).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::disputes::{build_tab_bar, make_table};

const TABS: &[(&str, &str)] = &[
    ("log", "Correction Log"),
    ("chains", "Correction Chains"),
    ("cascade", "Cascade Flags"),
];

const CORRECTIONS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "COR-001",
        "contribution",
        "did:qualia:timothy_charles_holborn",
        "self_correction",
        "superseded",
        "2026-08-15",
    ),
    (
        "COR-002",
        "decision",
        "did:qualia:reviewer_02",
        "proposed",
        "under_review",
        "2026-08-16",
    ),
    (
        "COR-003",
        "claim",
        "did:qualia:reviewer_01",
        "dispute_mandated",
        "superseded",
        "2026-08-14",
    ),
    (
        "COR-004",
        "credential",
        "did:qualia:contributor_03",
        "clarification",
        "linked",
        "2026-08-17",
    ),
];

const CHAINS: &[(&str, &str, &str, &str)] = &[
    (
        "COR-001",
        "Original: fair_value=480 (2026-08-10)",
        "Corrected: fair_value=720 (2026-08-15)",
        "Reason: expertise premium applied",
    ),
    (
        "COR-003",
        "Original: benchmark_accuracy=99.2% (2026-08-12)",
        "Corrected: benchmark_accuracy=97.8% (2026-08-14)",
        "Reason: methodology error in calculation",
    ),
];

const CASCADE: &[(&str, &str, &str, &str)] = &[
    (
        "COR-001",
        "royalty_share",
        "did:qualia:timothy_charles_holborn",
        "flagged",
    ),
    (
        "COR-001",
        "obligation_cost",
        "did:qualia:timothy_charles_holborn",
        "flagged",
    ),
    (
        "COR-003",
        "research_finding",
        "did:qualia:reviewer_02",
        "flagged",
    ),
];

pub fn build_corrections_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = build_tab_bar(document, "corrections-tab", TABS);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content.append_child(&build_log_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} corrections are append-only; originals are superseded, not deleted.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_log_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-corrections-panel", "log")
        .unwrap();

    let table = make_table(
        document,
        &[
            "ID",
            "Record Type",
            "By",
            "Correction Type",
            "Status",
            "Date",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, rtype, by, ctype, status, date) in CORRECTIONS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, rtype, by, ctype, status, date].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "superseded" => "rgba(100, 200, 100, 0.8)",
                    "under_review" => "rgba(255, 165, 0, 0.8)",
                    "linked" => "rgba(0, 200, 255, 0.8)",
                    "rejected" => "rgba(255, 100, 100, 0.8)",
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

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-corrections-panel", tab_id)
        .unwrap();

    match tab_id {
        "chains" => build_chains_tab(document, &panel),
        "cascade" => build_cascade_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_chains_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Correction chains show the full history: original record \u{2192} corrections \u{2192} current. \
         Originals are always visible (superseded, not deleted).",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    for (id, original, corrected, reason) in CHAINS {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "border: 1px solid var(--border-subtle); border-radius: 4px; \
             padding: 8px; margin-bottom: 8px; background: var(--surface-panel);",
        );

        let title = document.create_element("div").unwrap();
        title.set_text_content(Some(id));
        let t_el: HtmlElement = title.clone().dyn_into().unwrap();
        t_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--accent-cyan); \
             font-family: var(--font-mono); margin-bottom: 4px;",
        );
        card.append_child(&title).unwrap();

        for (label, text, color) in [
            ("Original", *original, "var(--text-muted)"),
            ("Corrected", *corrected, "var(--text-primary)"),
            ("Reason", *reason, "var(--text-secondary)"),
        ] {
            let row = document.create_element("div").unwrap();
            row.set_text_content(Some(&format!("{}: {}", label, text)));
            let r_el: HtmlElement = row.clone().dyn_into().unwrap();
            r_el.style().set_css_text(&format!(
                "font-size: 9px; color: {}; font-family: var(--font-mono); \
                 padding: 2px 0;",
                color,
            ));
            card.append_child(&row).unwrap();
        }

        panel.append_child(&card).unwrap();
    }
}

fn build_cascade_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "When a record is corrected, dependent records may need recalculation. \
         Flagged records require review.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(
        document,
        &["Correction", "Dependent Record", "Affected Agent", "Status"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, dep, agent, status) in CASCADE {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, dep, agent, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "flagged" => "rgba(255, 165, 0, 0.8)",
                    "recalculated" => "rgba(100, 200, 100, 0.8)",
                    "ignored" => "var(--text-muted)",
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
