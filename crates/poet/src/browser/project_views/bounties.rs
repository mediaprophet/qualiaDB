//! Bounties — bounty board with claim and payout tracking (§2.7.3).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("open", "Open Bounties"),
    ("claimed", "Claimed"),
    ("completed", "Completed"),
];

const BOUNTIES: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    (
        "BNT-001",
        "Write SHACL test suite",
        "5,000 XEC",
        "did:qualia:timothy_charles_holborn",
        "open",
        "unclaimed",
        "",
    ),
    (
        "BNT-002",
        "Optimise FST dictionary loading",
        "3,000 XEC",
        "did:qualia:timothy_charles_holborn",
        "open",
        "unclaimed",
        "",
    ),
    (
        "BNT-003",
        "Create ontology visualisation tool",
        "8,000 XEC",
        "did:qualia:contributor_02",
        "open",
        "unclaimed",
        "",
    ),
    (
        "BNT-004",
        "Fix CBOR-LD provenance bug",
        "2,000 XEC",
        "did:qualia:timothy_charles_holborn",
        "claimed",
        "claimed",
        "did:qualia:contributor_03",
    ),
    (
        "BNT-005",
        "Write contributor guide v4",
        "1,500 XEC",
        "did:qualia:timothy_charles_holborn",
        "completed",
        "paid",
        "did:qualia:contributor_02",
    ),
    (
        "BNT-006",
        "Benchmark graph index performance",
        "1,000 XEC",
        "did:qualia:timothy_charles_holborn",
        "completed",
        "paid",
        "did:qualia:contributor_03",
    ),
];

pub fn build_bounties_view(document: &Document) -> Element {
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
        .append_child(&build_bounties_tab(document, "open"))
        .unwrap();

    for (i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_bounties_tab(document, tab_id);
        if i > 0 {
            let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
            p_el.style().set_css_text("display: none;");
        }
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} bounties require COP-C5 funding engine + wallet integration + escrow support.",
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
        tab.set_attribute("data-bounties-tab", tab_id).unwrap();
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

fn build_bounties_tab(document: &Document, filter: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-bounties-panel", filter).unwrap();

    let table = make_table(
        document,
        &["ID", "Task", "Reward", "Sponsor", "Claimant", "Payout", ""],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, task, reward, sponsor, status, payout, claimant) in BOUNTIES {
        let show = match filter {
            "open" => *status == "open",
            "claimed" => *status == "claimed",
            "completed" => *status == "completed",
            _ => true,
        };
        if !show {
            continue;
        }

        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, task, reward, sponsor, claimant, payout]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 5 {
                let color = match **val {
                    "paid" => "rgba(100, 200, 100, 0.8)",
                    "claimed" => "rgba(255, 165, 0, 0.8)",
                    "unclaimed" => "var(--text-muted)",
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

        let action_td = document.create_element("td").unwrap();
        let a_el: HtmlElement = action_td.clone().dyn_into().unwrap();
        a_el.style()
            .set_css_text("padding: 4px 6px; border-bottom: 1px solid var(--border-subtle);");
        let btn = document.create_element("button").unwrap();
        let label = match *status {
            "open" => "Claim",
            "claimed" => "Submit Evidence",
            "completed" => "View",
            _ => "View",
        };
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 8px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 9px; font-family: var(--font-mono);",
        );
        action_td.append_child(&btn).unwrap();
        tr.append_child(&action_td).unwrap();

        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    if filter == "open" {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some("+ Post Bounty"));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 4px 12px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 10px; margin-top: 6px;",
        );
        panel.append_child(&btn).unwrap();
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
