//! Agreement Builder — instrument-based agreement composition (§8a).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::super::cop_records::{build_family_panel, CopField};
use super::super::live_invoke;

const TABS: &[(&str, &str)] = &[
    ("instruments", "Instruments"),
    ("clauses", "Clauses"),
    ("signatories", "Signatories"),
    ("lifecycle", "Lifecycle"),
    ("conflicts", "Conflicts"),
];

const INSTRUMENTS: &[(&str, &str, &str, &str)] = &[
    (
        "UDHR",
        "human_rights",
        "Universal Declaration of Human Rights",
        "P0",
    ),
    (
        "ICCPR",
        "human_rights",
        "International Covenant on Civil & Political Rights",
        "P0",
    ),
    (
        "ICESCR",
        "human_rights",
        "International Covenant on Economic, Social & Cultural Rights",
        "P0",
    ),
    (
        "UNDRIP",
        "human_rights",
        "UN Declaration on the Rights of Indigenous Peoples",
        "P0",
    ),
    (
        "CC-BY",
        "creative_commons",
        "Attribution — share, adapt, credit",
        "P0",
    ),
    (
        "CC-BY-SA",
        "creative_commons",
        "Attribution-ShareAlike — derivative works must share alike",
        "P0",
    ),
    (
        "CC-BY-NC",
        "creative_commons",
        "Attribution-NonCommercial — no commercial use",
        "P0",
    ),
    (
        "CC0",
        "creative_commons",
        "No rights reserved — public domain dedication",
        "P0",
    ),
    (
        "COP-R4",
        "permissive_commons",
        "Deontic norms — obligation, prohibition, permission",
        "P0",
    ),
    (
        "COP-M1",
        "permissive_commons",
        "Selfhood — personal data sovereignty",
        "P0",
    ),
    (
        "COP-M3",
        "permissive_commons",
        "Membership — constituent onboarding & roles",
        "P0",
    ),
    (
        "COP-A2",
        "permissive_commons",
        "Agency — DID-signed credentials",
        "P0",
    ),
    (
        "COP-A3",
        "permissive_commons",
        "Authority — governance & consensus thresholds",
        "P0",
    ),
    (
        "COP-C1",
        "permissive_commons",
        "Contribution — fair value & obligation tracking",
        "P0",
    ),
    (
        "COP-C5",
        "permissive_commons",
        "Funding — bounty, escrow, royalty distribution",
        "P0",
    ),
    (
        "COP-P1",
        "permissive_commons",
        "Project taxonomy — work items & phases",
        "P0",
    ),
    (
        "Fiduciary Duty",
        "fiduciary",
        "Legal obligations of custodians & stewards",
        "P1",
    ),
    (
        "Stewardship",
        "fiduciary",
        "Resource stewardship terms",
        "P1",
    ),
    (
        "Escrow Terms",
        "fiduciary",
        "Escrow release conditions & dispute resolution",
        "P1",
    ),
    (
        "Data Sharing",
        "data_governance",
        "Data sharing terms between parties",
        "P1",
    ),
    (
        "Consent Mgmt",
        "data_governance",
        "Consent management & withdrawal terms",
        "P1",
    ),
    (
        "Data Sovereignty",
        "data_governance",
        "Data sovereignty & jurisdictional terms",
        "P1",
    ),
    (
        "Fair Labour",
        "labour",
        "Fair labour standards & contributor covenants",
        "P1",
    ),
    (
        "Code of Conduct",
        "labour",
        "Contributor code of conduct",
        "P1",
    ),
    (
        "Geneva Principles",
        "peace_humanitarian",
        "Geneva Conventions principles for humanitarian ICT",
        "P1",
    ),
    (
        "Do No Harm",
        "peace_humanitarian",
        "Humanitarian access & do-no-harm principles",
        "P1",
    ),
];

const INSTRUMENT_CLASSES: &[(&str, &str)] = &[
    ("human_rights", "Human Rights"),
    ("creative_commons", "Creative Commons"),
    ("permissive_commons", "Permissive Commons (COP)"),
    ("fiduciary", "Fiduciary"),
    ("data_governance", "Data Governance"),
    ("labour", "Labour"),
    ("peace_humanitarian", "Peace & Humanitarian"),
];

const SELECTED_INSTRUMENTS: &[&str] = &["UDHR", "CC-BY-SA", "COP-R4", "COP-C1", "Fair Labour"];

const CLAUSE_OVERLAYS: &[(&str, &str, &str)] = &[
    (
        "Royalty rate override",
        "COP-C5",
        "2x base for uncompensated contributors (justified: project bootstrap phase)",
    ),
    (
        "Data retention period",
        "Data Sharing",
        "7 years post-termination (justified: regulatory compliance)",
    ),
    (
        "Consensus threshold",
        "COP-A3",
        "3-of-5 for stage transitions (justified: multi-stakeholder project)",
    ),
];

