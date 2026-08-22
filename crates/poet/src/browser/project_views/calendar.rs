//! Calendar — month/week/day views for deadlines, meetings, milestones (§2.1.4).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const EVENTS: &[(&str, &str, &str, &str)] = &[
    ("Ontology Spec Due", "2026-08-20", "deadline", "high"),
    ("Governance Meeting", "2026-08-22", "meeting", "normal"),
    ("Review: NLP Pipeline", "2026-08-25", "review", "normal"),
    ("Alpha Release", "2026-09-01", "milestone", "high"),
    ("Funding Review", "2026-09-05", "funding", "normal"),
    ("Beta Release", "2026-10-01", "milestone", "high"),
    ("Obligation Shift", "2026-09-10", "obligation", "low"),
];

pub fn build_calendar_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 4px 8px;",
    );

    let month_label = document.create_element("div").unwrap();
    month_label.set_text_content(Some("August 2026"));
    let m_el: HtmlElement = month_label.clone().dyn_into().unwrap();
    m_el.style().set_css_text(
        "font-size: 12px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    header.append_child(&month_label).unwrap();

    let nav = document.create_element("div").unwrap();
    let n_el: HtmlElement = nav.clone().dyn_into().unwrap();
    n_el.style().set_css_text("display: flex; gap: 4px;");

    for label in &["\u{2190} Prev", "Today", "Next \u{2192}"] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 8px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 9px; font-family: var(--font-mono);",
        );
        nav.append_child(&btn).unwrap();
    }
    header.append_child(&nav).unwrap();
    wrapper.append_child(&header).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px; display: flex; gap: 8px;");

    let grid = build_month_grid(document);
    content.append_child(&grid).unwrap();

    let sidebar = build_events_sidebar(document);
    content.append_child(&sidebar).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} calendar requires COP-X4 events + COP-P5 milestones. \
         iCal export and drag-to-reschedule pending.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_month_grid(document: &Document) -> Element {
    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style()
        .set_css_text("display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; flex: 1;");

    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    for day in &weekdays {
        let hdr = document.create_element("div").unwrap();
        hdr.set_text_content(Some(day));
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); text-align: center; \
             font-family: var(--font-mono); padding: 2px;",
        );
        grid.append_child(&hdr).unwrap();
    }

    let days_with_events: &[u32] = &[20, 22, 25];
    let today: u32 = 18;

    for d in 1..=31 {
        let cell = document.create_element("div").unwrap();
        let c_el: HtmlElement = cell.clone().dyn_into().unwrap();
        let has_event = days_with_events.contains(&d);
        let is_today = d == today;
        let bg = if is_today {
            "rgba(0, 200, 255, 0.15)"
        } else if has_event {
            "rgba(100, 200, 100, 0.08)"
        } else {
            "var(--surface-panel)"
        };
        c_el.style().set_css_text(&format!(
            "min-height: 40px; border: 1px solid var(--border-subtle); \
             border-radius: 3px; padding: 2px; background: {}; \
             display: flex; flex-direction: column; gap: 1px;",
            bg,
        ));

        let num = document.create_element("div").unwrap();
        num.set_text_content(Some(&d.to_string()));
        let n_el: HtmlElement = num.clone().dyn_into().unwrap();
        n_el.style().set_css_text(&format!(
            "font-size: 9px; color: {}; font-family: var(--font-mono);",
            if is_today {
                "var(--accent-cyan)"
            } else {
                "var(--text-muted)"
            },
        ));
        cell.append_child(&num).unwrap();

        if has_event {
            let dot = document.create_element("div").unwrap();
            let d_el: HtmlElement = dot.clone().dyn_into().unwrap();
            d_el.style().set_css_text(
                "width: 4px; height: 4px; border-radius: 50%; \
                 background: rgba(100, 200, 100, 0.8);",
            );
            cell.append_child(&dot).unwrap();
        }

        grid.append_child(&cell).unwrap();
    }

    grid
}

fn build_events_sidebar(document: &Document) -> Element {
    let sidebar = document.create_element("div").unwrap();
    let s_el: HtmlElement = sidebar.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "width: 280px; display: flex; flex-direction: column; gap: 4px; \
         background: var(--surface-panel); border-radius: 6px; padding: 8px;",
    );

    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Upcoming Events"));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    sidebar.append_child(&title).unwrap();

    for (name, date, kind, urgency) in EVENTS {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "padding: 4px 6px; border: 1px solid var(--border-subtle); \
             border-radius: 4px; margin-bottom: 2px;",
        );

        let n = document.create_element("div").unwrap();
        n.set_text_content(Some(name));
        let n_el: HtmlElement = n.clone().dyn_into().unwrap();
        let urgency_color = match *urgency {
            "high" => "rgba(255, 100, 100, 0.9)",
            "normal" => "var(--text-primary)",
            _ => "var(--text-muted)",
        };
        n_el.style().set_css_text(&format!(
            "font-size: 10px; color: {}; font-family: var(--font-mono);",
            urgency_color,
        ));
        row.append_child(&n).unwrap();

        let meta = document.create_element("div").unwrap();
        meta.set_text_content(Some(&format!("{} \u{2014} {}", date, kind)));
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        row.append_child(&meta).unwrap();

        sidebar.append_child(&row).unwrap();
    }

    sidebar
}
