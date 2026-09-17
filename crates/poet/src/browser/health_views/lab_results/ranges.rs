//! Lab Results — reference ranges tab: range datasets per test (§3.1).
//!
//! Shows reference range datasets with age/gender context, critical
//! thresholds, LOINC codes, and source authority for each test.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Reference range dataset for a single test.
struct RangeDataset {
    test_name: &'static str,
    loinc: &'static str,
    unit: &'static str,
    age_range: &'static str,
    gender: &'static str,
    ref_low: f64,
    ref_high: f64,
    critical_low: Option<f64>,
    critical_high: Option<f64>,
    source: &'static str,
}

const RANGES: &[RangeDataset] = &[
    RangeDataset {
        test_name: "Haemoglobin",
        loinc: "LOINC 718-7",
        unit: "g/L",
        age_range: "18-65",
        gender: "All",
        ref_low: 120.0,
        ref_high: 160.0,
        critical_low: Some(80.0),
        critical_high: Some(200.0),
        source: "Pathology Lab A",
    },
    RangeDataset {
        test_name: "Haemoglobin",
        loinc: "LOINC 718-7",
        unit: "g/L",
        age_range: "65+",
        gender: "All",
        ref_low: 110.0,
        ref_high: 150.0,
        critical_low: Some(80.0),
        critical_high: Some(200.0),
        source: "Pathology Lab A",
    },
    RangeDataset {
        test_name: "Ferritin",
        loinc: "LOINC 2276-4",
        unit: "\u{00B5}g/L",
        age_range: "18-65",
        gender: "All",
        ref_low: 30.0,
        ref_high: 400.0,
        critical_low: Some(10.0),
        critical_high: None,
        source: "Pathology Lab A",
    },
    RangeDataset {
        test_name: "TSH",
        loinc: "LOINC 3019-0",
        unit: "mIU/L",
        age_range: "18-65",
        gender: "All",
        ref_low: 0.4,
        ref_high: 4.0,
        critical_low: None,
        critical_high: Some(10.0),
        source: "Endocrine Society 2024",
    },
    RangeDataset {
        test_name: "25-OH Vitamin D",
        loinc: "LOINC 62292-8",
        unit: "nmol/L",
        age_range: "18-65",
        gender: "All",
        ref_low: 75.0,
        ref_high: 250.0,
        critical_low: Some(25.0),
        critical_high: None,
        source: "Endocrine Society 2024",
    },
    RangeDataset {
        test_name: "HbA1c",
        loinc: "LOINC 4548-4",
        unit: "%",
        age_range: "All",
        gender: "All",
        ref_low: 0.0,
        ref_high: 5.6,
        critical_low: None,
        critical_high: Some(6.5),
        source: "ADA 2025 Guidelines",
    },
    RangeDataset {
        test_name: "Total Cholesterol",
        loinc: "LOINC 2093-3",
        unit: "mmol/L",
        age_range: "All",
        gender: "All",
        ref_low: 0.0,
        ref_high: 5.2,
        critical_low: None,
        critical_high: Some(6.5),
        source: "Cardiac Society 2024",
    },
    RangeDataset {
        test_name: "LDL",
        loinc: "LOINC 2085-9",
        unit: "mmol/L",
        age_range: "All",
        gender: "All",
        ref_low: 0.0,
        ref_high: 3.4,
        critical_low: None,
        critical_high: Some(4.9),
        source: "Cardiac Society 2024",
    },
    RangeDataset {
        test_name: "ALT",
        loinc: "LOINC 1742-6",
        unit: "U/L",
        age_range: "18-65",
        gender: "All",
        ref_low: 10.0,
        ref_high: 45.0,
        critical_low: None,
        critical_high: Some(200.0),
        source: "Pathology Lab A",
    },
    RangeDataset {
        test_name: "Iron",
        loinc: "LOINC 2502-3",
        unit: "\u{00B5}mol/L",
        age_range: "18-65",
        gender: "All",
        ref_low: 10.0,
        ref_high: 30.0,
        critical_low: Some(5.0),
        critical_high: None,
        source: "Pathology Lab A",
    },
    RangeDataset {
        test_name: "Vitamin B12",
        loinc: "LOINC 2131-4",
        unit: "pmol/L",
        age_range: "All",
        gender: "All",
        ref_low: 150.0,
        ref_high: 700.0,
        critical_low: Some(100.0),
        critical_high: None,
        source: "Pathology Lab A",
    },
    RangeDataset {
        test_name: "Free T4",
        loinc: "LOINC 3024-0",
        unit: "pmol/L",
        age_range: "18-65",
        gender: "All",
        ref_low: 10.0,
        ref_high: 20.0,
        critical_low: Some(5.0),
        critical_high: Some(40.0),
        source: "Endocrine Society 2024",
    },
];

pub fn build_ranges_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-lab-panel", "ranges").unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Reference range datasets sourced from pathology authorities and clinical guidelines. \
         Ranges may vary by age, gender, and methodology. Critical thresholds trigger urgent review.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(
        document,
        &[
            "Test",
            "LOINC",
            "Unit",
            "Age",
            "Gender",
            "Ref Range",
            "Critical",
            "Source",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();

    for r in RANGES {
        let tr = document.create_element("tr").unwrap();

        // Test name
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(r.test_name));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-primary); font-size: 9px; font-weight: 600; \
             font-family: var(--font-mono); white-space: nowrap;",
        );
        tr.append_child(&td).unwrap();

        // LOINC
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(r.loinc));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Unit
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(r.unit));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Age range
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(r.age_range));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-secondary); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Gender
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(r.gender));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-secondary); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Ref range
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(&format!("{} - {}", r.ref_low, r.ref_high)));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: rgba(100, 200, 100, 0.8); font-size: 9px; font-weight: 600; \
             font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Critical thresholds
        let td = document.create_element("td").unwrap();
        let crit_text = match (r.critical_low, r.critical_high) {
            (Some(cl), Some(ch)) => format!("< {} or > {}", cl, ch),
            (Some(cl), None) => format!("< {}", cl),
            (None, Some(ch)) => format!("> {}", ch),
            (None, None) => "\u{2014}".to_string(),
        };
        td.set_text_content(Some(&crit_text));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        let crit_color = if r.critical_low.is_some() || r.critical_high.is_some() {
            "rgba(255, 0, 0, 0.7)"
        } else {
            "var(--text-muted)"
        };
        td_el.style().set_css_text(&format!(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: {}; font-size: 8px; font-family: var(--font-mono);",
            crit_color,
        ));
        tr.append_child(&td).unwrap();

        // Source
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(r.source));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--accent-cyan); font-size: 8px; font-family: var(--font-mono); \
             white-space: nowrap;",
        );
        tr.append_child(&td).unwrap();

        tbody.append_child(&tr).unwrap();
    }

    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    // Legend
    let legend = document.create_element("div").unwrap();
    let l_el: HtmlElement = legend.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "display: flex; gap: 12px; padding: 6px 8px; margin-top: 6px; \
         font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
    );

    for (label, color) in &[
        ("Ref Range", "rgba(100, 200, 100, 0.8)"),
        ("Critical Threshold", "rgba(255, 0, 0, 0.7)"),
        ("Source Authority", "var(--accent-cyan)"),
    ] {
        let item = document.create_element("div").unwrap();
        item.set_text_content(Some(label));
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(&format!("color: {};", color));
        l_el.append_child(&item).unwrap();
    }
    panel.append_child(&legend).unwrap();

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
            "text-align: left; padding: 4px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