const SIGNATORIES: &[(&str, &str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "Principal",
        "signed",
        "2026-08-18",
    ),
    (
        "did:qualia:contributor_02",
        "Contributor",
        "signed",
        "2026-08-18",
    ),
    (
        "did:qualia:contributor_03",
        "Contributor",
        "pending",
        "\u{2014}",
    ),
    ("did:qualia:reviewer_01", "Reviewer", "pending", "\u{2014}"),
    ("did:qualia:reviewer_02", "Reviewer", "pending", "\u{2014}"),
];

const LIFECYCLE_STAGES: &[(&str, &str, &str)] = &[
    ("draft", "Draft", "active"),
    ("review", "Review", "pending"),
    ("sign", "Sign", "pending"),
    ("active", "Active", "pending"),
    ("amend", "Amend", "pending"),
    ("expire", "Expire", "pending"),
    ("terminate", "Terminate", "pending"),
];

const CONFLICTS: &[(&str, &str, &str, &str)] = &[
    (
        "CC-BY-NC",
        "COP-C5 Funding",
        "Non-commercial restriction vs commercial funding distribution",
        "unresolved",
    ),
    (
        "Data Sovereignty",
        "Data Sharing",
        "Jurisdictional data sovereignty vs cross-party sharing",
        "resolved",
    ),
];

