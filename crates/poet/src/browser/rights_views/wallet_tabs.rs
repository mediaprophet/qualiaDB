//! Wallet tab views — balances, ILP/Lightning/XEC, tax suite, compute costs.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

pub fn build_balances_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-wallet-panel", "balances")
        .unwrap();

    let id_card = document.create_element("div").unwrap();
    let ic_el: HtmlElement = id_card.clone().dyn_into().unwrap();
    ic_el.style().set_css_text(
        "background: var(--surface-panel); border: 1px solid var(--border-subtle); \
         border-radius: 4px; padding: 8px 10px; margin-bottom: 6px;",
    );

    let id_h = document.create_element("div").unwrap();
    let ih_el: HtmlElement = id_h.clone().dyn_into().unwrap();
    ih_el
        .style()
        .set_css_text("display: flex; justify-content: space-between; margin-bottom: 4px;");

    let id_title = document.create_element("span").unwrap();
    id_title.set_text_content(Some("Identity"));
    let it_el: HtmlElement = id_title.clone().dyn_into().unwrap();
    it_el
        .style()
        .set_css_text("font-size: 11px; font-weight: 600; color: var(--text-primary);");
    id_h.append_child(&id_title).unwrap();

    let id_badge = document.create_element("span").unwrap();
    id_badge.set_class_name("honesty-badge honesty-live");
    id_badge.set_text_content(Some("live"));
    id_h.append_child(&id_badge).unwrap();
    id_card.append_child(&id_h).unwrap();

    let id_meta = document.create_element("div").unwrap();
    id_meta.set_text_content(Some(
        "DID: did:qualia:timothy_charles_holborn \u{00B7} BIP39: 12 words",
    ));
    let im_el: HtmlElement = id_meta.clone().dyn_into().unwrap();
    im_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    id_card.append_child(&id_meta).unwrap();
    panel.append_child(&id_card).unwrap();

    let balances: &[(&str, &str, &str)] = &[
        ("XEC", "1,250.00", "ecash"),
        ("USDC", "340.12", "usdc"),
        ("sats", "48,000", "lightning"),
        ("Q42", "8,000", "internal"),
    ];

    let table = make_table(document, &["Asset", "Amount", "Network"]);
    let tbody = document.create_element("tbody").unwrap();
    for (asset, amount, network) in balances {
        let tr = document.create_element("tr").unwrap();
        for val in &[asset, amount, network] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                 color: var(--text-primary); font-size: 10px; font-family: var(--font-mono);",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let btn_row = document.create_element("div").unwrap();
    let br_el: HtmlElement = btn_row.clone().dyn_into().unwrap();
    br_el
        .style()
        .set_css_text("display: flex; gap: 6px; margin-top: 6px;");

    for label in &["Send", "Receive", "Export"] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 4px 12px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 10px;",
        );
        btn_row.append_child(&btn).unwrap();
    }
    panel.append_child(&btn_row).unwrap();

    panel
}

pub fn build_ilp_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-wallet-panel", "ilp").unwrap();

    let pointers: &[(&str, &str, &str)] = &[
        ("$qualia.tfholborn/sats", "ILP", "active"),
        ("$qualia.tfholborn/xec", "ILP", "active"),
        ("lnbc1q...abc", "Lightning", "active"),
        ("xec1q...xyz", "XEC", "pending"),
    ];

    let table = make_table(document, &["Payment Pointer", "Protocol", "Status"]);
    let tbody = document.create_element("tbody").unwrap();
    for (pointer, protocol, status) in pointers {
        let tr = document.create_element("tr").unwrap();
        let status_color = match *status {
            "active" => "rgba(100, 200, 100, 0.8)",
            _ => "rgba(255, 165, 0, 0.8)",
        };
        for (i, val) in [pointer, protocol, status].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    status_color
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "ILP / Lightning / XEC micropayment execution \u{2014} PRT-1/2/3\n\
         Send/receive via payment pointers; Lightning channel management.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "margin-top: 6px; padding: 6px 8px; font-size: 10px; color: var(--text-muted); \
         font-family: var(--font-mono); background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    panel
}

pub fn build_tax_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-wallet-panel", "tax").unwrap();

    let recipients: &[(&str, &str, &str)] = &[
        ("ATO", "8.0%", "government"),
        ("State Revenue", "2.5%", "state"),
        ("Local Council", "1.5%", "local"),
        ("Total", "12.0%", "\u{2014}"),
    ];

    let table = make_table(document, &["Recipient", "Share %", "Jurisdiction"]);
    let tbody = document.create_element("tbody").unwrap();
    for (recipient, share, jurisdiction) in recipients {
        let tr = document.create_element("tr").unwrap();
        let is_total = *recipient == "Total";
        for (i, val) in [recipient, share, jurisdiction].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if is_total && i == 0 {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-top: 1px solid var(--border-medium); \
                     color: var(--text-primary); font-size: 10px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else if is_total && i == 1 {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-top: 1px solid var(--border-medium); \
                     color: var(--accent-cyan); font-size: 10px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "TaxRecipientSuite editor \u{2014} route_tax_payment\n\
         Shares must sum to 100%. Nym mixing optional.\n\
         Execute via ILP / Lightning / XEC.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "margin-top: 6px; padding: 6px 8px; font-size: 10px; color: var(--text-muted); \
         font-family: var(--font-mono); background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    panel
}

pub fn build_compute_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-wallet-panel", "compute").unwrap();

    let receipts: &[(&str, &str, &str, &str)] = &[
        (
            "NLP pipeline run",
            "1,200 sats",
            "2026-08-15",
            "model_inference",
        ),
        ("Ontology compile", "450 sats", "2026-08-14", "build"),
        ("SHACL validation", "180 sats", "2026-08-15", "validation"),
        ("SPARQL query", "60 sats", "2026-08-15", "query"),
    ];

    let table = make_table(document, &["Task", "Cost", "Date", "Type"]);
    let tbody = document.create_element("tbody").unwrap();
    for (task, cost, date, task_type) in receipts {
        let tr = document.create_element("tr").unwrap();
        for val in &[task, cost, date, task_type] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                 color: var(--text-primary); font-size: 10px; font-family: var(--font-mono);",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let total = document.create_element("div").unwrap();
    total.set_text_content(Some(
        "Total: 1,890 sats \u{2014} ComputeCostReceipt::generate",
    ));
    let t_el: HtmlElement = total.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "margin-top: 6px; padding: 6px 8px; font-size: 10px; color: var(--text-muted); \
         font-family: var(--font-mono); background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&total).unwrap();

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
