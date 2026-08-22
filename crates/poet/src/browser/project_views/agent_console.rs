//! Agent Console — project specialist agent query interface (§8k.2).
//!
//! Software agents are computational systems — no mind, no intent.
//! Responses are model assertions requiring verification by a natural agent.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("query", "Query"),
    ("agents", "Agent Definitions"),
    ("provenance", "Provenance Log"),
];

const AGENTS: &[(&str, &str, &str, &str)] = &[
    (
        "NLP Project Specialist",
        "read_only",
        "wiki, decisions, tasks, deliverables",
        "local",
    ),
    (
        "Ontology Reviewer",
        "read_only",
        "wiki, ontology, SHACL shapes",
        "local",
    ),
    (
        "Governance Assistant",
        "read_only",
        "governance, meetings, resolutions, COI",
        "remote",
    ),
];

const PROVENANCE_LOG: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Q-001",
        "NLP Project Specialist",
        "What is the current ontology approach?",
        "2026-08-18 10:32",
        "grounded",
    ),
    (
        "Q-002",
        "Ontology Reviewer",
        "Are SHACL shapes valid for N3?",
        "2026-08-18 10:35",
        "grounded",
    ),
    (
        "Q-003",
        "Governance Assistant",
        "What decisions are pending?",
        "2026-08-18 10:40",
        "grounded",
    ),
    (
        "Q-004",
        "NLP Project Specialist",
        "Summarise contribution distribution",
        "2026-08-18 11:00",
        "partially_grounded",
    ),
];

const SAMPLE_RESPONSE: &str = "The project currently uses N3 (Notation3) for ontology definition, \
with SHACL shapes for validation. The approach was approved on 2026-08-08 via governance resolution \
RES-002 (2-1-0 vote). Key deliverables include the Ontology Specification (authored by \
did:qualia:timothy_charles_holborn) and the SHACL Shapes Library (authored by \
did:qualia:contributor_02).";

