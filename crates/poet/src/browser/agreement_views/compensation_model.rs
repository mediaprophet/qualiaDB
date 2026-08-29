//! Compensation Model — fair value calculator (§8b).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::super::cop_records::{build_family_panel, CopField};

const TABS: &[(&str, &str)] = &[
    ("fair_value", "Fair Value"),
    ("multipliers", "Multipliers"),
    ("royalty", "Royalty Share"),
    ("summary", "Summary"),
];

const CONTRIBUTORS: &[(&str, &str, &str, f64, f64, f64, f64)] = &[
    // (DID, role, skill_level, base_rate, ppp_factor, skill_multiplier, hours)
    (
        "did:qualia:timothy_charles_holborn",
        "Principal",
        "specialist",
        50.0,
        1.2,
        5.0,
        120.0,
    ),
    (
        "did:qualia:contributor_02",
        "Developer",
        "advanced",
        50.0,
        0.8,
        2.0,
        200.0,
    ),
    (
        "did:qualia:contributor_03",
        "Designer",
        "intermediate",
        50.0,
        1.0,
        1.5,
        80.0,
    ),
    (
        "did:qualia:contributor_04",
        "Researcher",
        "expert",
        50.0,
        0.9,
        3.0,
        160.0,
    ),
];

const COMPENSATION_STATUS: &[(&str, &str, f64)] = &[
    // (DID, status, multiplier)
    ("did:qualia:timothy_charles_holborn", "uncompensated", 3.0),
    ("did:qualia:contributor_02", "partially_compensated", 1.5),
    ("did:qualia:contributor_03", "fully_compensated", 1.0),
    ("did:qualia:contributor_04", "uncompensated", 3.0),
];

const STAGE_MULTIPLIERS: &[(&str, f64)] = &[
    ("initiation", 1.0),
    ("planning", 1.2),
    ("execution", 2.0),
    ("review", 1.5),
    ("operation", 1.0),
    ("maintenance", 1.0),
    ("archival", 0.8),
];

const SKILL_LEVELS: &[(&str, f64)] = &[
    ("entry", 1.0),
    ("intermediate", 1.5),
    ("advanced", 2.0),
    ("expert", 3.0),
    ("specialist", 5.0),
];

pub fn build_compensation_model_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    wrapper.append_child(&build_family_panel(
        document,
        "compensation",
        "Persisted contributor compensation records. Stage/skill tables below are the COP-C1 multiplier catalog.",
        &[
            CopField {
                key: "did",
                placeholder: "Contributor DID",
            },
            CopField {
                key: "hours",
                placeholder: "Hours",
            },
            CopField {
                key: "rate",
                placeholder: "Base rate",
            },
            CopField {
                key: "multiplier",
                placeholder: "Skill/PPP multiplier",
            },
            CopField {
                key: "status",
                placeholder: "uncompensated|partial|full",
            },
        ],
    ))
    .unwrap();

    let tab_bar = build_tab_bar(document);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content
        .append_child(&build_fair_value_tab(document))
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
        "COP-C1 compensation records persist on the daemon. Multiplier tables are the catalog, not fabricated payouts.",
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
        tab.set_attribute("data-compensation-tab", tab_id).unwrap();
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

