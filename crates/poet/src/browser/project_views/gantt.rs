//! Gantt — interactive Gantt chart with task bars and dependencies (§2.1.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("chart", "Gantt Chart"),
    ("dependencies", "Dependencies"),
    ("critical", "Critical Path"),
];

const TASKS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "Ontology Spec",
        "2026-07-15",
        "2026-08-20",
        "Design",
        "100%",
        "done",
    ),
    (
        "NLP Pipeline",
        "2026-08-01",
        "2026-09-01",
        "Build",
        "60%",
        "in_progress",
    ),
    (
        "SHACL Library",
        "2026-08-05",
        "2026-09-15",
        "Build",
        "40%",
        "in_progress",
    ),
    (
        "FST Engine",
        "2026-08-12",
        "2026-09-10",
        "Build",
        "25%",
        "in_progress",
    ),
    (
        "Alpha Release",
        "2026-09-01",
        "2026-09-15",
        "Release",
        "0%",
        "not_started",
    ),
    (
        "Beta Release",
        "2026-10-01",
        "2026-10-15",
        "Release",
        "0%",
        "not_started",
    ),
];

const DEPS: &[(&str, &str, &str)] = &[
    ("Ontology Spec", "NLP Pipeline", "FS"),
    ("Ontology Spec", "SHACL Library", "FS"),
    ("NLP Pipeline", "Alpha Release", "FS"),
    ("SHACL Library", "Alpha Release", "FS"),
    ("FST Engine", "Alpha Release", "SS"),
    ("Alpha Release", "Beta Release", "FS"),
];

const CRITICAL: &[(&str, &str, &str, &str)] = &[
    ("Ontology Spec", "2026-07-15", "2026-08-20", "36 days"),
    ("NLP Pipeline", "2026-08-01", "2026-09-01", "31 days"),
    ("Alpha Release", "2026-09-01", "2026-09-15", "14 days"),
    ("Beta Release", "2026-10-01", "2026-10-15", "14 days"),
];

pub fn build_gantt_view(document: &Document) -> Element {
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

    content.append_child(&build_chart_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} Gantt requires COP-P4 dependency engine. \
         Drag-to-reschedule needs interactive timeline canvas.",
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
        tab.set_attribute("data-gantt-tab", tab_id).unwrap();
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

fn build_chart_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-gantt-panel", "chart").unwrap();

    for (name, start, end, phase, progress, status) in TASKS {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; align-items: center; gap: 6px; padding: 4px 0; \
             border-bottom: 1px solid var(--border-subtle);",
        );

        let lbl = document.create_element("div").unwrap();
        lbl.set_text_content(Some(name));
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-primary); font-family: var(--font-mono); \
             min-width: 140px;",
        );
        row.append_child(&lbl).unwrap();

        let bar_container = document.create_element("div").unwrap();
        let bc_el: HtmlElement = bar_container.clone().dyn_into().unwrap();
        bc_el.style().set_css_text(
            "flex: 1; height: 16px; background: var(--surface-panel); \
             border-radius: 3px; position: relative; overflow: hidden;",
        );

        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        let (color, opacity) = match *status {
            "done" => ("rgba(100, 200, 100, 0.6)", "1.0"),
            "in_progress" => ("rgba(0, 200, 255, 0.5)", "1.0"),
            _ => ("var(--border-medium)", "0.5"),
        };
        b_el.style().set_css_text(&format!(
            "height: 100%; width: {}; background: {}; border-radius: 3px; \
             display: flex; align-items: center; padding: 0 4px;",
            progress, color,
        ));

        let p_text = document.create_element("span").unwrap();
        p_text.set_text_content(Some(progress));
        let pt_el: HtmlElement = p_text.clone().dyn_into().unwrap();
        pt_el.style().set_css_text(&format!(
            "font-size: 8px; color: var(--text-primary); opacity: {}; \
             font-family: var(--font-mono);",
            opacity,
        ));
        bar.append_child(&p_text).unwrap();
        bc_el.append_child(&bar).unwrap();
        row.append_child(&bar_container).unwrap();

        let dates = document.create_element("div").unwrap();
        dates.set_text_content(Some(&format!("{} \u{2192} {}", start, end)));
        let d_el: HtmlElement = dates.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 160px; text-align: right;",
        );
        row.append_child(&dates).unwrap();

        let ph = document.create_element("div").unwrap();
        ph.set_text_content(Some(phase));
        let ph_el: HtmlElement = ph.clone().dyn_into().unwrap();
        ph_el.style().set_css_text(
            "font-size: 8px; color: var(--accent-cyan); font-family: var(--font-mono); \
             min-width: 60px;",
        );
        row.append_child(&ph).unwrap();

        panel.append_child(&row).unwrap();
    }

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-gantt-panel", tab_id).unwrap();

    match tab_id {
        "dependencies" => {
            let table = make_table(document, &["From", "To", "Type"]);
            let tbody = document.create_element("tbody").unwrap();
            for (from, to, dep_type) in DEPS {
                let tr = document.create_element("tr").unwrap();
                for val in [from, to, dep_type].iter() {
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
        "critical" => {
            let info = document.create_element("div").unwrap();
            info.set_text_content(Some(
                "Critical path: longest chain of dependent tasks. \
                 Any delay on critical path delays the project end date.",
            ));
            let i_el: HtmlElement = info.clone().dyn_into().unwrap();
            i_el.style().set_css_text(
                "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
                 font-family: var(--font-mono); margin-bottom: 6px; \
                 background: var(--surface-panel); border-radius: 4px;",
            );
            panel.append_child(&info).unwrap();

            let table = make_table(document, &["Task", "Start", "End", "Duration"]);
            let tbody = document.create_element("tbody").unwrap();
            for (name, start, end, dur) in CRITICAL {
                let tr = document.create_element("tr").unwrap();
                for val in [name, start, end, dur].iter() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gantt_tasks_not_empty() {
        assert!(!TASKS.is_empty());
        for (name, start, end, phase, pct, status) in TASKS {
            assert!(!name.is_empty());
            assert!(!start.is_empty());
            assert!(!end.is_empty());
            assert!(!phase.is_empty());
            assert!(!pct.is_empty());
            assert!(!status.is_empty());
        }
    }

    #[test]
    fn test_gantt_tabs_complete() {
        assert_eq!(TABS.len(), 3);
        assert_eq!(TABS[0].0, "chart");
        assert_eq!(TABS[1].0, "dependencies");
        assert_eq!(TABS[2].0, "critical");
    }
}
