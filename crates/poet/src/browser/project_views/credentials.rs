//! Credentials — project-scoped credential manager (§2.6.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("members", "Membership"),
    ("roles", "Role Credentials"),
    ("professional", "Professional"),
    ("skills", "Skill Credentials"),
];

const MEMBERSHIP: &[(&str, &str, &str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "COP-A2 Membership",
        "active",
        "2026-07-01",
        "2027-07-01",
    ),
    (
        "did:qualia:contributor_02",
        "COP-A2 Membership",
        "active",
        "2026-07-15",
        "2027-07-15",
    ),
    (
        "did:qualia:contributor_03",
        "COP-A2 Membership",
        "active",
        "2026-08-01",
        "2027-08-01",
    ),
    (
        "did:qualia:contributor_04",
        "COP-A2 Membership",
        "pending",
        "2026-08-18",
        "n/a",
    ),
];

const ROLE_CREDS: &[(&str, &str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "Project Founder",
        "active",
        "governance:full",
    ),
    (
        "did:qualia:contributor_02",
        "Reviewer",
        "active",
        "review:full",
    ),
    (
        "did:qualia:contributor_03",
        "Contributor",
        "active",
        "contribute:full",
    ),
];

const PROFESSIONAL: &[(&str, &str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "PhD Computer Science",
        "verified",
        "2026-01-15",
    ),
    (
        "did:qualia:contributor_02",
        "MSc Linguistics",
        "verified",
        "2026-03-20",
    ),
    (
        "did:qualia:contributor_03",
        "BSc Mathematics",
        "pending",
        "2026-08-18",
    ),
];

const SKILLS: &[(&str, &str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "Rust",
        "expert",
        "self-attested + peer-verified",
    ),
    (
        "did:qualia:timothy_charles_holborn",
        "NLP/Computational Linguistics",
        "expert",
        "self-attested + peer-verified",
    ),
    (
        "did:qualia:contributor_02",
        "SHACL/RDF",
        "advanced",
        "self-attested",
    ),
    (
        "did:qualia:contributor_02",
        "Ontology Engineering",
        "advanced",
        "self-attested + peer-verified",
    ),
    (
        "did:qualia:contributor_03",
        "Formal Logic",
        "advanced",
        "self-attested",
    ),
    (
        "did:qualia:contributor_03",
        "Data Analysis",
        "intermediate",
        "self-attested",
    ),
];

pub fn build_credentials_view(document: &Document) -> Element {
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

    content.append_child(&build_members_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} credentials require COP-A2 DID signing engine. \
         Verify/revoke actions need Sentinel VM capability resolution.",
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
        tab.set_attribute("data-credentials-tab", tab_id).unwrap();
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

fn build_members_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-credentials-panel", "members")
        .unwrap();

    let table = make_table(
        document,
        &["DID", "Credential", "Status", "Issued", "Expires"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (did, cred, status, issued, expires) in MEMBERSHIP {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [did, cred, status, issued, expires].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "active" => "rgba(100, 200, 100, 0.8)",
                    "pending" => "rgba(255, 165, 0, 0.8)",
                    "suspended" => "rgba(255, 100, 100, 0.8)",
                    "revoked" => "rgba(255, 0, 0, 0.9)",
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
        .set_attribute("data-credentials-panel", tab_id)
        .unwrap();

    match tab_id {
        "roles" => build_simple_tab(
            document,
            &panel,
            &["DID", "Role", "Status", "Capabilities"],
            ROLE_CREDS,
            2,
        ),
        "professional" => build_simple_tab(
            document,
            &panel,
            &["DID", "Credential", "Status", "Date"],
            PROFESSIONAL,
            2,
        ),
        "skills" => build_simple_tab(
            document,
            &panel,
            &["DID", "Skill", "Level", "Attestation"],
            SKILLS,
            2,
        ),
        _ => {}
    }

    panel
}

fn build_simple_tab(
    document: &Document,
    panel: &Element,
    headers: &[&str],
    rows: &[(&str, &str, &str, &str)],
    status_col: usize,
) {
    let table = make_table(document, headers);
    let tbody = document.create_element("tbody").unwrap();
    for (a, b, c, d) in rows {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [a, b, c, d].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == status_col {
                let color = match **val {
                    "active" | "verified" | "expert" => "rgba(100, 200, 100, 0.8)",
                    "pending" | "advanced" => "rgba(255, 165, 0, 0.8)",
                    "suspended" | "intermediate" => "rgba(200, 200, 100, 0.8)",
                    "revoked" | "entry" => "rgba(255, 100, 100, 0.8)",
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