fn build_fair_value_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-compensation-panel", "fair_value")
        .unwrap();

    let formula = document.create_element("div").unwrap();
    formula.set_text_content(Some(
        "Fair Value = base_rate \u{00D7} PPP_factor \u{00D7} skill_multiplier \u{00D7} hours",
    ));
    let f_el: HtmlElement = formula.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "padding: 6px 8px; font-size: 10px; color: var(--accent-cyan); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&formula).unwrap();

    let table = make_table(
        document,
        &[
            "Contributor",
            "Role",
            "Skill",
            "Base/hr",
            "PPP",
            "Skill \u{00D7}",
            "Hours",
            "Fair Value",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    let mut total_fv = 0.0f64;

    for (did, role, skill, base, ppp, skill_mult, hours) in CONTRIBUTORS {
        let fv = base * ppp * skill_mult * hours;
        total_fv += fv;
        let fv_str = format!("{:.0}", fv);
        let base_str = format!("{:.0}", base);
        let ppp_str = format!("{:.1}", ppp);
        let sm_str = format!("{:.1}", skill_mult);
        let hours_str = format!("{:.0}", hours);

        let tr = document.create_element("tr").unwrap();
        for (i, val) in [
            *did,
            *role,
            *skill,
            base_str.as_str(),
            ppp_str.as_str(),
            sm_str.as_str(),
            hours_str.as_str(),
            fv_str.as_str(),
        ]
        .iter()
        .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 7 {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 10px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
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

    let tr = document.create_element("tr").unwrap();
    let td = document.create_element("td").unwrap();
    td.set_text_content(Some("Total Fair Value"));
    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
    td_el.style().set_css_text(
        "padding: 4px 6px; font-weight: 600; color: var(--text-primary); font-size: 10px; \
         font-family: var(--font-mono);",
    );
    tr.append_child(&td).unwrap();
    for _ in 0..6 {
        let td = document.create_element("td").unwrap();
        tr.append_child(&td).unwrap();
    }
    let td = document.create_element("td").unwrap();
    td.set_text_content(Some(&format!("{:.0}", total_fv)));
    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
    td_el.style().set_css_text(
        "padding: 4px 6px; font-weight: 600; color: var(--accent-cyan); font-size: 10px; \
         font-family: var(--font-mono);",
    );
    tr.append_child(&td).unwrap();
    tbody.append_child(&tr).unwrap();

    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-compensation-panel", tab_id)
        .unwrap();

    match tab_id {
        "multipliers" => build_multipliers_tab(document, &panel),
        "royalty" => build_royalty_tab(document, &panel),
        "summary" => build_summary_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_multipliers_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Obligation Cost = fair_value \u{00D7} compensation_multiplier \u{00D7} stage_multiplier \u{00D7} time_factor",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 10px; color: var(--accent-cyan); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let comp_label = document.create_element("div").unwrap();
    comp_label.set_text_content(Some("Compensation Status Multipliers"));
    let cl_el: HtmlElement = comp_label.clone().dyn_into().unwrap();
    cl_el.style().set_css_text(
        "padding: 4px 0 2px 0; font-size: 10px; font-weight: 600; \
         color: var(--accent-cyan); font-family: var(--font-mono); \
         border-bottom: 1px solid var(--border-subtle); margin-bottom: 4px;",
    );
    panel.append_child(&comp_label).unwrap();

    let table = make_table(
        document,
        &["Contributor", "Status", "Multiplier", "Obligation Cost"],
    );
    let tbody = document.create_element("tbody").unwrap();
    let mut total_obligation = 0.0f64;

    for (did, _role, _skill, base, ppp, skill_mult, hours) in CONTRIBUTORS {
        let fv = base * ppp * skill_mult * hours;
        let comp_mult = COMPENSATION_STATUS
            .iter()
            .find(|(d, _, _)| d == did)
            .map(|(_, _, m)| *m)
            .unwrap_or(1.0);
        let stage_mult = 2.0; // execution stage
        let obligation = fv * comp_mult * stage_mult;
        total_obligation += obligation;

        let status = COMPENSATION_STATUS
            .iter()
            .find(|(d, _, _)| d == did)
            .map(|(_, s, _)| *s)
            .unwrap_or("fully_compensated");

        let ob_str = format!("{:.0}", obligation);
        let cm_str = format!("{:.1}x", comp_mult);

        let tr = document.create_element("tr").unwrap();
        for (i, val) in [*did, status, cm_str.as_str(), ob_str.as_str()]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                let color = match *val {
                    "uncompensated" => "rgba(255, 100, 100, 0.8)",
                    "partially_compensated" => "rgba(255, 165, 0, 0.8)",
                    "fully_compensated" => "rgba(100, 200, 100, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color,
                ));
            } else if i == 3 {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 10px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
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

    let tr = document.create_element("tr").unwrap();
    let td = document.create_element("td").unwrap();
    td.set_text_content(Some("Total Obligation Cost"));
    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
    td_el.style().set_css_text(
        "padding: 4px 6px; font-weight: 600; color: var(--text-primary); font-size: 10px; \
         font-family: var(--font-mono);",
    );
    tr.append_child(&td).unwrap();
    for _ in 0..2 {
        let td = document.create_element("td").unwrap();
        tr.append_child(&td).unwrap();
    }
    let td = document.create_element("td").unwrap();
    td.set_text_content(Some(&format!("{:.0}", total_obligation)));
    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
    td_el.style().set_css_text(
        "padding: 4px 6px; font-weight: 600; color: var(--accent-cyan); font-size: 10px; \
         font-family: var(--font-mono);",
    );
    tr.append_child(&td).unwrap();
    tbody.append_child(&tr).unwrap();
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let stage_label = document.create_element("div").unwrap();
    stage_label.set_text_content(Some("Stage-Specific Multipliers"));
    let sl_el: HtmlElement = stage_label.clone().dyn_into().unwrap();
    sl_el.style().set_css_text(
        "padding: 8px 0 2px 0; font-size: 10px; font-weight: 600; \
         color: var(--accent-cyan); font-family: var(--font-mono); \
         border-bottom: 1px solid var(--border-subtle); margin-bottom: 4px;",
    );
    panel.append_child(&stage_label).unwrap();

    let stage_table = make_table(document, &["Stage", "Multiplier"]);
    let stage_tbody = document.create_element("tbody").unwrap();
    for (stage, mult) in STAGE_MULTIPLIERS {
        let tr = document.create_element("tr").unwrap();
        let mult_s = format!("{:.1}x", mult);
        for val in &[*stage, mult_s.as_str()] {
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
        stage_tbody.append_child(&tr).unwrap();
    }
    stage_table.append_child(&stage_tbody).unwrap();
    panel.append_child(&stage_table).unwrap();
}

fn build_royalty_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Royalty share = (fair_value \u{00D7} multiplier \u{00D7} time_factor) / total_weight \u{00D7} 100%",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 10px; color: var(--accent-cyan); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let total_weight: f64 = CONTRIBUTORS
        .iter()
        .map(|(did, _, _, base, ppp, skill_mult, hours)| {
            let fv = base * ppp * skill_mult * hours;
            let comp_mult = COMPENSATION_STATUS
                .iter()
                .find(|(d, _, _)| d == did)
                .map(|(_, _, m)| *m)
                .unwrap_or(1.0);
            fv * comp_mult
        })
        .sum();

    let table = make_table(
        document,
        &["Contributor", "Weight", "Share %", "Royalty Pool"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (did, _, _, base, ppp, skill_mult, hours) in CONTRIBUTORS {
        let fv = base * ppp * skill_mult * hours;
        let comp_mult = COMPENSATION_STATUS
            .iter()
            .find(|(d, _, _)| d == did)
            .map(|(_, _, m)| *m)
            .unwrap_or(1.0);
        let weight = fv * comp_mult;
        let share = (weight / total_weight) * 100.0;

        let w_str = format!("{:.0}", weight);
        let s_str = format!("{:.1}%", share);

        let tr = document.create_element("tr").unwrap();
        for (i, val) in [*did, w_str.as_str(), s_str.as_str(), "pending"]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 10px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
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

fn build_summary_tab(document: &Document, panel: &Element) {
    let total_fv: f64 = CONTRIBUTORS
        .iter()
        .map(|(_, _, _, base, ppp, skill_mult, hours)| base * ppp * skill_mult * hours)
        .sum();

    let total_obligation: f64 = CONTRIBUTORS
        .iter()
        .map(|(did, _, _, base, ppp, skill_mult, hours)| {
            let fv = base * ppp * skill_mult * hours;
            let comp_mult = COMPENSATION_STATUS
                .iter()
                .find(|(d, _, _)| d == did)
                .map(|(_, _, m)| *m)
                .unwrap_or(1.0);
            fv * comp_mult * 2.0 // execution stage
        })
        .sum();

    let total_compensated: f64 = CONTRIBUTORS
        .iter()
        .filter(|(did, _, _, _, _, _, _)| {
            COMPENSATION_STATUS
                .iter()
                .any(|(d, s, _)| d == did && *s == "fully_compensated")
        })
        .map(|(_, _, _, base, ppp, skill_mult, hours)| base * ppp * skill_mult * hours)
        .sum();

    let fv_s = format!("{:.0}", total_fv);
    let ob_s = format!("{:.0}", total_obligation);
    let comp_s = format!("{:.0}", total_compensated);
    let owing_s = format!("{:.0}", total_obligation - total_compensated);
    let count_s = format!("{}", CONTRIBUTORS.len());
    let stats: &[(&str, &str, &str)] = &[
        ("Total Fair Value", fv_s.as_str(), "var(--accent-cyan)"),
        (
            "Total Obligation Cost",
            ob_s.as_str(),
            "rgba(255, 100, 100, 0.8)",
        ),
        (
            "Total Compensated",
            comp_s.as_str(),
            "rgba(100, 200, 100, 0.8)",
        ),
        (
            "Outstanding Obligation",
            owing_s.as_str(),
            "rgba(255, 165, 0, 0.8)",
        ),
        ("Contributors", count_s.as_str(), "var(--text-primary)"),
        ("Current Stage", "execution (2.0x)", "var(--text-primary)"),
    ];

    for (label, value, color) in stats {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; justify-content: space-between; align-items: center; \
             padding: 8px 12px; margin-bottom: 4px; border: 1px solid var(--border-subtle); \
             border-radius: 4px; background: var(--surface-panel);",
        );

        let l = document.create_element("span").unwrap();
        l.set_text_content(Some(label));
        let l_el: HtmlElement = l.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        row.append_child(&l).unwrap();

        let v = document.create_element("span").unwrap();
        v.set_text_content(Some(value));
        let v_el: HtmlElement = v.clone().dyn_into().unwrap();
        v_el.style().set_css_text(&format!(
            "font-size: 12px; font-weight: 600; color: {}; font-family: var(--font-mono);",
            color
        ));
        row.append_child(&v).unwrap();

        panel.append_child(&row).unwrap();
    }

    let skill_label = document.create_element("div").unwrap();
    skill_label.set_text_content(Some("Skill Level Reference"));
    let sl_el: HtmlElement = skill_label.clone().dyn_into().unwrap();
    sl_el.style().set_css_text(
        "padding: 8px 0 2px 0; font-size: 10px; font-weight: 600; \
         color: var(--accent-cyan); font-family: var(--font-mono); \
         border-bottom: 1px solid var(--border-subtle); margin-bottom: 4px;",
    );
    panel.append_child(&skill_label).unwrap();

    let skill_table = make_table(document, &["Level", "Multiplier"]);
    let skill_tbody = document.create_element("tbody").unwrap();
    for (level, mult) in SKILL_LEVELS {
        let tr = document.create_element("tr").unwrap();
        let mult_s = format!("{:.1}x", mult);
        for val in &[*level, mult_s.as_str()] {
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
        skill_tbody.append_child(&tr).unwrap();
    }
    skill_table.append_child(&skill_tbody).unwrap();
    panel.append_child(&skill_table).unwrap();
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
