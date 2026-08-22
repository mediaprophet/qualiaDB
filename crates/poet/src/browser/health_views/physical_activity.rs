//! Physical Activity — activity log (§2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const ACTIVITIES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Walking",
        "2026-08-18 07:00",
        "30 min",
        "2.5 km",
        "180 kcal",
    ),
    ("Cycling", "2026-08-17 17:00", "45 min", "12 km", "320 kcal"),
    (
        "Walking",
        "2026-08-16 07:00",
        "45 min",
        "3.8 km",
        "270 kcal",
    ),
    (
        "Swimming",
        "2026-08-15 18:00",
        "40 min",
        "1.5 km",
        "400 kcal",
    ),
    (
        "Walking",
        "2026-08-14 07:00",
        "30 min",
        "2.5 km",
        "180 kcal",
    ),
    ("Yoga", "2026-08-13 20:00", "60 min", "—", "200 kcal"),
];

pub fn build_physical_activity_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let summary = document.create_element("div").unwrap();
    summary.set_text_content(Some(
        "7d total: 250 min active, 1,550 kcal burned, 4 sessions",
    ));
    let s_el: HtmlElement = summary.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "padding: 4px 8px; font-size: 10px; color: var(--text-primary); \
         font-family: var(--font-mono); background: var(--surface-panel); \
         border-radius: 4px; margin: 4px 8px; border-bottom: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&summary).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    let table = make_table(
        document,
        &["Activity", "Date/Time", "Duration", "Distance", "Calories"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (activity, dt, duration, distance, cal) in ACTIVITIES {
        let tr = document.create_element("tr").unwrap();
        for val in [activity, dt, duration, distance, cal].iter() {
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
    content.append_child(&table).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} physical activity requires HW-5 activity engine.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
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
