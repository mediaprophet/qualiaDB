//! License Builder — differential licensing wizard (§8c).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("composition", "Composition"),
    ("agent_pricing", "Agent Pricing"),
    ("constituents", "Constituents"),
    ("mixed_view", "Mixed View"),
    ("provenance", "Provenance"),
];

const AGENT_TYPES: &[(&str, &str, f64, &str)] = &[
    ("natural_person", "Natural Person (human)", 0.0, "free"),
    (
        "legal_person_small",
        "Small Enterprise",
        150.0,
        "obligation_recovery",
    ),
    (
        "legal_person_medium",
        "Medium Enterprise",
        500.0,
        "obligation_recovery",
    ),
    (
        "legal_person_large",
        "Large Enterprise",
        2000.0,
        "obligation_recovery",
    ),
    ("government", "Government", 1000.0, "obligation_recovery"),
    (
        "humanitarian_org",
        "Humanitarian Organization",
        0.0,
        "waiver",
    ),
    ("research_use", "Research (academic)", 0.0, "free"),
    ("non_commercial", "Non-Commercial (any agent)", 0.0, "free"),
    (
        "commercial_use",
        "Commercial Use (any agent)",
        750.0,
        "obligation_recovery",
    ),
];

const CONSTITUENT_LICENSES: &[(&str, &str, &str, &str)] = &[
    (
        "NLP Pipeline v0.1",
        "deliverable",
        "COP-Permissive",
        "project_default",
    ),
    ("Ontology Specification", "document", "CC-BY-SA", "override"),
    ("SHACL Shapes", "ontology", "COP-Permissive", "inherited"),
    ("Benchmark Dataset", "dataset", "CC-BY", "override"),
    (
        "Source Code Module A",
        "source_code",
        "COP-Permissive",
        "project_default",
    ),
    (
        "Hardware Design",
        "hardware",
        "Commercial+Obligation",
        "override",
    ),
    ("Wiki: Architecture", "wiki_page", "CC-BY-SA", "inherited"),
    ("Research Paper", "publication", "CC-BY-NC", "override"),
];

const LICENSE_INSTRUMENTS: &[(&str, &str)] = &[
    ("COP-Permissive", "Permissive Commons Protocol"),
    ("CC-BY-SA", "Creative Commons Attribution-ShareAlike"),
    ("CC-BY", "Creative Commons Attribution"),
    ("CC-BY-NC", "Creative Commons Attribution-NonCommercial"),
    ("CC0", "Public Domain Dedication"),
    (
        "Commercial+Obligation",
        "Commercial license with obligation recovery",
    ),
];

const PROVENANCE_ENTRIES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "2026-08-01",
        "NLP Pipeline v0.1",
        "COP-Permissive",
        "did:qualia:timothy_charles_holborn",
        "default",
    ),
    (
        "2026-08-03",
        "Ontology Specification",
        "CC-BY-SA",
        "did:qualia:timothy_charles_holborn",
        "override",
    ),
    (
        "2026-08-05",
        "Benchmark Dataset",
        "CC-BY",
        "did:qualia:contributor_04",
        "override",
    ),
    (
        "2026-08-10",
        "Hardware Design",
        "Commercial+Obligation",
        "did:qualia:timothy_charles_holborn",
        "override",
    ),
    (
        "2026-08-12",
        "Research Paper",
        "CC-BY-NC",
        "did:qualia:contributor_02",
        "override",
    ),
];