pub fn build_agreement_builder_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "padding: 4px 8px; font-size: 10px; color: var(--text-muted); \
         font-family: var(--font-mono); border-bottom: 1px solid var(--border-subtle);",
    );
    header.set_text_content(Some(
        "Agreement composer \u{2014} persist the working instrument set as a COP agreement record.",
    ));
    wrapper.append_child(&header).unwrap();
    let ledger = build_family_panel(
        document,
        "agreement",
        "Saved agreements. Instrument classes below are the COP library, not a fake executed contract.",
        &[
            CopField {
                key: "status",
                placeholder: "draft|review|active",
            },
            CopField {
                key: "instruments",
                placeholder: "Selected instruments (COP-R4, ICCPR, \u{2026})",
            },
            CopField {
                key: "threshold",
                placeholder: "Consensus threshold",
            },
        ],
    );
    ledger
        .append_child(&live_invoke::action_bar(
            document,
            &[(
                "DeonticLogic.evaluate",
                "DeonticLogic.evaluate",
                serde_json::json!({ "modality": "obligate", "body": "agreement" }),
            )],
        ))
        .unwrap();
    wrapper.append_child(&ledger).unwrap();

    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-agreement-tab", tab_id).unwrap();
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
        .append_child(&build_instruments_tab(document))
        .unwrap();

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
        "COP agreement records persist on the daemon. The instrument catalog is the COP library, not an executed contract.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_instruments_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-agreement-panel", "instruments")
        .unwrap();

    for (class_id, class_label) in INSTRUMENT_CLASSES {
        let class_header = document.create_element("div").unwrap();
        class_header.set_text_content(Some(class_label));
        let ch_el: HtmlElement = class_header.clone().dyn_into().unwrap();
        ch_el.style().set_css_text(
            "padding: 4px 0 2px 0; font-size: 10px; font-weight: 600; \
             color: var(--accent-cyan); font-family: var(--font-mono); \
             border-bottom: 1px solid var(--border-subtle); margin-bottom: 4px;",
        );
        panel.append_child(&class_header).unwrap();

        for (id, inst_class, desc, priority) in INSTRUMENTS {
            if inst_class != class_id {
                continue;
            }
            let is_selected = SELECTED_INSTRUMENTS.contains(id);
            let row = document.create_element("div").unwrap();
            let r_el: HtmlElement = row.clone().dyn_into().unwrap();
            let bg = if is_selected {
                "rgba(0, 200, 255, 0.08)"
            } else {
                "transparent"
            };
            let border_color = if is_selected {
                "var(--accent-cyan)"
            } else {
                "var(--border-subtle)"
            };
            r_el.style().set_css_text(&format!(
                "display: flex; align-items: center; gap: 6px; padding: 3px 6px; \
                 margin-bottom: 2px; border: 1px solid {}; border-radius: 3px; \
                 background: {}; cursor: pointer;",
                border_color, bg,
            ));

            let check = document.create_element("span").unwrap();
            check.set_text_content(Some(if is_selected { "\u{2705}" } else { "\u{2B1C}" }));
            let check_el: HtmlElement = check.clone().dyn_into().unwrap();
            check_el
                .style()
                .set_css_text("font-size: 10px; width: 16px; text-align: center;");
            row.append_child(&check).unwrap();

            let name = document.create_element("span").unwrap();
            name.set_text_content(Some(id));
            let n_el: HtmlElement = name.clone().dyn_into().unwrap();
            n_el.style()
                .set_css_text("font-size: 10px; font-weight: 600; color: var(--text-primary); min-width: 100px; font-family: var(--font-mono);");
            row.append_child(&name).unwrap();

            let d = document.create_element("span").unwrap();
            d.set_text_content(Some(desc));
            let d_el: HtmlElement = d.clone().dyn_into().unwrap();
            d_el.style()
                .set_css_text("font-size: 9px; color: var(--text-muted); flex: 1;");
            row.append_child(&d).unwrap();

            let p = document.create_element("span").unwrap();
            p.set_text_content(Some(priority));
            let p_el: HtmlElement = p.clone().dyn_into().unwrap();
            let p_color = match *priority {
                "P0" => "rgba(255, 100, 100, 0.8)",
                "P1" => "rgba(255, 165, 0, 0.8)",
                _ => "var(--text-muted)",
            };
            p_el.style().set_css_text(&format!(
                "font-size: 8px; color: {}; font-family: var(--font-mono);",
                p_color
            ));
            row.append_child(&p).unwrap();

            panel.append_child(&row).unwrap();
        }
    }

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ Add Custom Instrument"));
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
    panel.set_attribute("data-agreement-panel", tab_id).unwrap();

    match tab_id {
        "clauses" => build_clauses_tab(document, &panel),
        "signatories" => build_signatories_tab(document, &panel),
        "lifecycle" => build_lifecycle_tab(document, &panel),
        "conflicts" => build_conflicts_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_clauses_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Clause overlays extend or override instrument defaults. \
         Each override must be explicitly justified and is logged in the provenance chain.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["Clause", "Instrument", "Justification"]);

    let tbody = document.create_element("tbody").unwrap();
    for (clause, instrument, justification) in CLAUSE_OVERLAYS {
        let tr = document.create_element("tr").unwrap();
        for val in &[clause, instrument, justification] {
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
    add_btn.set_text_content(Some("+ Add Clause Overlay"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();
}

fn build_signatories_tab(document: &Document, panel: &Element) {
    let threshold_info = document.create_element("div").unwrap();
    threshold_info.set_text_content(Some(
        "Activation threshold: 3-of-5 (M-of-N). \
         Agreement activates when 3 signers have signed.",
    ));
    let ti_el: HtmlElement = threshold_info.clone().dyn_into().unwrap();
    ti_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--accent-cyan); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&threshold_info).unwrap();

    let table = make_table(document, &["DID", "Role", "Status", "Date"]);

    let tbody = document.create_element("tbody").unwrap();
    for (did, role, status, date) in SIGNATORIES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [did, role, status, date].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match **val {
                    "signed" => "rgba(100, 200, 100, 0.8)",
                    "pending" => "rgba(255, 165, 0, 0.8)",
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

    let progress = document.create_element("div").unwrap();
    progress.set_text_content(Some("Signing progress: 2 / 5 signed \u{2014} 40%"));
    let p_el: HtmlElement = progress.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 8px; font-size: 10px; color: var(--text-secondary); \
         font-family: var(--font-mono); background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&progress).unwrap();

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ Add Signatory"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();
}

fn build_lifecycle_tab(document: &Document, panel: &Element) {
    let stages: Vec<(&str, &str, &str)> = LIFECYCLE_STAGES.iter().copied().collect();

    for (i, (_id, label, status)) in stages.iter().enumerate() {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();

        let (bg, border_c, text_c) = match *status {
            "active" => (
                "rgba(100, 200, 100, 0.08)",
                "rgba(100, 200, 100, 0.4)",
                "rgba(100, 200, 100, 0.9)",
            ),
            "pending" => ("transparent", "var(--border-subtle)", "var(--text-muted)"),
            _ => ("transparent", "var(--border-subtle)", "var(--text-muted)"),
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
            "font-size: 10px; font-weight: 600; color: {}; flex: 1;",
            text_c
        ));
        row.append_child(&name).unwrap();

        let st = document.create_element("span").unwrap();
        st.set_text_content(Some(status));
        let st_el: HtmlElement = st.clone().dyn_into().unwrap();
        st_el.style().set_css_text(&format!(
            "font-size: 9px; color: {}; font-family: var(--font-mono);",
            text_c
        ));
        row.append_child(&st).unwrap();

        panel.append_child(&row).unwrap();
    }

    let transition_btn = document.create_element("button").unwrap();
    transition_btn.set_text_content(Some("\u{2192} Advance to Review Stage"));
    let t_el: HtmlElement = transition_btn.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&transition_btn).unwrap();
}

fn build_conflicts_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Normative conflicts detected between composed instruments. \
         Each conflict must be resolved before agreement activation.",
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
        &["Instrument A", "Instrument B", "Conflict", "Status"],
    );

    let tbody = document.create_element("tbody").unwrap();
    for (a, b, conflict, status) in CONFLICTS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [a, b, conflict, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "resolved" => "rgba(100, 200, 100, 0.8)",
                    "unresolved" => "rgba(255, 100, 100, 0.8)",
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
                     color: var(--text-primary); font-size: 10px;",
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
