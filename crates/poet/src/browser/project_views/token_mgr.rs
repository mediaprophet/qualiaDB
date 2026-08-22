//! Token Manager — project token definition, issuance, vesting (§8d.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("tokens", "Token Definitions"),
    ("holders", "Holders"),
    ("vesting", "Vesting Schedules"),
    ("rules", "Distribution Rules"),
];

const TOKENS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "QUALIA",
        "QLA",
        "1,000,000",
        "fixed",
        "governance",
        "6 decimals",
    ),
    (
        "ContribReward",
        "CRB",
        "500,000",
        "mintable",
        "reward",
        "2 decimals",
    ),
    (
        "BountyToken",
        "BNT",
        "100,000",
        "mintable",
        "payment",
        "0 decimals",
    ),
];

const HOLDERS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "QLA",
        "50,000",
        "50,000",
        "5.0%",
    ),
    (
        "did:qualia:contributor_02",
        "QLA",
        "20,000",
        "15,000",
        "2.0%",
    ),
    (
        "did:qualia:contributor_03",
        "QLA",
        "10,000",
        "8,000",
        "1.0%",
    ),
    (
        "did:qualia:timothy_charles_holborn",
        "CRB",
        "12,500",
        "12,500",
        "2.5%",
    ),
    ("did:qualia:contributor_02", "CRB", "8,000", "6,000", "1.6%"),
    ("did:qualia:contributor_03", "BNT", "2,000", "1,000", "2.0%"),
];

const VESTING: &[(&str, &str, &str, &str, &str)] = &[
    (
        "did:qualia:contributor_02",
        "QLA",
        "20,000",
        "15,000",
        "milestone: RES-002 passed",
    ),
    (
        "did:qualia:contributor_03",
        "QLA",
        "10,000",
        "8,000",
        "time: 90 days from issuance",
    ),
    (
        "did:qualia:contributor_02",
        "CRB",
        "8,000",
        "6,000",
        "stage: execution phase complete",
    ),
];

const RULES: &[(&str, &str, &str, &str)] = &[
    (
        "per_contribution",
        "CRB",
        "fair_value × 0.1",
        "auto on contribution commit",
    ),
    (
        "per_milestone",
        "QLA",
        "5,000 per milestone",
        "manual by governance",
    ),
    (
        "per_bounty",
        "BNT",
        "bounty reward amount",
        "auto on bounty completion",
    ),
    (
        "per_vote",
        "QLA",
        "100 per governance vote",
        "auto on vote cast",
    ),
];

pub fn build_token_mgr_view(document: &Document) -> Element {
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

    content.append_child(&build_tokens_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} token manager requires COP-C5 funding engine + wallet integration. \
         Token issuance needs DID signing.",
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
        tab.set_attribute("data-tokenmgr-tab", tab_id).unwrap();
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

fn build_tokens_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-tokenmgr-panel", "tokens")
        .unwrap();

    let table = make_table(
        document,
        &["Name", "Symbol", "Supply", "Type", "Purpose", "Decimals"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (name, symbol, supply, supply_type, purpose, decimals) in TOKENS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, symbol, supply, supply_type, purpose, decimals]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "fixed" => "rgba(100, 200, 100, 0.8)",
                    "mintable" => "rgba(255, 165, 0, 0.8)",
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

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ Define Token"));
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
    panel.set_attribute("data-tokenmgr-panel", tab_id).unwrap();

    match tab_id {
        "holders" => build_holders_tab(document, &panel),
        "vesting" => build_vesting_tab(document, &panel),
        "rules" => build_rules_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_holders_tab(document: &Document, panel: &Element) {
    let table = make_table(document, &["DID", "Token", "Balance", "Vested", "Share"]);
    let tbody = document.create_element("tbody").unwrap();
    for (did, token, balance, vested, share) in HOLDERS {
        let tr = document.create_element("tr").unwrap();
        for val in [did, token, balance, vested, share].iter() {
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

fn build_vesting_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Vesting schedules: tokens lock until condition is met (time, milestone, stage transition, TSL shift). \
         Vested tokens are claimable; unvested are locked.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["DID", "Token", "Total", "Vested", "Condition"]);
    let tbody = document.create_element("tbody").unwrap();
    for (did, token, total, vested, condition) in VESTING {
        let tr = document.create_element("tr").unwrap();
        for val in [did, token, total, vested, condition].iter() {
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

fn build_rules_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Distribution rules define how tokens are allocated. Rules can trigger automatically \
         on events (contribution commit, bounty completion, vote cast) or require governance approval.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["Rule", "Token", "Formula", "Trigger"]);
    let tbody = document.create_element("tbody").unwrap();
    for (rule, token, formula, trigger) in RULES {
        let tr = document.create_element("tr").unwrap();
        for val in [rule, token, formula, trigger].iter() {
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