pub fn build_license_builder_view(document: &Document) -> Element {
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
        .append_child(&build_composition_tab(document))
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
        "\u{26A0} Mock data \u{2014} license composition requires COP-R3 licensing engine command.",
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
        tab.set_attribute("data-license-tab", tab_id).unwrap();
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

fn build_composition_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-license-panel", "composition")
        .unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "License = instrument(s) + agent-type pricing + obligation recovery terms + TSL parameters + waiver policy",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 10px; color: var(--accent-cyan); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let label = document.create_element("div").unwrap();
    label.set_text_content(Some("License Instruments"));
    let l_el: HtmlElement = label.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "padding: 4px 0 2px 0; font-size: 10px; font-weight: 600; \
         color: var(--accent-cyan); font-family: var(--font-mono); \
         border-bottom: 1px solid var(--border-subtle); margin-bottom: 4px;",
    );
    panel.append_child(&label).unwrap();

    for (id, desc) in LICENSE_INSTRUMENTS {
        let is_selected = id == &"COP-Permissive";
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        let border_c = if is_selected {
            "var(--accent-cyan)"
        } else {
            "var(--border-subtle)"
        };
        let bg = if is_selected {
            "rgba(0, 200, 255, 0.08)"
        } else {
            "transparent"
        };
        r_el.style().set_css_text(&format!(
            "display: flex; align-items: center; gap: 6px; padding: 3px 6px; \
             margin-bottom: 2px; border: 1px solid {}; border-radius: 3px; \
             background: {}; cursor: pointer;",
            border_c, bg,
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
            .set_css_text("font-size: 10px; font-weight: 600; color: var(--text-primary); min-width: 140px; font-family: var(--font-mono);");
        row.append_child(&name).unwrap();

        let d = document.create_element("span").unwrap();
        d.set_text_content(Some(desc));
        let d_el: HtmlElement = d.clone().dyn_into().unwrap();
        d_el.style()
            .set_css_text("font-size: 9px; color: var(--text-muted); flex: 1;");
        row.append_child(&d).unwrap();

        panel.append_child(&row).unwrap();
    }

    let tsl_label = document.create_element("div").unwrap();
    tsl_label.set_text_content(Some("TSL Parameters"));
    let tl_el: HtmlElement = tsl_label.clone().dyn_into().unwrap();
    tl_el.style().set_css_text(
        "padding: 8px 0 2px 0; font-size: 10px; font-weight: 600; \
         color: var(--accent-cyan); font-family: var(--font-mono); \
         border-bottom: 1px solid var(--border-subtle); margin-bottom: 4px;",
    );
    panel.append_child(&tsl_label).unwrap();

    let tsl_items = &[
        (
            "State A (Obligation-bearing)",
            "Active \u{2014} obligation cost accrues until recovery",
        ),
        (
            "State B (Share-alike seed)",
            "Triggered when obligation fully recovered \u{2192} TSL shift",
        ),
        ("Recovery target", "8,790 sats (total obligation cost)"),
        ("Recovery rate", "750 sats per commercial license"),
        ("Projected satisfaction", "~12 commercial licenses"),
    ];

    for (label, value) in tsl_items {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; justify-content: space-between; padding: 4px 8px; \
             margin-bottom: 2px; border: 1px solid var(--border-subtle); \
             border-radius: 3px; background: var(--surface-panel);",
        );

        let l = document.create_element("span").unwrap();
        l.set_text_content(Some(label));
        let l_el: HtmlElement = l.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        row.append_child(&l).unwrap();

        let v = document.create_element("span").unwrap();
        v.set_text_content(Some(value));
        let v_el: HtmlElement = v.clone().dyn_into().unwrap();
        v_el.style().set_css_text(
            "font-size: 9px; color: var(--text-primary); font-family: var(--font-mono);",
        );
        row.append_child(&v).unwrap();

        panel.append_child(&row).unwrap();
    }

    let preview_btn = document.create_element("button").unwrap();
    preview_btn.set_text_content(Some("\u{1F441} Preview License (Human-Readable)"));
    let p_el: HtmlElement = preview_btn.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&preview_btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-license-panel", tab_id).unwrap();

    match tab_id {
        "agent_pricing" => build_agent_pricing_tab(document, &panel),
        "constituents" => build_constituents_tab(document, &panel),
        "mixed_view" => build_mixed_view_tab(document, &panel),
        "provenance" => build_provenance_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_agent_pricing_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Agent-type-based pricing: natural persons free, legal persons pay obligation recovery rate. \
         Humanitarian organizations may apply for waiver.",
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
        &["Agent Type", "Description", "Rate (sats)", "Pricing Model"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, desc, rate, model) in AGENT_TYPES {
        let rate_s = if *rate == 0.0 {
            "free".to_string()
        } else {
            format!("{:.0}", rate)
        };
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, desc, rate_s.as_str(), model].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match *val {
                    "free" => "rgba(100, 200, 100, 0.8)",
                    "obligation_recovery" => "rgba(255, 165, 0, 0.8)",
                    "waiver" => "rgba(0, 200, 255, 0.8)",
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

fn build_constituents_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Per-constituent licensing: each artefact may have a different license. \
         Project default is applied unless overridden. Overrides are logged with provenance.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["Constituent", "Type", "License", "Scope"]);
    let tbody = document.create_element("tbody").unwrap();
    for (name, ctype, license, scope) in CONSTITUENT_LICENSES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, ctype, license, scope].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "project_default" => "var(--text-muted)",
                    "override" => "rgba(255, 165, 0, 0.8)",
                    "inherited" => "rgba(0, 200, 255, 0.8)",
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

    let bulk_btn = document.create_element("button").unwrap();
    bulk_btn.set_text_content(Some("+ Bulk Assign License"));
    let b_el: HtmlElement = bulk_btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&bulk_btn).unwrap();
}

