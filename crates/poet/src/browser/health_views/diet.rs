//! Diet — diet entry log + nutrition analysis (§2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[("log", "Diet Log"), ("nutrition", "Nutrition Analysis")];

const ENTRIES: &[(&str, &str, &str, &str)] = &[
    (
        "Breakfast",
        "2026-08-18 07:30",
        "Oats with berries, coffee",
        "420 kcal",
    ),
    (
        "Lunch",
        "2026-08-18 12:30",
        "Chicken salad, whole grain bread",
        "550 kcal",
    ),
    ("Snack", "2026-08-18 15:00", "Apple, almonds", "180 kcal"),
    (
        "Dinner",
        "2026-08-18 18:30",
        "Salmon, rice, broccoli",
        "700 kcal",
    ),
    (
        "Breakfast",
        "2026-08-17 07:30",
        "Eggs, toast, orange juice",
        "380 kcal",
    ),
    (
        "Dinner",
        "2026-08-17 19:00",
        "Pasta with tomato sauce",
        "620 kcal",
    ),
];

const NUTRITION: &[(&str, &str, &str, &str)] = &[
    ("Calories", "1,850 kcal", "2,200 kcal", "under"),
    ("Protein", "72 g", "80 g", "under"),
    ("Carbohydrates", "210 g", "250 g", "under"),
    ("Fat", "55 g", "70 g", "under"),
    ("Fibre", "28 g", "30 g", "under"),
    ("Iron", "8 mg", "18 mg", "low"),
    ("Vitamin D", "400 IU", "1000 IU", "low"),
    ("Calcium", "900 mg", "1000 mg", "under"),
];

pub fn build_diet_view(document: &Document) -> Element {
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

    content.append_child(&build_log_tab(document)).unwrap();

    let nutrition_panel = build_nutrition_tab(document);
    let np_el: HtmlElement = nutrition_panel.clone().dyn_into().unwrap();
    np_el.style().set_css_text("display: none;");
    content.append_child(&nutrition_panel).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} diet requires wellfare-core/diet + DIET-1..DIET-2 nutrition engine.",
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
        tab.set_attribute("data-diet-tab", tab_id).unwrap();
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

fn build_log_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-diet-panel", "log").unwrap();

    let table = make_table(document, &["Meal", "Time", "Description", "Calories"]);
    let tbody = document.create_element("tbody").unwrap();
    for (meal, time, desc, cal) in ENTRIES {
        let tr = document.create_element("tr").unwrap();
        for val in [meal, time, desc, cal].iter() {
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

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ Log Meal"));
    let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 6px;",
    );
    panel.append_child(&btn).unwrap();

    panel
}

fn build_nutrition_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-diet-panel", "nutrition").unwrap();

    let table = make_table(document, &["Nutrient", "Today", "Target", "Status"]);
    let tbody = document.create_element("tbody").unwrap();
    for (nutrient, today, target, status) in NUTRITION {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [nutrient, today, target, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "low" => "rgba(255, 100, 100, 0.8)",
                    "under" => "rgba(255, 165, 0, 0.8)",
                    "met" => "rgba(100, 200, 100, 0.8)",
                    "over" => "rgba(255, 165, 0, 0.8)",
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
