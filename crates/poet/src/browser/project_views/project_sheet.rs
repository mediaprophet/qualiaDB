//! Project Sheet — metadata, members, roles, licensing, agency, consent tabs.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("overview", "Overview"),
    ("members", "Members"),
    ("roles", "Roles & Permissions"),
    ("licensing", "Licensing"),
    ("agency", "Agency & Delegation"),
    ("consent", "Consent"),
];

const PROJECT_TYPES: &[(&str, &str)] = &[
    ("welfare_support", "Welfare Support"),
    ("professional_house", "Professional \u{2014} House Build"),
    ("professional_open", "Professional / Open (Software)"),
    ("civic_open", "Civic / Open"),
    ("research", "Research"),
    ("humanitarian_ict", "Humanitarian ICT / Commons"),
];

const SENSITIVITY_CLASSES: &[(&str, &str)] = &[
    ("public", "Public"),
    ("restricted", "Restricted"),
    ("classified", "Classified"),
];

pub fn build_project_sheet_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    // Header badges
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "display: flex; gap: 6px; flex-wrap: wrap; padding: 4px 8px; \
         border-bottom: 1px solid var(--border-subtle);",
    );
    for (label, cls) in &[
        ("Type: Research", "tag-epistemic"),
        ("Sensitivity: Permissive", "honesty-present"),
        ("Values: UN-HR", "tag-governance"),
        ("License: Permissive Commons", "tag-governance"),
    ] {
        let badge = document.create_element("span").unwrap();
        badge.set_class_name(&format!("container-type-tag {}", cls));
        badge.set_text_content(Some(label));
        let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
        b_el.style()
            .set_css_text("font-size: 9px; padding: 1px 6px;");
        header.append_child(&badge).unwrap();
    }
    wrapper.append_child(&header).unwrap();

    // Tab bar
    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-ps-tab", tab_id).unwrap();
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

    // Tab content area
    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Overview tab (visible by default)
    let overview = build_overview_tab(document);
    content.append_child(&overview).unwrap();

    // Members tab (hidden)
    let members = build_members_tab(document);
    let m_el: HtmlElement = members.clone().dyn_into().unwrap();
    m_el.style().set_css_text("display: none;");
    content.append_child(&members).unwrap();

    // Roles tab (hidden)
    let roles = build_roles_tab(document);
    let r_el: HtmlElement = roles.clone().dyn_into().unwrap();
    r_el.style().set_css_text("display: none;");
    content.append_child(&roles).unwrap();

    // Licensing tab (hidden)
    let licensing = build_licensing_tab(document);
    let l_el: HtmlElement = licensing.clone().dyn_into().unwrap();
    l_el.style().set_css_text("display: none;");
    content.append_child(&licensing).unwrap();

    // Agency tab (hidden)
    let agency = build_agency_tab(document);
    let a_el: HtmlElement = agency.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: none;");
    content.append_child(&agency).unwrap();

    // Consent tab (hidden)
    let consent = build_consent_tab(document);
    let co_el: HtmlElement = consent.clone().dyn_into().unwrap();
    co_el.style().set_css_text("display: none;");
    content.append_child(&consent).unwrap();

    wrapper.append_child(&content).unwrap();

    // Honesty footer
    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} project metadata requires wellfair_add_project engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_overview_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-ps-panel", "overview").unwrap();

    for (label, value, is_select) in &[
        ("Project name", "Qualia NLP Engine", false),
        (
            "Description",
            "High-performance NLP and semantic enrichment library for the QualiaDB ecosystem.",
            false,
        ),
        ("Project type", "", true),
        ("Sensitivity class", "", true),
        ("Created at", "2026-08-01T00:00:00Z", false),
        (
            "Values anchor",
            "UN Universal Declaration of Human Rights",
            false,
        ),
    ] {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; flex-direction: column; gap: 2px; \
             margin-bottom: 8px;",
        );

        let lbl = document.create_element("label").unwrap();
        lbl.set_text_content(Some(label));
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); \
             font-family: var(--font-mono);",
        );
        row.append_child(&lbl).unwrap();

        if *is_select {
            let select = document.create_element("select").unwrap();
            let s_el: HtmlElement = select.clone().dyn_into().unwrap();
            s_el.style().set_css_text(
                "padding: 4px 6px; background: var(--surface-panel); \
                 border: 1px solid var(--border-medium); border-radius: 3px; \
                 color: var(--text-primary); font-size: 11px;",
            );
            let options = if label.contains("type") {
                PROJECT_TYPES
            } else {
                SENSITIVITY_CLASSES
            };
            for (val, text) in options {
                let opt = document.create_element("option").unwrap();
                opt.set_attribute("value", val).unwrap();
                opt.set_text_content(Some(text));
                select.append_child(&opt).unwrap();
            }
            row.append_child(&select).unwrap();
        } else {
            let input = document.create_element("input").unwrap();
            input
                .clone()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap()
                .set_value(value);
            let i_el: HtmlElement = input.clone().dyn_into().unwrap();
            i_el.style().set_css_text(
                "padding: 4px 6px; background: var(--surface-panel); \
                 border: 1px solid var(--border-medium); border-radius: 3px; \
                 color: var(--text-primary); font-size: 11px;",
            );
            row.append_child(&input).unwrap();
        }

        panel.append_child(&row).unwrap();
    }

    panel
}

