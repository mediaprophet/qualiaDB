//! Project Dashboard — KPI widgets, health status, activity feed (§2.1.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const KPIS: &[(&str, &str, &str, &str)] = &[
    ("Health", "Good", "green", "On track"),
    ("Budget Burn", "62%", "cyan", "45,000 / 72,000 XEC"),
    ("Task Completion", "68%", "green", "17 / 25 tasks done"),
    ("Team Velocity", "14 pts/wk", "cyan", "Avg over 4 weeks"),
    ("Risk Count", "3 open", "orange", "1 high, 2 medium"),
    ("Milestones", "2 upcoming", "cyan", "Next: 2026-09-01"),
];

const ACTIVITY: &[(&str, &str, &str)] = &[
    (
        "did:qualia:contributor_02",
        "reviewed deliverable DRAFT-003",
        "2026-08-18 10:32",
    ),
    (
        "did:qualia:timothy_charles_holborn",
        "created agreement AGR-002",
        "2026-08-18 09:15",
    ),
    (
        "did:qualia:contributor_03",
        "completed task TASK-018",
        "2026-08-18 08:40",
    ),
    ("system", "auto-checkpoint saved", "2026-08-18 08:00"),
    (
        "did:qualia:contributor_02",
        "filed dispute DSP-001",
        "2026-08-17 16:20",
    ),
    (
        "did:qualia:timothy_charles_holborn",
        "approved resolution RES-002",
        "2026-08-17 15:00",
    ),
    (
        "did:qualia:contributor_03",
        "added contribution to ledger",
        "2026-08-17 14:30",
    ),
];

const MILESTONES: &[(&str, &str, &str)] = &[
    ("Ontology Specification", "2026-08-20", "on_track"),
    ("NLP Pipeline Alpha", "2026-09-01", "at_risk"),
    ("SHACL Shapes Library", "2026-09-15", "on_track"),
    ("Beta Release", "2026-10-01", "not_started"),
];

pub fn build_dashboard_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style().set_css_text(
        "flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 8px;",
    );

    let kpi_grid = build_kpi_grid(document);
    content.append_child(&kpi_grid).unwrap();

    let lower = document.create_element("div").unwrap();
    let l_el: HtmlElement = lower.clone().dyn_into().unwrap();
    l_el.style()
        .set_css_text("display: flex; gap: 8px; flex: 1;");

    let milestones = build_milestones_panel(document);
    lower.append_child(&milestones).unwrap();

    let activity = build_activity_panel(document);
    lower.append_child(&activity).unwrap();

    content.append_child(&lower).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} dashboard aggregates from all project surfaces. \
         Requires COP-P1 lifecycle engine for live data.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_kpi_grid(document: &Document) -> Element {
    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style()
        .set_css_text("display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;");

    for (label, value, color, detail) in KPIS {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        let border_color = match *color {
            "green" => "rgba(100, 200, 100, 0.3)",
            "orange" => "rgba(255, 165, 0, 0.3)",
            "red" => "rgba(255, 100, 100, 0.3)",
            _ => "var(--border-medium)",
        };
        let text_color = match *color {
            "green" => "rgba(100, 200, 100, 0.9)",
            "orange" => "rgba(255, 165, 0, 0.9)",
            "red" => "rgba(255, 100, 100, 0.9)",
            _ => "var(--accent-cyan)",
        };
        c_el.style().set_css_text(&format!(
            "border: 1px solid {}; border-radius: 6px; padding: 8px; \
             background: var(--surface-panel);",
            border_color,
        ));

        let lbl = document.create_element("div").unwrap();
        lbl.set_text_content(Some(label));
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); \
             margin-bottom: 4px;",
        );
        card.append_child(&lbl).unwrap();

        let val = document.create_element("div").unwrap();
        val.set_text_content(Some(value));
        let v_el: HtmlElement = val.clone().dyn_into().unwrap();
        v_el.style().set_css_text(&format!(
            "font-size: 16px; font-weight: 700; color: {}; font-family: var(--font-mono);",
            text_color,
        ));
        card.append_child(&val).unwrap();

        let det = document.create_element("div").unwrap();
        det.set_text_content(Some(detail));
        let d_el: HtmlElement = det.clone().dyn_into().unwrap();
        d_el.style()
            .set_css_text("font-size: 8px; color: var(--text-muted); margin-top: 2px;");
        card.append_child(&det).unwrap();

        grid.append_child(&card).unwrap();
    }

    grid
}

