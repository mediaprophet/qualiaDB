//! Analytics — metrics, KPIs, burndown, velocity, cycle time (§2.10.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("burndown", "Burndown"),
    ("velocity", "Velocity"),
    ("cycle", "Cycle Time"),
    ("budget", "Budget Variance"),
];

const BURNDOWN: &[(&str, &str, &str)] = &[
    ("Week 1", "25", "28"),
    ("Week 2", "20", "22"),
    ("Week 3", "15", "16"),
    ("Week 4", "12", "10"),
    ("Week 5", "8", "6"),
    ("Week 6", "5", "4"),
];

const VELOCITY: &[(&str, &str, &str)] = &[
    ("Week 1", "8", "8"),
    ("Week 2", "10", "9"),
    ("Week 3", "12", "10"),
    ("Week 4", "14", "11"),
    ("Week 5", "14", "12"),
    ("Week 6", "16", "13"),
];

const CYCLE: &[(&str, &str, &str)] = &[
    ("Design", "3.5 days", "2-6 days"),
    ("Build", "5.2 days", "3-12 days"),
    ("Review", "1.8 days", "1-4 days"),
    ("Release", "2.1 days", "1-5 days"),
];

const BUDGET: &[(&str, &str, &str, &str)] = &[
    ("Planning", "10,000", "9,500", "-500 (under)"),
    ("Design", "15,000", "16,200", "+1,200 (over)"),
    ("Build", "30,000", "19,300", "-10,700 (under)"),
    ("Review", "5,000", "3,800", "-1,200 (under)"),
    ("Release", "12,000", "0", "-12,000 (not started)"),
];

pub fn build_analytics_view(document: &Document) -> Element {
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

    content.append_child(&build_burndown_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} analytics derived from all project surfaces. \
         Chart rendering requires canvas/SVG integration.",
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
        tab.set_attribute("data-analytics-tab", tab_id).unwrap();
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

fn build_burndown_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-analytics-panel", "burndown")
        .unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Burndown: remaining work (ideal vs actual). \
         Ideal line is linear from total to zero; actual tracks remaining scope.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let max_val: f64 = 25.0;
    for (week, remaining, ideal) in BURNDOWN {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style()
            .set_css_text("display: flex; align-items: center; gap: 6px; padding: 2px 0;");

        let lbl = document.create_element("div").unwrap();
        lbl.set_text_content(Some(week));
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 60px;",
        );
        row.append_child(&lbl).unwrap();

        let bar_area = document.create_element("div").unwrap();
        let ba_el: HtmlElement = bar_area.clone().dyn_into().unwrap();
        ba_el.style().set_css_text(
            "flex: 1; height: 12px; background: var(--surface-panel); \
             border-radius: 2px; position: relative;",
        );

        let ideal_f: f64 = ideal.parse().unwrap_or(0.0);
        let remaining_f: f64 = remaining.parse().unwrap_or(0.0);

        let ideal_bar = document.create_element("div").unwrap();
        let ib_el: HtmlElement = ideal_bar.clone().dyn_into().unwrap();
        ib_el.style().set_css_text(&format!(
            "position: absolute; height: 100%; width: {:.0}%; \
             background: rgba(255, 165, 0, 0.3); border-radius: 2px;",
            (ideal_f / max_val) * 100.0,
        ));
        ba_el.append_child(&ideal_bar).unwrap();

        let actual_bar = document.create_element("div").unwrap();
        let ab_el: HtmlElement = actual_bar.clone().dyn_into().unwrap();
        ab_el.style().set_css_text(&format!(
            "position: absolute; height: 100%; width: {:.0}%; \
             background: rgba(0, 200, 255, 0.5); border-radius: 2px;",
            (remaining_f / max_val) * 100.0,
        ));
        ba_el.append_child(&actual_bar).unwrap();

        row.append_child(&bar_area).unwrap();

        let vals = document.create_element("div").unwrap();
        vals.set_text_content(Some(&format!("{} / {}", remaining, ideal)));
        let v_el: HtmlElement = vals.clone().dyn_into().unwrap();
        v_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 60px; text-align: right;",
        );
        row.append_child(&vals).unwrap();

        panel.append_child(&row).unwrap();
    }

    let legend = document.create_element("div").unwrap();
    legend.set_text_content(Some(
        "\u{26AB} Actual remaining  |  \u{26AB} Ideal burndown",
    ));
    let lg_el: HtmlElement = legend.clone().dyn_into().unwrap();
    lg_el.style().set_css_text(
        "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
         margin-top: 4px;",
    );
    panel.append_child(&legend).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-analytics-panel", tab_id).unwrap();

    match tab_id {
        "velocity" => {
            let table = make_table(document, &["Week", "Story Points", "Tasks Completed"]);
            let tbody = document.create_element("tbody").unwrap();
            for (week, points, tasks) in VELOCITY {
                let tr = document.create_element("tr").unwrap();
                for val in [week, points, tasks].iter() {
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
        "cycle" => {
            let table = make_table(document, &["Phase", "Avg Cycle Time", "Range"]);
            let tbody = document.create_element("tbody").unwrap();
            for (phase, avg, range) in CYCLE {
                let tr = document.create_element("tr").unwrap();
                for val in [phase, avg, range].iter() {
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
        "budget" => {
            let table = make_table(
                document,
                &["Phase", "Budget (XEC)", "Actual (XEC)", "Variance"],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (phase, budget, actual, variance) in BUDGET {
                let tr = document.create_element("tr").unwrap();
                for (i, val) in [phase, budget, actual, variance].iter().enumerate() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    if i == 3 {
                        let color = if val.contains("over") {
                            "rgba(255, 100, 100, 0.8)"
                        } else if val.contains("not started") {
                            "var(--text-muted)"
                        } else {
                            "rgba(100, 200, 100, 0.8)"
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
