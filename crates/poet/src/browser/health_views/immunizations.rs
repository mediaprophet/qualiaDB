//! Immunizations — immunization record list (§2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const IMMUNIZATIONS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "COVID-19 (Pfizer)",
        "Dose 3 (Booster)",
        "2024-03-15",
        "City Health Clinic",
        "verified",
    ),
    (
        "Influenza",
        "2025 season",
        "2025-04-20",
        "GP Clinic",
        "verified",
    ),
    (
        "Tetanus/Diphtheria",
        "Booster",
        "2023-06-10",
        "City Health Clinic",
        "verified",
    ),
    (
        "Hepatitis B",
        "Series complete",
        "2018-01-15",
        "Travel Clinic",
        "verified",
    ),
    (
        "MMR",
        "Dose 2",
        "1995-08-01",
        "Childhood Clinic",
        "historical",
    ),
    (
        "COVID-19 (Pfizer)",
        "Dose 1",
        "2021-07-10",
        "City Health Clinic",
        "verified",
    ),
    (
        "COVID-19 (Pfizer)",
        "Dose 2",
        "2021-08-01",
        "City Health Clinic",
        "verified",
    ),
];

pub fn build_immunizations_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    let table = make_table(document, &["Vaccine", "Dose", "Date", "Facility", "Status"]);
    let tbody = document.create_element("tbody").unwrap();
    for (vaccine, dose, date, facility, status) in IMMUNIZATIONS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [vaccine, dose, date, facility, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "verified" => "rgba(100, 200, 100, 0.8)",
                    "historical" => "rgba(255, 165, 0, 0.8)",
                    "pending" => "var(--text-muted)",
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
    content.append_child(&table).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} immunizations require HW-2 immunization engine.",
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
