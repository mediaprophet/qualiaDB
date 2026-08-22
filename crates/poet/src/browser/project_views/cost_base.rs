//! Cost Base & Obligation — cost entries, obligation, TSL state, consumer classes, commons.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("cost_base", "Cost Base"),
    ("obligation", "Obligation"),
    ("tsl", "TSL State"),
    ("consumer", "Consumer Classes"),
    ("publish", "Commons Publication"),
    ("artefacts", "Commons Artefacts"),
];

pub fn build_cost_base_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-cost-tab", tab_id).unwrap();
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
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content
        .append_child(&build_cost_base_tab(document))
        .unwrap();

    for (i, (_, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_hidden_tab(document, i);
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} cost base requires COP-C1 CostBaseEntry engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_cost_base_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-cost-panel", "cost_base").unwrap();

    let entries = &[
        (
            "did:qualia:timothy_charles_holborn",
            "effort",
            2000000,
            "sats",
            "Phase 1",
            "evidence_001",
        ),
        (
            "did:qualia:timothy_charles_holborn",
            "capital",
            500000,
            "sats",
            "Phase 1",
            "evidence_002",
        ),
        (
            "did:qualia:researcher_01",
            "effort",
            800000,
            "sats",
            "Phase 1",
            "evidence_003",
        ),
        (
            "did:qualia:researcher_01",
            "material",
            120000,
            "sats",
            "Phase 2",
            "evidence_004",
        ),
    ];

    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 10px;");

    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in &[
        "Contributor",
        "Kind",
        "Amount",
        "Currency",
        "Phase",
        "Evidence",
    ] {
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

    let tbody = document.create_element("tbody").unwrap();
    for (who, kind, amount, curr, phase, evidence) in entries {
        let tr = document.create_element("tr").unwrap();
        let amount_str = format!("{} {}", amount, curr);
        for val in &[who, kind, amount_str.as_str(), curr, phase, evidence] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px;",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ Add Cost Entry"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();

    panel
}

fn build_hidden_tab(document: &Document, idx: usize) -> Element {
    let (tab_id, _) = TABS[idx];
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-cost-panel", tab_id).unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text("display: none;");

    let content = match tab_id {
        "obligation" => {
            "Per-contributor obligation:\n\
             \u{2022} timothy_charles_holborn: effort=2000000, capital=500000, score=2.5M\n\
             \u{2022} researcher_01: effort=800000, material=120000, score=0.92M\n\
             \u{2022} Project total: 3.42M \u{2014} Outstanding: 1.2M (35%)"
        }
        "tsl" => {
            "TSL State: A (Commercial)\n\
             \u{2022} Obligation met: 65%\n\
             \u{2022} Shift to State B (Commons): not yet triggered\n\
             \u{2022} WAL audit trail: 0 shift events\n\
             \u{2022} Base fair value: 10000 sats | Risk multiplier: 1.5x"
        }
        "consumer" => {
            "Consumer Class bindings:\n\
             \u{2022} corporation: paydown_weight=1.0\n\
             \u{2022} government: paydown_weight=0.5\n\
             \u{2022} indigenous_knowledge_holder: paydown_weight=0.0 (exempt)\n\
             \u{2022} researcher: paydown_weight=0.0 (exempt)"
        }
        "publish" => {
            "Commons Publication Flow:\n\
             1. Select artefacts to publish\n\
             2. Classify: Selfhood / Personhood / Unmarked / Permissive Commons\n\
             3. Set consumer classes + obligation terms\n\
             4. Emit magnets for Permissive Commons volumes\n\
             \u{26A0} Selfhood artefacts cannot be published to Commons."
        }
        "artefacts" => {
            "Published Artefacts:\n\
             \u{2022} hash:0xabc123 \u{2014} ontology \u{2014} State A \u{2014} 65% paydown\n\
             \u{2022} hash:0xdef456 \u{2014} dataset \u{2014} State A \u{2014} 30% paydown\n\
             \u{2022} hash:0xghi789 \u{2014} ruleset \u{2014} State B \u{2014} 100% paydown \u{2728}"
        }
        _ => "",
    };

    let pre = document.create_element("pre").unwrap();
    pre.set_text_content(Some(content));
    let p_el: HtmlElement = pre.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "padding: 8px; font-size: 10px; color: var(--text-secondary); \
             font-family: var(--font-mono); white-space: pre-wrap; \
             line-height: 1.5;",
    );
    panel.append_child(&pre).unwrap();

    if tab_id == "publish" {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some("\u{1F4E1} Publish to Commons"));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
                 background: transparent; color: var(--text-secondary); border-radius: 3px; \
                 cursor: pointer; font-size: 10px;",
        );
        panel.append_child(&btn).unwrap();
    }

    panel
}
