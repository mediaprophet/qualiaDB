//! Contribution Ledger — append-only contribution records (§8b.3.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const ENTRIES: &[(&str, &str, &str, f64, f64, f64, f64, f64, f64)] = &[
    // (date, DID, contribution_type, quantity, fair_value, multiplier, obligation_cost, actual_comp, balance_owing)
    (
        "2026-08-01",
        "did:qualia:timothy_charles_holborn",
        "time",
        8.0,
        480.0,
        3.0,
        1440.0,
        0.0,
        1440.0,
    ),
    (
        "2026-08-02",
        "did:qualia:contributor_02",
        "time",
        8.0,
        128.0,
        1.5,
        192.0,
        64.0,
        128.0,
    ),
    (
        "2026-08-03",
        "did:qualia:contributor_02",
        "expertise",
        4.0,
        320.0,
        1.5,
        480.0,
        0.0,
        480.0,
    ),
    (
        "2026-08-05",
        "did:qualia:contributor_03",
        "time",
        6.0,
        90.0,
        1.0,
        90.0,
        90.0,
        0.0,
    ),
    (
        "2026-08-07",
        "did:qualia:contributor_04",
        "time",
        8.0,
        216.0,
        3.0,
        648.0,
        0.0,
        648.0,
    ),
    (
        "2026-08-10",
        "did:qualia:timothy_charles_holborn",
        "skill",
        4.0,
        1200.0,
        3.0,
        3600.0,
        0.0,
        3600.0,
    ),
    (
        "2026-08-12",
        "did:qualia:contributor_02",
        "time",
        8.0,
        128.0,
        1.5,
        192.0,
        64.0,
        128.0,
    ),
    (
        "2026-08-14",
        "did:qualia:contributor_04",
        "resource",
        1.0,
        500.0,
        3.0,
        1500.0,
        0.0,
        1500.0,
    ),
    (
        "2026-08-15",
        "did:qualia:contributor_03",
        "time",
        6.0,
        90.0,
        1.0,
        90.0,
        90.0,
        0.0,
    ),
    (
        "2026-08-16",
        "did:qualia:timothy_charles_holborn",
        "time",
        8.0,
        480.0,
        3.0,
        1440.0,
        0.0,
        1440.0,
    ),
];

pub fn build_contribution_ledger_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Append-only ledger \u{2014} all entries are immutable. \
         Replay-safe merge: entries are content-addressed by hash.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 4px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); border-bottom: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&info).unwrap();

    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 9px;");

    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in &[
        "Date",
        "Contributor",
        "Type",
        "Qty",
        "Fair Val",
        "Mult",
        "Obligation",
        "Compensated",
        "Owing",
    ] {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 3px 4px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono); white-space: nowrap;",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();

    let tbody = document.create_element("tbody").unwrap();
    let mut total_fv = 0.0f64;
    let mut total_obligation = 0.0f64;
    let mut total_compensated = 0.0f64;
    let mut total_owing = 0.0f64;

    for (date, did, ctype, qty, fv, mult, obligation, comp, owing) in ENTRIES {
        total_fv += fv;
        total_obligation += obligation;
        total_compensated += comp;
        total_owing += owing;

        let qty_s = format!("{:.0}", qty);
        let fv_s = format!("{:.0}", fv);
        let mult_s = format!("{:.1}x", mult);
        let ob_s = format!("{:.0}", obligation);
        let comp_s = format!("{:.0}", comp);
        let owing_s = format!("{:.0}", owing);

        let tr = document.create_element("tr").unwrap();
        for (i, val) in [
            date,
            did,
            ctype,
            qty_s.as_str(),
            fv_s.as_str(),
            mult_s.as_str(),
            ob_s.as_str(),
            comp_s.as_str(),
            owing_s.as_str(),
        ]
        .iter()
        .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            let color = if i == 8 && *owing > 0.0 {
                "rgba(255, 165, 0, 0.8)"
            } else if i == 8 {
                "rgba(100, 200, 100, 0.8)"
            } else if i == 7 && *comp > 0.0 {
                "rgba(100, 200, 100, 0.8)"
            } else if i == 6 {
                "rgba(255, 100, 100, 0.6)"
            } else {
                "var(--text-primary)"
            };
            td_el.style().set_css_text(&format!(
                "padding: 3px 4px; border-bottom: 1px solid var(--border-subtle); \
                 color: {}; font-size: 9px; font-family: var(--font-mono); white-space: nowrap;",
                color,
            ));
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }

    let tr = document.create_element("tr").unwrap();
    for val in [
        "Total",
        "",
        "",
        "",
        &format!("{:.0}", total_fv),
        "",
        &format!("{:.0}", total_obligation),
        &format!("{:.0}", total_compensated),
        &format!("{:.0}", total_owing),
    ]
    .iter()
    {
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(val));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 4px 4px; font-weight: 600; color: var(--accent-cyan); font-size: 9px; \
             font-family: var(--font-mono); border-top: 1px solid var(--border-medium); \
             white-space: nowrap;",
        );
        tr.append_child(&td).unwrap();
    }
    tbody.append_child(&tr).unwrap();

    table.append_child(&tbody).unwrap();

    let table_wrapper = document.create_element("div").unwrap();
    let tw_el: HtmlElement = table_wrapper.clone().dyn_into().unwrap();
    tw_el.style().set_css_text("flex: 1; overflow: auto;");
    table_wrapper.append_child(&table).unwrap();
    wrapper.append_child(&table_wrapper).unwrap();

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ Add Contribution Entry"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 4px; padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; align-self: flex-start;",
    );
    wrapper.append_child(&add_btn).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} ledger entries require wellfair_add_contribution engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
