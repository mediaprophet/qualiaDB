//! Lab Results — groups tab: expandable lab group to result rows (§3.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

pub(super) const GROUPS: &[(&str, &str, &str, &str)] = &[
    (
        "Full Blood Count",
        "2026-08-15",
        "2026-08-16",
        "ClinicianConfirmed",
    ),
    ("Iron Studies", "2026-08-15", "2026-08-16", "Disputed"),
    (
        "Thyroid Panel",
        "2026-06-01",
        "2026-06-02",
        "ClinicianConfirmed",
    ),
    ("Vitamin D", "2025-11-20", "2025-11-21", "Submitted"),
    (
        "Lipid Panel",
        "2026-01-15",
        "2026-01-16",
        "ClinicianConfirmed",
    ),
    (
        "Liver Function",
        "2026-01-15",
        "2026-01-16",
        "ClinicianConfirmed",
    ),
    ("HbA1c", "2026-03-10", "2026-03-11", "Submitted"),
    ("Vitamin B12", "2025-11-20", "2025-11-21", "Submitted"),
];

pub(super) const RESULTS: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    // Full Blood Count
    (
        "Full Blood Count",
        "Haemoglobin",
        "142",
        "g/L",
        "120-160",
        "normal",
        "LOINC 718-7",
    ),
    (
        "Full Blood Count",
        "White Cell Count",
        "6.8",
        "x10^9/L",
        "4.0-11.0",
        "normal",
        "LOINC 6690-2",
    ),
    (
        "Full Blood Count",
        "Platelets",
        "245",
        "x10^9/L",
        "150-400",
        "normal",
        "LOINC 777-3",
    ),
    (
        "Full Blood Count",
        "RBC",
        "4.8",
        "x10^12/L",
        "4.2-5.4",
        "normal",
        "LOINC 789-8",
    ),
    (
        "Full Blood Count",
        "MCV",
        "88",
        "fL",
        "80-100",
        "normal",
        "LOINC 787-2",
    ),
    (
        "Full Blood Count",
        "MCH",
        "29.6",
        "pg",
        "27-33",
        "normal",
        "LOINC 785-6",
    ),
    // Iron Studies
    (
        "Iron Studies",
        "Ferritin",
        "12",
        "\u{00B5}g/L",
        "30-400",
        "low",
        "LOINC 2276-4",
    ),
    (
        "Iron Studies",
        "Iron",
        "8",
        "\u{00B5}mol/L",
        "10-30",
        "low",
        "LOINC 2502-3",
    ),
    (
        "Iron Studies",
        "Transferrin Sat",
        "12",
        "%",
        "20-50",
        "low",
        "LOINC 2504-9",
    ),
    (
        "Iron Studies",
        "TIBC",
        "67",
        "\u{00B5}mol/L",
        "45-72",
        "normal",
        "LOINC 2503-1",
    ),
    // Thyroid Panel
    (
        "Thyroid Panel",
        "TSH",
        "2.1",
        "mIU/L",
        "0.4-4.0",
        "normal",
        "LOINC 3019-0",
    ),
    (
        "Thyroid Panel",
        "Free T4",
        "14",
        "pmol/L",
        "10-20",
        "normal",
        "LOINC 3024-0",
    ),
    (
        "Thyroid Panel",
        "Free T3",
        "4.8",
        "pmol/L",
        "3.5-6.5",
        "normal",
        "LOINC 3025-7",
    ),
    // Vitamin D
    (
        "Vitamin D",
        "25-OH Vitamin D",
        "28",
        "nmol/L",
        "75-250",
        "low",
        "LOINC 62292-8",
    ),
    // Lipid Panel
    (
        "Lipid Panel",
        "Total Cholesterol",
        "4.9",
        "mmol/L",
        "<5.2",
        "normal",
        "LOINC 2093-3",
    ),
    (
        "Lipid Panel",
        "LDL",
        "2.8",
        "mmol/L",
        "<3.4",
        "normal",
        "LOINC 2085-9",
    ),
    (
        "Lipid Panel",
        "HDL",
        "1.6",
        "mmol/L",
        ">1.0",
        "normal",
        "LOINC 2089-1",
    ),
    (
        "Lipid Panel",
        "Triglycerides",
        "1.1",
        "mmol/L",
        "<1.7",
        "normal",
        "LOINC 2571-8",
    ),
    // Liver Function
    (
        "Liver Function",
        "ALT",
        "24",
        "U/L",
        "10-45",
        "normal",
        "LOINC 1742-6",
    ),
    (
        "Liver Function",
        "AST",
        "22",
        "U/L",
        "10-35",
        "normal",
        "LOINC 1920-8",
    ),
    (
        "Liver Function",
        "GGT",
        "18",
        "U/L",
        "10-45",
        "normal",
        "LOINC 2324-2",
    ),
    (
        "Liver Function",
        "ALP",
        "78",
        "U/L",
        "40-120",
        "normal",
        "LOINC 6768-6",
    ),
    (
        "Liver Function",
        "Bilirubin",
        "9",
        "\u{00B5}mol/L",
        "3-21",
        "normal",
        "LOINC 1975-2",
    ),
    // HbA1c
    (
        "HbA1c",
        "HbA1c",
        "5.2",
        "%",
        "<5.7",
        "normal",
        "LOINC 4548-4",
    ),
    // Vitamin B12
    (
        "Vitamin B12",
        "Vitamin B12",
        "310",
        "pmol/L",
        "150-700",
        "normal",
        "LOINC 2131-4",
    ),
];