fn build_members_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-ps-panel", "members").unwrap();

    let members = &[
        (
            "did:qualia:timothy_charles_holborn",
            "Principal",
            "verified",
            "2026-08-01",
        ),
        (
            "did:qualia:researcher_01",
            "Researcher",
            "verified",
            "2026-08-05",
        ),
        (
            "did:qualia:reviewer_02",
            "Reviewer",
            "pending",
            "2026-08-10",
        ),
    ];

    for (did, role, cred_status, joined) in members {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; align-items: center; gap: 8px; padding: 6px 8px; \
             border-bottom: 1px solid var(--border-subtle); font-size: 11px;",
        );

        let did_span = document.create_element("span").unwrap();
        did_span.set_text_content(Some(did));
        let ds_el: HtmlElement = did_span.clone().dyn_into().unwrap();
        ds_el
            .style()
            .set_css_text("flex: 1; color: var(--text-primary); font-family: var(--font-mono);");
        row.append_child(&did_span).unwrap();

        let role_span = document.create_element("span").unwrap();
        role_span.set_text_content(Some(role));
        let rs_el: HtmlElement = role_span.clone().dyn_into().unwrap();
        rs_el
            .style()
            .set_css_text("color: var(--accent-cyan); font-size: 10px;");
        row.append_child(&role_span).unwrap();

        let cred = document.create_element("span").unwrap();
        cred.set_text_content(Some(cred_status));
        cred.set_class_name(&format!("honesty-badge honesty-{}", cred_status));
        let c_el: HtmlElement = cred.clone().dyn_into().unwrap();
        c_el.style()
            .set_css_text("font-size: 9px; padding: 1px 4px;");
        row.append_child(&cred).unwrap();

        let date = document.create_element("span").unwrap();
        date.set_text_content(Some(joined));
        let dt_el: HtmlElement = date.clone().dyn_into().unwrap();
        dt_el
            .style()
            .set_css_text("color: var(--text-muted); font-size: 9px;");
        row.append_child(&date).unwrap();

        panel.append_child(&row).unwrap();
    }

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ Add Member"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 8px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();

    panel
}

fn build_roles_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-ps-panel", "roles").unwrap();

    let roles = &[
        ("Principal", "Right", "Power", "Full project authority"),
        (
            "Researcher",
            "Privilege",
            "Immunity",
            "Contribute work items, edit deliverables",
        ),
        (
            "Reviewer",
            "Duty",
            "Liability",
            "Review assignments, sign decisions",
        ),
        (
            "Funder",
            "Right",
            "Disability",
            "Fund project, view financials",
        ),
    ];

    // Role table
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 10px;");

    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in &["Role", "Hohfeld Position", "Correlative", "Scope"] {
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
    for (role, pos, corr, scope) in roles {
        let tr = document.create_element("tr").unwrap();
        for val in &[role, pos, corr, scope] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary);",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    panel
}

fn build_licensing_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-ps-panel", "licensing").unwrap();

    for (label, value) in &[
        ("Licensing profile", "Permissive Commons (share-alike)"),
        ("Obligation terms", "Cost-base paydown by consumer class"),
        ("Share-alike", "Enabled"),
        (
            "Consumer class restrictions",
            "corporation: paydown; indigenous_knowledge_holder: exempt",
        ),
        ("TSL base fair value", "10000 sats"),
        ("TSL risk multiplier", "1.5x"),
        ("TSL temporal compound", "0.1% per day"),
    ] {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; justify-content: space-between; padding: 4px 8px; \
             border-bottom: 1px solid var(--border-subtle); font-size: 10px;",
        );
        let lbl = document.create_element("span").unwrap();
        lbl.set_text_content(Some(label));
        let lb_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        lb_el
            .style()
            .set_css_text("color: var(--text-muted); font-family: var(--font-mono);");
        row.append_child(&lbl).unwrap();
        let val = document.create_element("span").unwrap();
        val.set_text_content(Some(value));
        let v_el: HtmlElement = val.clone().dyn_into().unwrap();
        v_el.style().set_css_text("color: var(--text-primary);");
        row.append_child(&val).unwrap();
        panel.append_child(&row).unwrap();
    }

    panel
}

