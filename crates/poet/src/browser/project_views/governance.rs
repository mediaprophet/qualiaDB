//! Governance — project policy configuration (§2.4.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("policy", "Policy Settings"),
    ("roles", "Role Taxonomy"),
    ("values", "Values Anchor"),
    ("escalation", "Escalation"),
];

const ROLES: &[(&str, &str, &str)] = &[
    (
        "Project Founder",
        "did:qualia:timothy_charles_holborn",
        "full",
    ),
    ("Reviewer", "did:qualia:contributor_02", "review + comment"),
    (
        "Contributor",
        "did:qualia:contributor_03",
        "contribute + comment",
    ),
    ("Observer", "(any DID)", "read-only"),
];

const VALUES: &[(&str, &str, &str)] = &[
    ("Human Rights", "UDHR, ICCPR, ICESCR", "non-negotiable"),
    (
        "Open Knowledge",
        "CC-BY, CC-BY-SA, Permissive Commons",
        "non-negotiable",
    ),
    (
        "Fair Compensation",
        "PPP-adjusted, skill-tiered",
        "configurable",
    ),
    (
        "Transparency",
        "Append-only, provenance chain",
        "non-negotiable",
    ),
    (
        "Peace & Non-Harm",
        "Do-no-harm, humanitarian principles",
        "non-negotiable",
    ),
];

const ESCALATION: &[(&str, &str, &str)] = &[
    ("1. Review", "Assigned reviewer", "48 hours"),
    ("2. Mediation", "Project Founder + 1 contributor", "7 days"),
    ("3. Governance Vote", "M-of-N consensus", "14 days"),
    (
        "4. Arbitration",
        "External arbitrator (if configured)",
        "30 days",
    ),
];

pub fn build_governance_view(document: &Document) -> Element {
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

    content.append_child(&build_policy_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} governance config requires COP-R4 deontic engine + LOG-1/2/5 logic modalities.",
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
        tab.set_attribute("data-governance-tab", tab_id).unwrap();
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

fn build_policy_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-governance-panel", "policy")
        .unwrap();

    let settings: &[(&str, &str, &str)] = &[
        ("Decision Threshold", "M-of-N", "3 of 5 (60%)"),
        ("Consensus Protocol", "select", "Ranked-choice"),
        (
            "Deontic Norm Compilation",
            "select",
            "Strict (reject on conflict)",
        ),
        (
            "Sensitivity Class Policy",
            "select",
            "Public default, Restricted on flag",
        ),
        ("Voting Timeout", "text", "14 days"),
        ("Amendment Quorum", "text", "4 of 5 (80%)"),
        (
            "Escalation Path",
            "select",
            "Review \u{2192} Mediation \u{2192} Vote \u{2192} Arbitration",
        ),
    ];

    for (label, input_type, value) in settings {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; align-items: center; gap: 8px; padding: 4px 0; \
             border-bottom: 1px solid var(--border-subtle);",
        );

        let lbl = document.create_element("label").unwrap();
        lbl.set_text_content(Some(label));
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-secondary); font-family: var(--font-mono); \
             min-width: 200px;",
        );
        row.append_child(&lbl).unwrap();

        if *input_type == "select" {
            let sel = document.create_element("select").unwrap();
            let opt = document.create_element("option").unwrap();
            opt.set_text_content(Some(value));
            sel.append_child(&opt).unwrap();
            let s_el: HtmlElement = sel.clone().dyn_into().unwrap();
            s_el.style().set_css_text(
                "flex: 1; padding: 3px 6px; border: 1px solid var(--border-medium); \
                 border-radius: 3px; background: var(--surface-panel); \
                 color: var(--text-primary); font-size: 10px; font-family: var(--font-mono);",
            );
            row.append_child(&sel).unwrap();
        } else {
            let inp = document.create_element("input").unwrap();
            inp.set_attribute("type", "text").unwrap();
            inp.set_attribute("value", value).unwrap();
            let i_el: HtmlElement = inp.clone().dyn_into().unwrap();
            i_el.style().set_css_text(
                "flex: 1; padding: 3px 6px; border: 1px solid var(--border-medium); \
                 border-radius: 3px; background: var(--surface-panel); \
                 color: var(--text-primary); font-size: 10px; font-family: var(--font-mono);",
            );
            row.append_child(&inp).unwrap();
        }

        panel.append_child(&row).unwrap();
    }

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-governance-panel", tab_id)
        .unwrap();

    match tab_id {
        "roles" => build_roles_tab(document, &panel),
        "values" => build_values_tab(document, &panel),
        "escalation" => build_escalation_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_roles_tab(document: &Document, panel: &Element) {
    let table = make_table(document, &["Role", "Holder", "Capabilities"]);
    let tbody = document.create_element("tbody").unwrap();
    for (role, holder, caps) in ROLES {
        let tr = document.create_element("tr").unwrap();
        for val in [role, holder, caps].iter() {
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

fn build_values_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Values anchors are foundational instruments that all agreements inherit. \
         Non-negotiable anchors cannot be overridden by project policy.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["Value", "Instruments", "Mutability"]);
    let tbody = document.create_element("tbody").unwrap();
    for (name, instruments, mutability) in VALUES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, instruments, mutability].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "non-negotiable" => "rgba(255, 100, 100, 0.8)",
                    "configurable" => "rgba(255, 165, 0, 0.8)",
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

fn build_escalation_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Escalation path for disputes and complaints. Each stage has a timeout; \
         failure to resolve within timeout advances to the next stage.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    for (stage, body, timeout) in ESCALATION {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; align-items: center; gap: 8px; padding: 6px 8px; \
             border: 1px solid var(--border-subtle); border-radius: 4px; \
             margin-bottom: 4px; background: var(--surface-panel);",
        );

        let s = document.create_element("div").unwrap();
        s.set_text_content(Some(stage));
        let s_el: HtmlElement = s.clone().dyn_into().unwrap();
        s_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--accent-cyan); \
             font-family: var(--font-mono); min-width: 200px;",
        );
        row.append_child(&s).unwrap();

        let b = document.create_element("div").unwrap();
        b.set_text_content(Some(body));
        let b_el: HtmlElement = b.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "font-size: 10px; color: var(--text-secondary); flex: 1; \
             font-family: var(--font-mono);",
        );
        row.append_child(&b).unwrap();

        let t = document.create_element("div").unwrap();
        t.set_text_content(Some(timeout));
        let t_el: HtmlElement = t.clone().dyn_into().unwrap();
        t_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        row.append_child(&t).unwrap();

        panel.append_child(&row).unwrap();
    }
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
