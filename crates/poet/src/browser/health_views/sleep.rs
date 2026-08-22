//! Sleep — sleep night samples with debt report and weekly heatmap (§2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const SLEEP: &[(&str, &str, &str, &str, &str)] = &[
    ("2026-08-18", "7.5h", "23:30 - 07:00", "good", "8/10"),
    ("2026-08-17", "6.0h", "00:30 - 06:30", "fair", "6/10"),
    ("2026-08-16", "8.2h", "22:45 - 07:00", "excellent", "9/10"),
    ("2026-08-15", "5.5h", "01:00 - 06:30", "poor", "4/10"),
    ("2026-08-14", "7.0h", "23:45 - 06:45", "good", "7/10"),
    ("2026-08-13", "6.5h", "00:15 - 06:45", "fair", "6/10"),
    ("2026-08-12", "4.5h", "02:00 - 06:30", "poor", "3/10"),
];

pub fn build_sleep_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let summary = document.create_element("div").unwrap();
    let s_el: HtmlElement = summary.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; \
         padding: 4px 8px; border-bottom: 1px solid var(--border-subtle);",
    );

    let kpis: &[(&str, &str, &str)] = &[
        ("7d Avg", "6.5h", "var(--text-primary)"),
        ("Cumulative Debt", "2.1h", "rgba(255, 165, 0, 0.8)"),
        ("Chronic Flag", "No", "rgba(100, 200, 100, 0.8)"),
    ];
    for (label, value, color) in kpis {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "text-align: center; padding: 4px; background: var(--surface-panel); \
             border-radius: 4px;",
        );
        let v = document.create_element("div").unwrap();
        v.set_text_content(Some(value));
        let v_el: HtmlElement = v.clone().dyn_into().unwrap();
        v_el.style().set_css_text(&format!(
            "font-size: 14px; font-weight: 700; color: {}; font-family: var(--font-mono);",
            color,
        ));
        card.append_child(&v).unwrap();
        let l = document.create_element("div").unwrap();
        l.set_text_content(Some(label));
        let l_el: HtmlElement = l.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&l).unwrap();
        s_el.append_child(&card).unwrap();
    }
    wrapper.append_child(&summary).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    let max_val: f64 = 8.0;
    for (date, duration, _times, quality, score) in SLEEP {
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
        let dur_f: f64 = duration.trim_end_matches('h').parse().unwrap_or(0.0);
        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        let bar_color = match *quality {
            "excellent" => "rgba(100, 200, 100, 0.5)",
            "good" => "rgba(100, 200, 100, 0.4)",
            "fair" => "rgba(255, 165, 0, 0.4)",
            "poor" => "rgba(255, 100, 100, 0.4)",
            _ => "var(--accent-cyan)",
        };
        b_el.style().set_css_text(&format!(
            "position: absolute; height: 100%; width: {:.0}%; \
             background: {}; border-radius: 2px;",
            (dur_f / max_val) * 100.0,
            bar_color,
        ));
        ba_el.append_child(&bar).unwrap();
        row.append_child(&bar_area).unwrap();

        let vals = document.create_element("div").unwrap();
        vals.set_text_content(Some(&format!("{} {} ({})", duration, quality, score)));
        let v_el: HtmlElement = vals.clone().dyn_into().unwrap();
        v_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 120px; text-align: right;",
        );
        row.append_child(&vals).unwrap();

        content.append_child(&row).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} sleep requires wellfare-core/sleep_analytics.rs + HW-1 telemetry.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