fn build_agency_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-ps-panel", "agency").unwrap();

    let delegations = &[
        (
            "Social Worker",
            "personal_welfare",
            "HumanConsensus 1-of-1",
            "active",
            "2026-08-01",
        ),
        (
            "Health Provider",
            "medical",
            "HumanConsensus 2-of-3",
            "active",
            "2026-08-01",
        ),
        (
            "Financial Counselor",
            "financial",
            "VerifiableEvent",
            "active",
            "2026-08-01",
        ),
    ];

    for (agent, domain, trigger, status, valid_from) in delegations {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "padding: 8px; border: 1px solid var(--border-medium); \
             border-radius: 4px; margin-bottom: 6px; background: var(--surface-panel);",
        );

        let top = document.create_element("div").unwrap();
        let t_el: HtmlElement = top.clone().dyn_into().unwrap();
        t_el.style()
            .set_css_text("display: flex; justify-content: space-between; margin-bottom: 4px;");
        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(agent));
        let n_el: HtmlElement = name.clone().dyn_into().unwrap();
        n_el.style()
            .set_css_text("font-weight: 600; color: var(--text-primary);");
        top.append_child(&name).unwrap();
        let badge = document.create_element("span").unwrap();
        badge.set_class_name(&format!("honesty-badge honesty-{}", status));
        badge.set_text_content(Some(status));
        top.append_child(&badge).unwrap();
        card.append_child(&top).unwrap();

        let meta = document.create_element("div").unwrap();
        meta.set_text_content(Some(&format!(
            "Domain: {} \u{00B7} Trigger: {} \u{00B7} From: {}",
            domain, trigger, valid_from
        )));
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&meta).unwrap();

        panel.append_child(&card).unwrap();
    }

    let revoke_btn = document.create_element("button").unwrap();
    revoke_btn.set_text_content(Some("Revoke Selected"));
    let rb_el: HtmlElement = revoke_btn.clone().dyn_into().unwrap();
    rb_el.style().set_css_text(
        "margin-top: 4px; padding: 4px 12px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&revoke_btn).unwrap();

    panel
}

fn build_consent_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-ps-panel", "consent").unwrap();

    let consents = &[
        (
            "did:qualia:timothy_charles_holborn",
            "project_participation",
            "granted",
            "2026-08-01",
        ),
        (
            "did:qualia:researcher_01",
            "data_sharing",
            "granted",
            "2026-08-05",
        ),
        (
            "did:qualia:reviewer_02",
            "publication",
            "pending",
            "2026-08-10",
        ),
    ];

    for (principal, scope, status, granted_at) in consents {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; align-items: center; gap: 8px; padding: 6px 8px; \
             border-bottom: 1px solid var(--border-subtle); font-size: 10px;",
        );

        let who = document.create_element("span").unwrap();
        who.set_text_content(Some(principal));
        let w_el: HtmlElement = who.clone().dyn_into().unwrap();
        w_el.style()
            .set_css_text("flex: 1; color: var(--text-primary); font-family: var(--font-mono);");
        row.append_child(&who).unwrap();

        let sc = document.create_element("span").unwrap();
        sc.set_text_content(Some(scope));
        let sc_el: HtmlElement = sc.clone().dyn_into().unwrap();
        sc_el.style().set_css_text("color: var(--accent-cyan);");
        row.append_child(&sc).unwrap();

        let badge = document.create_element("span").unwrap();
        badge.set_class_name(&format!("honesty-badge honesty-{}", status));
        badge.set_text_content(Some(status));
        row.append_child(&badge).unwrap();

        let date = document.create_element("span").unwrap();
        date.set_text_content(Some(granted_at));
        let dt_el: HtmlElement = date.clone().dyn_into().unwrap();
        dt_el
            .style()
            .set_css_text("color: var(--text-muted); font-size: 9px;");
        row.append_child(&date).unwrap();

        panel.append_child(&row).unwrap();
    }

    panel
}
