//! Onboarding — step-by-step project setup wizard (§8f.3).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const STEPS: &[(&str, &str, &str)] = &[
    ("basic_info", "Basic Info", "completed"),
    ("project_type", "Project Type", "completed"),
    ("governance", "Governance Settings", "completed"),
    ("invite", "Invite Members", "active"),
    ("import", "Import Existing Data", "pending"),
    ("agreements", "Define Agreements", "pending"),
    ("compensation", "Set Compensation Model", "pending"),
    ("activate", "Review & Activate", "pending"),
];

const TEMPLATES: &[(&str, &str, &str)] = &[
    (
        "Humanitarian ICT Commons",
        "COP-P5",
        "Agreement Builder, Cost Base, Obligation Tracker, Commons Publication",
    ),
    (
        "Software (Open Source)",
        "COP-P1",
        "Kanban, Cost Base, Wiki, Issues, Automation",
    ),
    (
        "Welfare Support",
        "COP-P3",
        "Project Sheet, Kanban, Budget, Credentials, Governance",
    ),
    (
        "Research Project",
        "COP-P4",
        "Deliverables, Reviews, Roadmap, Wiki, IP Registry, Data Sources",
    ),
    (
        "Civic / Open",
        "COP-P2",
        "Project Sheet, Kanban, Funding, Portal, Governance Meetings",
    ),
];

pub fn build_onboarding_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let header = document.create_element("div").unwrap();
    header.set_text_content(Some(
        "Onboarding Wizard \u{2014} Step 4 of 8: Invite Members",
    ));
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "font-size: 11px; font-weight: 600; color: var(--accent-cyan); \
         font-family: var(--font-mono); padding: 4px 8px; \
         border-bottom: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&header).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px; display: flex; gap: 12px;");

    let steps_panel = build_steps_panel(document);
    content.append_child(&steps_panel).unwrap();

    let detail_panel = build_detail_panel(document);
    content.append_child(&detail_panel).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} onboarding wizard requires COP-P1 project lifecycle engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_steps_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("min-width: 200px; max-width: 220px;");

    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Steps"));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px;",
    );
    panel.append_child(&title).unwrap();

    for (i, (_id, label, status)) in STEPS.iter().enumerate() {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();

        let (bg, border_c, text_c) = match *status {
            "completed" => (
                "rgba(100, 200, 100, 0.06)",
                "rgba(100, 200, 100, 0.3)",
                "rgba(100, 200, 100, 0.8)",
            ),
            "active" => (
                "rgba(0, 200, 255, 0.08)",
                "rgba(0, 200, 255, 0.4)",
                "rgba(0, 200, 255, 0.9)",
            ),
            _ => ("transparent", "var(--border-subtle)", "var(--text-muted)"),
        };

        r_el.style().set_css_text(&format!(
            "display: flex; align-items: center; gap: 6px; padding: 4px 6px; \
             margin-bottom: 3px; border: 1px solid {}; border-radius: 3px; \
             background: {};",
            border_c, bg,
        ));

        let num = document.create_element("span").unwrap();
        num.set_text_content(Some(&format!("{}", i + 1)));
        let n_el: HtmlElement = num.clone().dyn_into().unwrap();
        n_el.style().set_css_text(&format!(
            "width: 16px; height: 16px; border-radius: 50%; display: flex; \
             align-items: center; justify-content: center; font-size: 8px; \
             font-family: var(--font-mono); border: 1px solid {}; color: {};",
            border_c, text_c,
        ));
        row.append_child(&num).unwrap();

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(label));
        let nm_el: HtmlElement = name.clone().dyn_into().unwrap();
        nm_el
            .style()
            .set_css_text(&format!("font-size: 9px; color: {}; flex: 1;", text_c));
        row.append_child(&name).unwrap();

        if *status == "completed" {
            let check = document.create_element("span").unwrap();
            check.set_text_content(Some("\u{2713}"));
            let c_el: HtmlElement = check.clone().dyn_into().unwrap();
            c_el.style()
                .set_css_text("font-size: 9px; color: rgba(100, 200, 100, 0.8);");
            row.append_child(&check).unwrap();
        }

        panel.append_child(&row).unwrap();
    }

    panel
}

fn build_detail_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("flex: 1; display: flex; flex-direction: column; gap: 8px;");

    let section_title = document.create_element("div").unwrap();
    section_title.set_text_content(Some("Invite Members"));
    let st_el: HtmlElement = section_title.clone().dyn_into().unwrap();
    st_el.style().set_css_text(
        "font-size: 11px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    panel.append_child(&section_title).unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Invite contributors by DID or email. Pseudo-anonymous handles supported. \
         Founder mode: enter historical contributors on behalf of others.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); background: var(--surface-panel); \
         border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let invited = &[
        (
            "did:qualia:timothy_charles_holborn",
            "Project Founder",
            "accepted",
        ),
        ("did:qualia:contributor_02", "Reviewer", "accepted"),
        ("did:qualia:contributor_03", "Contributor", "pending"),
        ("contributor_04@example.com", "Contributor", "pending"),
        ("anon_nlp_specialist", "Pseudo-anonymous", "pending"),
    ];

    let table = make_table(document, &["DID / Email / Handle", "Role", "Status"]);
    let tbody = document.create_element("tbody").unwrap();
    for (did, role, status) in invited {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [did, role, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "accepted" => "rgba(100, 200, 100, 0.8)",
                    "pending" => "rgba(255, 165, 0, 0.8)",
                    "declined" => "rgba(255, 100, 100, 0.8)",
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
    add_btn.set_text_content(Some("+ Invite Member"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; align-self: flex-start;",
    );
    panel.append_child(&add_btn).unwrap();

    let sep = document.create_element("div").unwrap();
    let sep_el: HtmlElement = sep.clone().dyn_into().unwrap();
    sep_el
        .style()
        .set_css_text("height: 1px; background: var(--border-subtle); margin: 4px 0;");
    panel.append_child(&sep).unwrap();

    let tmpl_title = document.create_element("div").unwrap();
    tmpl_title.set_text_content(Some("Project Templates"));
    let tt_el: HtmlElement = tmpl_title.clone().dyn_into().unwrap();
    tt_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
         font-family: var(--font-mono);",
    );
    panel.append_child(&tmpl_title).unwrap();

    for (name, cop_type, containers) in TEMPLATES {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "border: 1px solid var(--border-subtle); border-radius: 4px; \
             padding: 6px 8px; margin-bottom: 4px; background: var(--surface-panel);",
        );

        let n = document.create_element("div").unwrap();
        n.set_text_content(Some(name));
        let n_el: HtmlElement = n.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono);",
        );
        card.append_child(&n).unwrap();

        let t = document.create_element("div").unwrap();
        t.set_text_content(Some(cop_type));
        let t_el: HtmlElement = t.clone().dyn_into().unwrap();
        t_el.style().set_css_text(
            "font-size: 8px; color: var(--accent-cyan); font-family: var(--font-mono);",
        );
        card.append_child(&t).unwrap();

        let c = document.create_element("div").unwrap();
        c.set_text_content(Some(containers));
        let c_el: HtmlElement = c.clone().dyn_into().unwrap();
        c_el.style()
            .set_css_text("font-size: 8px; color: var(--text-muted); margin-top: 2px;");
        card.append_child(&c).unwrap();

        panel.append_child(&card).unwrap();
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