fn build_mixed_view_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Mixed-licensing view: all licenses across constituents. \
         Highlights where constituents have divergent licensing.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let mut license_counts: Vec<(&str, usize)> = Vec::new();
    for (_, _, license, _) in CONSTITUENT_LICENSES {
        let count = CONSTITUENT_LICENSES
            .iter()
            .filter(|(_, _, l, _)| l == license)
            .count();
        if !license_counts.iter().any(|(l, _)| l == license) {
            license_counts.push((license, count));
        }
    }

    for (license, count) in &license_counts {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; justify-content: space-between; align-items: center; \
             padding: 6px 12px; margin-bottom: 4px; border: 1px solid var(--border-subtle); \
             border-radius: 4px; background: var(--surface-panel);",
        );

        let l = document.create_element("span").unwrap();
        l.set_text_content(Some(license));
        let l_el: HtmlElement = l.clone().dyn_into().unwrap();
        l_el.style()
            .set_css_text("font-size: 10px; font-weight: 600; color: var(--text-primary); font-family: var(--font-mono);");
        row.append_child(&l).unwrap();

        let c = document.create_element("span").unwrap();
        c.set_text_content(Some(&format!("{} constituents", count)));
        let c_el: HtmlElement = c.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "font-size: 10px; color: var(--accent-cyan); font-family: var(--font-mono);",
        );
        row.append_child(&c).unwrap();

        panel.append_child(&row).unwrap();
    }

    let conflict_warning = document.create_element("div").unwrap();
    conflict_warning.set_text_content(Some(
        "\u{26A0} License conflict detected: CC-BY-NC (Research Paper) vs Commercial+Obligation (Hardware Design) \u{2014} incompatible commercial terms",
    ));
    let cw_el: HtmlElement = conflict_warning.clone().dyn_into().unwrap();
    cw_el.style().set_css_text(
        "margin-top: 6px; padding: 6px 8px; font-size: 9px; color: rgba(255, 100, 100, 0.8); \
         font-family: var(--font-mono); background: rgba(255, 100, 100, 0.05); \
         border: 1px solid rgba(255, 100, 100, 0.2); border-radius: 4px;",
    );
    panel.append_child(&conflict_warning).unwrap();
}

fn build_provenance_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "License provenance chain: append-only history of all license assignments per constituent.",
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
        &["Date", "Constituent", "License", "Assigned By", "Reason"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (date, constituent, license, assigned_by, reason) in PROVENANCE_ENTRIES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [date, constituent, license, assigned_by, reason]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "default" => "var(--text-muted)",
                    "override" => "rgba(255, 165, 0, 0.8)",
                    "inherited" => "rgba(0, 200, 255, 0.8)",
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