pub fn build_agent_console_view(document: &Document) -> Element {
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

    content.append_child(&build_query_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} agent responses are model assertions requiring verification. \
         Agents are capability-gated (read-only by default). No mind, no intent.",
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
        tab.set_attribute("data-agent-tab", tab_id).unwrap();
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

fn build_query_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-agent-panel", "query").unwrap();

    let selector = document.create_element("div").unwrap();
    selector.set_text_content(Some("Agent: NLP Project Specialist (read-only, local)"));
    let sel_el: HtmlElement = selector.clone().dyn_into().unwrap();
    sel_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--accent-cyan); \
         font-family: var(--font-mono); padding: 4px 8px; \
         background: var(--surface-panel); border-radius: 4px; margin-bottom: 6px;",
    );
    panel.append_child(&selector).unwrap();

    let chat_area = document.create_element("div").unwrap();
    let ca_el: HtmlElement = chat_area.clone().dyn_into().unwrap();
    ca_el
        .style()
        .set_css_text("display: flex; flex-direction: column; gap: 8px; margin-bottom: 8px;");

    let query_msg = document.create_element("div").unwrap();
    query_msg.set_text_content(Some("Q: What is the current ontology approach?"));
    let q_el: HtmlElement = query_msg.clone().dyn_into().unwrap();
    q_el.style().set_css_text(
        "font-size: 10px; color: var(--text-primary); font-family: var(--font-mono); \
         padding: 6px 8px; background: var(--surface-panel); border-radius: 4px; \
         border-left: 2px solid var(--accent-cyan);",
    );
    chat_area.append_child(&query_msg).unwrap();

    let response_msg = document.create_element("div").unwrap();
    response_msg.set_text_content(Some(SAMPLE_RESPONSE));
    let r_el: HtmlElement = response_msg.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "font-size: 10px; color: var(--text-secondary); font-family: var(--font-mono); \
         padding: 6px 8px; background: var(--surface-glass); border-radius: 4px; \
         border-left: 2px solid rgba(100, 200, 100, 0.4); line-height: 1.5;",
    );
    chat_area.append_child(&response_msg).unwrap();

    let citations = document.create_element("div").unwrap();
    citations.set_text_content(Some(
        "Citations: [1] Ontology Specification (wiki, 2026-08-03)  [2] RES-002 (decision, 2026-08-08)  \
         [3] SHACL Shapes Library (wiki, 2026-08-05)",
    ));
    let cit_el: HtmlElement = citations.clone().dyn_into().unwrap();
    cit_el.style().set_css_text(
        "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
         padding: 4px 8px; border-top: 1px solid var(--border-subtle);",
    );
    chat_area.append_child(&citations).unwrap();

    let warning = document.create_element("div").unwrap();
    warning.set_text_content(Some(
        "\u{26A0} Model assertion \u{2014} verify before acting. Grounded in knowledge base (3 citations).",
    ));
    let w_el: HtmlElement = warning.clone().dyn_into().unwrap();
    w_el.style().set_css_text(
        "font-size: 8px; color: rgba(255, 165, 0, 0.8); font-family: var(--font-mono); \
         padding: 2px 8px;",
    );
    chat_area.append_child(&warning).unwrap();

    panel.append_child(&chat_area).unwrap();

    let input_row = document.create_element("div").unwrap();
    let ir_el: HtmlElement = input_row.clone().dyn_into().unwrap();
    ir_el.style().set_css_text("display: flex; gap: 6px;");

    let input = document.create_element("input").unwrap();
    input.set_attribute("type", "text").unwrap();
    input
        .set_attribute("placeholder", "Ask a question...")
        .unwrap();
    let inp_el: HtmlElement = input.clone().dyn_into().unwrap();
    inp_el.style().set_css_text(
        "flex: 1; padding: 6px 10px; border: 1px solid var(--border-medium); \
         border-radius: 4px; background: var(--surface-panel); color: var(--text-primary); \
         font-size: 10px; font-family: var(--font-mono); box-sizing: border-box;",
    );
    input_row.append_child(&input).unwrap();

    let send_btn = document.create_element("button").unwrap();
    send_btn.set_text_content(Some("Send"));
    let sb_el: HtmlElement = send_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 6px 16px; border: 1px solid var(--accent-cyan); \
         background: rgba(0, 200, 255, 0.08); color: var(--accent-cyan); \
         border-radius: 4px; cursor: pointer; font-size: 10px; \
         font-family: var(--font-mono);",
    );
    input_row.append_child(&send_btn).unwrap();

    panel.append_child(&input_row).unwrap();

    let feedback = document.create_element("div").unwrap();
    let fb_el: HtmlElement = feedback.clone().dyn_into().unwrap();
    fb_el.style().set_css_text(
        "display: flex; gap: 8px; margin-top: 6px; font-size: 9px; \
         color: var(--text-muted); font-family: var(--font-mono);",
    );

    let helpful = document.create_element("span").unwrap();
    helpful.set_text_content(Some("\u{2705} Helpful"));
    let h_el: HtmlElement = helpful.clone().dyn_into().unwrap();
    h_el.style().set_css_text("cursor: pointer;");
    fb_el.append_child(&helpful).unwrap();

    let not_helpful = document.create_element("span").unwrap();
    not_helpful.set_text_content(Some("\u{274C} Not helpful"));
    let nh_el: HtmlElement = not_helpful.clone().dyn_into().unwrap();
    nh_el.style().set_css_text("cursor: pointer;");
    fb_el.append_child(&not_helpful).unwrap();

    let accurate = document.create_element("span").unwrap();
    accurate.set_text_content(Some("\u{2705} Accurate"));
    let a_el: HtmlElement = accurate.clone().dyn_into().unwrap();
    a_el.style().set_css_text("cursor: pointer;");
    fb_el.append_child(&accurate).unwrap();

    let inaccurate = document.create_element("span").unwrap();
    inaccurate.set_text_content(Some("\u{274C} Inaccurate"));
    let in_el: HtmlElement = inaccurate.clone().dyn_into().unwrap();
    in_el.style().set_css_text("cursor: pointer;");
    fb_el.append_child(&inaccurate).unwrap();

    panel.append_child(&feedback).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-agent-panel", tab_id).unwrap();

    match tab_id {
        "agents" => build_agents_tab(document, &panel),
        "provenance" => build_provenance_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_agents_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Software agents are computational systems \u{2014} no mind, no intent. \
         Capability-gated: read-only by default. Context window populated from knowledge base.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["Agent", "Capabilities", "Scope", "Model"]);
    let tbody = document.create_element("tbody").unwrap();
    for (name, caps, scope, model) in AGENTS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, caps, scope, model].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                let color = match **val {
                    "read_only" => "rgba(100, 200, 100, 0.8)",
                    "write_limited" => "rgba(255, 165, 0, 0.8)",
                    "write_full" => "rgba(255, 100, 100, 0.8)",
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

fn build_provenance_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Every agent response is logged: agent ID, query, response, source citations, \
         context window contents, model version, timestamp. Append-only audit trail.",
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
        &["ID", "Agent", "Query", "Timestamp", "Grounding"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, agent, query, ts, grounding) in PROVENANCE_LOG {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, agent, query, ts, grounding].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "grounded" => "rgba(100, 200, 100, 0.8)",
                    "partially_grounded" => "rgba(255, 165, 0, 0.8)",
                    "ungrounded" => "rgba(255, 100, 100, 0.8)",
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
