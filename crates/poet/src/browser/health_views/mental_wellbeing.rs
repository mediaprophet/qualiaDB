//! Mental Wellbeing — mood timeline + self-assessment (PHQ-9/GAD-7) (§2, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("mood", "Mood Timeline"),
    ("assessments", "Self-Assessments"),
    ("observations", "Observations"),
];

const MOOD: &[(&str, &str, &str)] = &[
    ("2026-08-18", "7/10", "stable"),
    ("2026-08-17", "6/10", "slightly low"),
    ("2026-08-16", "8/10", "good"),
    ("2026-08-15", "5/10", "low"),
    ("2026-08-14", "6/10", "stable"),
    ("2026-08-13", "7/10", "stable"),
    ("2026-08-12", "4/10", "low"),
];

const ASSESSMENTS: &[(&str, &str, &str, &str, &str)] = &[
    ("PHQ-9", "2026-08-15", "4", "minimal", "none"),
    ("GAD-7", "2026-08-15", "3", "minimal", "none"),
    ("PHQ-9", "2026-07-15", "6", "mild", "none"),
    ("GAD-7", "2026-07-15", "5", "mild", "none"),
    ("PHQ-9", "2026-06-15", "8", "mild", "none"),
    (
        "DASS-21",
        "2026-06-01",
        "Depression: 6, Anxiety: 4, Stress: 8",
        "normal-mild",
        "none",
    ),
];

const OBSERVATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "OBS-001",
        "2026-08-16",
        "Felt energetic after morning walk",
        "self",
    ),
    (
        "OBS-002",
        "2026-08-15",
        "Difficulty concentrating after poor sleep",
        "self",
    ),
    (
        "OBS-003",
        "2026-08-12",
        "Low mood, possible seasonal pattern",
        "self",
    ),
    (
        "OBS-004",
        "2026-08-01",
        "Improved sleep hygiene showing results",
        "self",
    ),
];

pub fn build_mental_wellbeing_view(document: &Document) -> Element {
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

    content.append_child(&build_mood_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} mental wellbeing requires wellfare-core/mental_wellbeing.rs + assessment.rs. \
         If you are in crisis, contact your local crisis line.",
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
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-wellbeing-tab", tab_id).unwrap();
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

fn build_mood_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-wellbeing-panel", "mood").unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Mood timeline: self-reported daily mood score (0-10). \
         Trends inform wellbeing observations and assessment scheduling.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let max_val: f64 = 10.0;
    for (date, score, note) in MOOD {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style()
            .set_css_text("display: flex; align-items: center; gap: 6px; padding: 2px 0;");

        let lbl = document.create_element("div").unwrap();
        lbl.set_text_content(Some(date));
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 80px;",
        );
        row.append_child(&lbl).unwrap();

        let bar_area = document.create_element("div").unwrap();
        let ba_el: HtmlElement = bar_area.clone().dyn_into().unwrap();
        ba_el.style().set_css_text(
            "flex: 1; height: 10px; background: var(--surface-panel); \
             border-radius: 2px; position: relative;",
        );

        let score_f: f64 = score
            .split('/')
            .next()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0);
        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        let bar_color = if score_f >= 7.0 {
            "rgba(100, 200, 100, 0.5)"
        } else if score_f >= 5.0 {
            "rgba(255, 165, 0, 0.5)"
        } else {
            "rgba(255, 100, 100, 0.5)"
        };
        b_el.style().set_css_text(&format!(
            "position: absolute; height: 100%; width: {:.0}%; \
             background: {}; border-radius: 2px;",
            (score_f / max_val) * 100.0,
            bar_color,
        ));
        ba_el.append_child(&bar).unwrap();
        row.append_child(&bar_area).unwrap();

        let vals = document.create_element("div").unwrap();
        vals.set_text_content(Some(&format!("{} ({})", score, note)));
        let v_el: HtmlElement = vals.clone().dyn_into().unwrap();
        v_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 100px; text-align: right;",
        );
        row.append_child(&vals).unwrap();

        panel.append_child(&row).unwrap();
    }

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ Log Mood"));
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
    panel.set_attribute("data-wellbeing-panel", tab_id).unwrap();

    match tab_id {
        "assessments" => {
            let disclaimer = document.create_element("div").unwrap();
            disclaimer.set_text_content(Some(
                "DISCLAIMER: Self-assessment tools are for screening purposes only. \
                 They do not constitute a diagnosis. Consult a qualified clinician for assessment.",
            ));
            let d_el: HtmlElement = disclaimer.clone().dyn_into().unwrap();
            d_el.style().set_css_text(
                "padding: 6px 8px; font-size: 8px; color: rgba(255, 165, 0, 0.8); \
                 font-family: var(--font-mono); margin-bottom: 6px; \
                 background: var(--surface-panel); border-radius: 4px; \
                 border: 1px solid rgba(255, 165, 0, 0.3);",
            );
            panel.append_child(&disclaimer).unwrap();

            let table = make_table(
                document,
                &["Tool", "Date", "Score", "Severity", "Safety Flag"],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (tool, date, score, severity, safety) in ASSESSMENTS {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [tool, date, score, severity, safety].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 3 {
                        let color = match **val {
                            "minimal" => "rgba(100, 200, 100, 0.8)",
                            "mild" => "rgba(255, 165, 0, 0.8)",
                            "moderate" => "rgba(255, 100, 100, 0.8)",
                            "severe" => "rgba(255, 0, 0, 0.9)",
                            _ => "var(--text-primary)",
                        };
                        td_el.style().set_css_text(&format!(
                            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                             color: {}; font-size: 10px; font-weight: 600;",
                            color,
                        ));
                    } else if i == 4 {
                        let color = if **val != "none" {
                            "rgba(255, 0, 0, 0.9)"
                        } else {
                            "var(--text-muted)"
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
            btn.set_text_content(Some("+ Take Assessment"));
            let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
            b_el.style().set_css_text(
                "padding: 4px 12px; border: 1px solid var(--border-medium); \
                 background: transparent; color: var(--text-secondary); border-radius: 3px; \
                 cursor: pointer; font-size: 10px; margin-top: 6px;",
            );
            panel.append_child(&btn).unwrap();
        }
        "observations" => {
            let table = make_table(document, &["ID", "Date", "Note", "Source"]);
            let tbody = document.create_element("tbody").unwrap();
            for (id, date, note, source) in OBSERVATIONS {
                let tr = document.create_element("tr").unwrap();
                for val in [id, date, note, source].iter() {
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
        _ => {}
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
