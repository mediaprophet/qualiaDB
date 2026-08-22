//! Data Sources — external data source registry & dataset management (§8l).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("sources", "Sources"),
    ("datasets", "Datasets"),
    ("evaluations", "Evaluations"),
    ("comparison", "Comparison"),
];

const DATA_SOURCES: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "PPP Indices",
        "OECD",
        "economic",
        "CSV",
        "quarterly",
        "public",
    ),
    (
        "PPP Indices",
        "World Bank",
        "economic",
        "JSON",
        "annual",
        "public",
    ),
    ("PPP Indices", "IMF", "economic", "XML", "annual", "public"),
    (
        "Exchange Rates",
        "BIS",
        "economic",
        "CSV",
        "daily",
        "public",
    ),
    (
        "Exchange Rates",
        "ECB",
        "economic",
        "JSON",
        "daily",
        "public",
    ),
    (
        "Exchange Rates",
        "Federal Reserve",
        "economic",
        "CSV",
        "daily",
        "public",
    ),
    (
        "Human Development Index",
        "UNDP",
        "social",
        "CSV",
        "annual",
        "public",
    ),
    (
        "Gini Coefficient",
        "World Bank",
        "social",
        "JSON",
        "annual",
        "public",
    ),
    (
        "Climate Data",
        "NOAA",
        "scientific",
        "JSON",
        "monthly",
        "public",
    ),
    (
        "Health Indicators",
        "WHO",
        "health",
        "JSON",
        "monthly",
        "public",
    ),
    (
        "Population Data",
        "UN Population Division",
        "demographic",
        "CSV",
        "annual",
        "public",
    ),
    (
        "Standards References",
        "ISO",
        "standards",
        "PDF",
        "irregular",
        "subscription",
    ),
    (
        "Geospatial",
        "OpenStreetMap",
        "geographic",
        "RDF",
        "real-time",
        "public",
    ),
    (
        "Crypto Price Feeds",
        "CoinGecko",
        "economic",
        "JSON",
        "real-time",
        "public",
    ),
    (
        "Lightning Network Stats",
        "1ML",
        "economic",
        "JSON",
        "daily",
        "public",
    ),
    (
        "Commons Artefact Registry",
        "QualiaDB",
        "permissive_commons",
        "CBOR-LD",
        "irregular",
        "public",
    ),
];

const DATASETS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "PPP 2024 Release",
        "OECD",
        "2024-01-01 to 2024-12-31",
        "global",
        "CC-BY",
        "Public",
    ),
    (
        "PPP 2024 Release",
        "World Bank",
        "2024-01-01 to 2024-12-31",
        "global",
        "CC-BY",
        "Public",
    ),
    (
        "Benchmark Dataset v1",
        "Internal",
        "2026-08-01 to 2026-08-15",
        "project",
        "CC-BY",
        "Public",
    ),
    (
        "Climate Indicators 2024",
        "NOAA",
        "2024-01-01 to 2024-12-31",
        "global",
        "CC0",
        "Public",
    ),
    (
        "Contributor Survey Data",
        "Internal",
        "2026-07-01 to 2026-07-31",
        "project",
        "Restricted",
        "Restricted",
    ),
    (
        "Ontology Corpus",
        "Internal",
        "2026-06-01 to 2026-08-18",
        "project",
        "COP-Permissive",
        "Public",
    ),
];

const EVALUATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "PPP-Adjusted Compensation",
        "OECD PPP 2024",
        "quarterly",
        "2026-08-18",
    ),
    (
        "PPP-Adjusted Compensation",
        "World Bank PPP 2024",
        "quarterly",
        "2026-08-18",
    ),
    (
        "Obligation Recovery Rate",
        "Internal Ledger",
        "monthly",
        "2026-08-16",
    ),
    (
        "Budget Variance",
        "Internal Budget",
        "monthly",
        "2026-08-15",
    ),
    (
        "Contribution Distribution Equity",
        "Gini World Bank",
        "annual",
        "2026-08-01",
    ),
];

const COMPARISON: &[(&str, &str, &str, &str)] = &[
    (
        "PPP Indices",
        "OECD",
        "World Bank",
        "OECD uses updated methodology; World Bank uses Atlas method",
    ),
    (
        "PPP Indices",
        "OECD",
        "IMF",
        "OECD covers 57 countries; IMF covers 190 but less frequently",
    ),
    (
        "Exchange Rates",
        "BIS",
        "ECB",
        "BIS includes crypto pairs; ECB focuses on EUR zone",
    ),
];

pub fn build_data_sources_view(document: &Document) -> Element {
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

    content.append_child(&build_sources_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} data source registry requires COP-X2 external data engine command.",
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
        tab.set_attribute("data-datasrc-tab", tab_id).unwrap();
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

fn build_sources_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-datasrc-panel", "sources")
        .unwrap();

    let table = make_table(
        document,
        &[
            "Name",
            "Publisher",
            "Category",
            "Format",
            "Frequency",
            "Access",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (name, publisher, category, format, frequency, access) in DATA_SOURCES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, publisher, category, format, frequency, access]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 5 {
                let color = match **val {
                    "public" => "rgba(100, 200, 100, 0.8)",
                    "subscription" => "rgba(255, 165, 0, 0.8)",
                    "api_key" => "rgba(0, 200, 255, 0.8)",
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

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ Register Data Source"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-datasrc-panel", tab_id).unwrap();

    match tab_id {
        "datasets" => build_datasets_tab(document, &panel),
        "evaluations" => build_evaluations_tab(document, &panel),
        "comparison" => build_comparison_tab(document, &panel),
        _ => {}
    }

    panel
}

fn build_datasets_tab(document: &Document, panel: &Element) {
    let table = make_table(
        document,
        &[
            "Dataset",
            "Source",
            "Date Range",
            "Coverage",
            "License",
            "Sensitivity",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (name, source, date_range, coverage, license, sensitivity) in DATASETS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, source, date_range, coverage, license, sensitivity]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 5 {
                let color = match **val {
                    "Public" => "rgba(100, 200, 100, 0.8)",
                    "Restricted" => "rgba(255, 165, 0, 0.8)",
                    "Classified" => "rgba(255, 100, 100, 0.8)",
                    "Selfhood" => "rgba(255, 0, 0, 0.9)",
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
}

fn build_evaluations_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Evaluations reference exact source versions. Reproducible: re-run with same versions to verify results.",
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
        &["Evaluation", "Input Source", "Frequency", "Last Run"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (name, input, frequency, last_run) in EVALUATIONS {
        let tr = document.create_element("tr").unwrap();
        for val in [name, input, frequency, last_run].iter() {
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

fn build_comparison_tab(document: &Document, panel: &Element) {
    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Side-by-side comparison of sources covering the same metric.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(document, &["Metric", "Source A", "Source B", "Difference"]);
    let tbody = document.create_element("tbody").unwrap();
    for (metric, a, b, diff) in COMPARISON {
        let tr = document.create_element("tr").unwrap();
        for val in [metric, a, b, diff].iter() {
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