pub fn build_groups_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-lab-panel", "groups").unwrap();

    for (group_name, collected, reported, status) in GROUPS {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(
            "border: 1px solid var(--border-subtle); border-radius: 6px; \
             margin-bottom: 6px; background: var(--surface-panel); overflow: hidden;",
        );

        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "padding: 6px 8px; display: flex; justify-content: space-between; \
             align-items: center; cursor: pointer; border-bottom: 1px solid var(--border-subtle);",
        );

        let left = document.create_element("div").unwrap();
        left.set_text_content(Some(&format!(
            "{}  |  Collected: {}  |  Reported: {}",
            group_name, collected, reported
        )));
        let l_el: HtmlElement = left.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-primary); font-family: var(--font-mono);",
        );
        hdr.append_child(&left).unwrap();

        let badge = document.create_element("span").unwrap();
        badge.set_text_content(Some(status));
        let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
        let badge_color = match *status {
            "ClinicianConfirmed" => "rgba(100, 200, 100, 0.8)",
            "Submitted" => "rgba(0, 200, 255, 0.8)",
            "Disputed" => "rgba(255, 100, 100, 0.8)",
            _ => "var(--text-muted)",
        };
        b_el.style().set_css_text(&format!(
            "font-size: 8px; color: {}; font-family: var(--font-mono); \
             font-weight: 600; text-transform: uppercase;",
            badge_color,
        ));
        hdr.append_child(&badge).unwrap();
        card.append_child(&hdr).unwrap();

        let table = make_table(
            document,
            &["Test", "Value", "Unit", "Ref Range", "Flag", "Code"],
        );
        let tbody = document.create_element("tbody").unwrap();
        for (gname, test, value, unit, ref_range, flag, code) in RESULTS {
            if *gname != *group_name {
                continue;
            }
            let tr = document.create_element("tr").unwrap();
            for (i, val) in [test, value, unit, ref_range, flag, code]
                .iter()
                .enumerate()
            {
                let td = document.create_element("td").unwrap();
                td.set_text_content(Some(val));
                let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                if i == 4 {
                    let (color, arrow) = match **val {
                        "normal" => ("rgba(100, 200, 100, 0.8)", ""),
                        "low" => ("rgba(255, 165, 0, 0.8)", " \u{2193}"),
                        "high" => ("rgba(255, 165, 0, 0.8)", " \u{2191}"),
                        "critical" => ("rgba(255, 0, 0, 0.9)", " \u{26A0}"),
                        _ => ("var(--text-primary)", ""),
                    };
                    td.set_text_content(Some(&format!("{}{}", val, arrow)));
                    td_el.style().set_css_text(&format!(
                        "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                         color: {}; font-size: 9px; font-weight: 700;",
                        color,
                    ));
                } else if i == 5 {
                    td_el.style().set_css_text(
                        "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                         color: var(--text-muted); font-size: 8px; \
                         font-family: var(--font-mono);",
                    );
                } else if i == 1 {
                    let value_color = match *flag {
                        "low" | "high" => "rgba(255, 165, 0, 0.9)",
                        "critical" => "rgba(255, 0, 0, 0.9)",
                        _ => "var(--text-primary)",
                    };
                    td_el.style().set_css_text(&format!(
                        "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                         color: {}; font-size: 9px; font-weight: 600; \
                         font-family: var(--font-mono);",
                        value_color,
                    ));
                } else {
                    td_el.style().set_css_text(
                        "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                         color: var(--text-primary); font-size: 9px; \
                         font-family: var(--font-mono);",
                    );
                }
                tr.append_child(&td).unwrap();
            }
            tbody.append_child(&tr).unwrap();
        }
        table.append_child(&tbody).unwrap();
        card.append_child(&table).unwrap();

        panel.append_child(&card).unwrap();
    }

    panel
}

fn make_table(document: &Document, headers: &[&str]) -> Element {
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 9px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 3px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