fn build_milestones_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("flex: 1; display: flex; flex-direction: column; gap: 4px;");

    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Upcoming Milestones"));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    panel.append_child(&title).unwrap();

    for (name, date, status) in MILESTONES {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        let (border_c, text_c) = match *status {
            "on_track" => ("rgba(100, 200, 100, 0.3)", "rgba(100, 200, 100, 0.8)"),
            "at_risk" => ("rgba(255, 165, 0, 0.3)", "rgba(255, 165, 0, 0.8)"),
            "delayed" => ("rgba(255, 100, 100, 0.3)", "rgba(255, 100, 100, 0.8)"),
            _ => ("var(--border-subtle)", "var(--text-muted)"),
        };
        r_el.style().set_css_text(&format!(
            "border: 1px solid {}; border-radius: 4px; padding: 4px 6px; \
             background: var(--surface-panel);",
            border_c,
        ));

        let n = document.create_element("div").unwrap();
        n.set_text_content(Some(name));
        let n_el: HtmlElement = n.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 10px; color: var(--text-primary); font-family: var(--font-mono);",
        );
        row.append_child(&n).unwrap();

        let d = document.create_element("div").unwrap();
        d.set_text_content(Some(date));
        let d_el: HtmlElement = d.clone().dyn_into().unwrap();
        d_el.style()
            .set_css_text(&format!("font-size: 8px; color: {};", text_c));
        row.append_child(&d).unwrap();

        panel.append_child(&row).unwrap();
    }

    panel
}

fn build_activity_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("flex: 1; display: flex; flex-direction: column; gap: 2px;");

    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Recent Activity"));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    panel.append_child(&title).unwrap();

    for (actor, action, timestamp) in ACTIVITY {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             font-size: 9px; font-family: var(--font-mono);",
        );

        let a = document.create_element("span").unwrap();
        a.set_text_content(Some(actor));
        let a_el: HtmlElement = a.clone().dyn_into().unwrap();
        a_el.style()
            .set_css_text("color: var(--accent-cyan); margin-right: 4px;");
        row.append_child(&a).unwrap();

        let act = document.create_element("span").unwrap();
        act.set_text_content(Some(action));
        let act_el: HtmlElement = act.clone().dyn_into().unwrap();
        act_el.style().set_css_text("color: var(--text-secondary);");
        row.append_child(&act).unwrap();

        let ts = document.create_element("div").unwrap();
        ts.set_text_content(Some(timestamp));
        let ts_el: HtmlElement = ts.clone().dyn_into().unwrap();
        ts_el
            .style()
            .set_css_text("font-size: 8px; color: var(--text-muted);");
        row.append_child(&ts).unwrap();

        panel.append_child(&row).unwrap();
    }

    panel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_kpis_not_empty() {
        assert!(!KPIS.is_empty());
        for (label, val, color, sub) in KPIS {
            assert!(!label.is_empty());
            assert!(!val.is_empty());
            assert!(!color.is_empty());
            assert!(!sub.is_empty());
        }
    }

    #[test]
    fn test_dashboard_milestones_not_empty() {
        assert!(!MILESTONES.is_empty());
        for (name, date, status) in MILESTONES {
            assert!(!name.is_empty());
            assert!(!date.is_empty());
            assert!(!status.is_empty());
        }
    }
}
