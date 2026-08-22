//! Obligation Tracker — per-asset obligation recovery dashboard (§8c.3.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const ASSETS: &[(&str, &str, f64, f64, f64, &str, &str)] = &[
    // (asset, license, total_obligation, recovered, outstanding, tsl_state, projected)
    (
        "NLP Pipeline v0.1",
        "COP-Permissive",
        8790.0,
        2250.0,
        6540.0,
        "State A",
        "~9 licenses",
    ),
    (
        "Ontology Specification",
        "CC-BY-SA",
        1200.0,
        1200.0,
        0.0,
        "State B",
        "satisfied",
    ),
    (
        "SHACL Shapes",
        "COP-Permissive",
        3200.0,
        0.0,
        3200.0,
        "State A",
        "~5 licenses",
    ),
    (
        "Benchmark Dataset",
        "CC-BY",
        0.0,
        0.0,
        0.0,
        "N/A",
        "no obligation",
    ),
    (
        "Hardware Design",
        "Commercial+Obligation",
        1500.0,
        750.0,
        750.0,
        "State A",
        "~1 license",
    ),
];

pub fn build_obligation_tracker_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let total_obligation: f64 = ASSETS.iter().map(|a| a.2).sum();
    let total_recovered: f64 = ASSETS.iter().map(|a| a.3).sum();
    let total_outstanding: f64 = ASSETS.iter().map(|a| a.4).sum();
    let recovery_pct = if total_obligation > 0.0 {
        (total_recovered / total_obligation) * 100.0
    } else {
        100.0
    };

    let summary = document.create_element("div").unwrap();
    summary.set_text_content(Some(&format!(
        "Project Obligation Recovery: {:.0} / {:.0} sats ({:.1}%) \u{2014} {:.0} outstanding",
        total_recovered, total_obligation, recovery_pct, total_outstanding,
    )));
    let s_el: HtmlElement = summary.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "padding: 6px 8px; font-size: 10px; color: var(--accent-cyan); \
         font-family: var(--font-mono); border-bottom: 1px solid var(--border-subtle); \
         background: var(--surface-panel);",
    );
    wrapper.append_child(&summary).unwrap();

    let progress_bar = document.create_element("div").unwrap();
    let pb_el: HtmlElement = progress_bar.clone().dyn_into().unwrap();
    pb_el.style().set_css_text(
        "height: 6px; background: var(--surface-panel); border-radius: 3px; \
         overflow: hidden; margin: 4px 8px;",
    );

    let fill = document.create_element("div").unwrap();
    let f_el: HtmlElement = fill.clone().dyn_into().unwrap();
    f_el.style().set_css_text(&format!(
        "height: 100%; width: {:.1}%; background: var(--accent-cyan); transition: width 0.3s;",
        recovery_pct,
    ));
    progress_bar.append_child(&fill).unwrap();
    wrapper.append_child(&progress_bar).unwrap();

    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 10px;");

    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in &[
        "Asset",
        "License",
        "Total Obl.",
        "Recovered",
        "Outstanding",
        "TSL State",
        "Projected",
    ] {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 4px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono); white-space: nowrap;",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();

    let tbody = document.create_element("tbody").unwrap();
    for (asset, license, total, recovered, outstanding, tsl, projected) in ASSETS {
        let pct = if *total > 0.0 {
            (*recovered / *total) * 100.0
        } else {
            100.0
        };
        let total_s = format!("{:.0}", total);
        let rec_s = format!("{:.0}", recovered);
        let out_s = format!("{:.0}", outstanding);

        let tr = document.create_element("tr").unwrap();
        for (i, val) in [
            asset,
            license,
            total_s.as_str(),
            rec_s.as_str(),
            out_s.as_str(),
            tsl,
            projected,
        ]
        .iter()
        .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            let color = if i == 5 {
                match *val {
                    "State A" => "rgba(255, 165, 0, 0.8)",
                    "State B" => "rgba(100, 200, 100, 0.8)",
                    _ => "var(--text-muted)",
                }
            } else if i == 4 && *outstanding > 0.0 {
                "rgba(255, 165, 0, 0.8)"
            } else if i == 4 {
                "rgba(100, 200, 100, 0.8)"
            } else if i == 3 && *recovered > 0.0 {
                "rgba(100, 200, 100, 0.8)"
            } else {
                "var(--text-primary)"
            };
            td_el.style().set_css_text(&format!(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                 color: {}; font-size: 10px; font-family: var(--font-mono); white-space: nowrap;",
                color,
            ));
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();

        let bar_tr = document.create_element("tr").unwrap();
        let bar_td = document.create_element("td").unwrap();
        bar_td.set_attribute("colspan", "7").unwrap();
        let bar_container = document.create_element("div").unwrap();
        let bc_el: HtmlElement = bar_container.clone().dyn_into().unwrap();
        bc_el.style().set_css_text(
            "height: 3px; background: var(--surface-panel); border-radius: 2px; \
             overflow: hidden; margin: 0 6px 4px 6px;",
        );
        let bar_fill = document.create_element("div").unwrap();
        let bf_el: HtmlElement = bar_fill.clone().dyn_into().unwrap();
        let bar_color = if pct >= 100.0 {
            "rgba(100, 200, 100, 0.6)"
        } else if pct >= 50.0 {
            "rgba(255, 165, 0, 0.6)"
        } else {
            "rgba(255, 100, 100, 0.4)"
        };
        bf_el.style().set_css_text(&format!(
            "height: 100%; width: {:.1}%; background: {};",
            pct, bar_color,
        ));
        bar_container.append_child(&bar_fill).unwrap();
        bar_td.append_child(&bar_container).unwrap();
        bar_tr.append_child(&bar_td).unwrap();
        tbody.append_child(&bar_tr).unwrap();
    }

    table.append_child(&tbody).unwrap();

    let table_wrapper = document.create_element("div").unwrap();
    let tw_el: HtmlElement = table_wrapper.clone().dyn_into().unwrap();
    tw_el.style().set_css_text("flex: 1; overflow: auto;");
    table_wrapper.append_child(&table).unwrap();
    wrapper.append_child(&table_wrapper).unwrap();

    let tsl_info = document.create_element("div").unwrap();
    tsl_info.set_text_content(Some(
        "TSL State A \u{2192} State B: When obligation is fully recovered, \
         asset shifts from obligation-bearing to share-alike seed.",
    ));
    let ti_el: HtmlElement = tsl_info.clone().dyn_into().unwrap();
    ti_el.style().set_css_text(
        "padding: 4px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&tsl_info).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} obligation tracking requires COP-C1 contribution valuation engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
