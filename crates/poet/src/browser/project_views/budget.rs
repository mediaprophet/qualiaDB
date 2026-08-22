//! Budget & Finance — budget line items, ledger, variance, funding, royalties, tax.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("budget", "Budget"),
    ("ledger", "Ledger"),
    ("variance", "Variance"),
    ("funding", "Funding"),
    ("royalties", "Royalties"),
    ("tax", "Tax Router"),
];

pub fn build_budget_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    // Tab bar
    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-budget-tab", tab_id).unwrap();
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
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Budget tab
    content.append_child(&build_budget_tab(document)).unwrap();

    // Ledger tab (hidden)
    let ledger = build_ledger_tab(document);
    {
        let l_el: HtmlElement = ledger.clone().dyn_into().unwrap();
        l_el.style().set_css_text("display: none;");
    }
    content.append_child(&ledger).unwrap();

    let variance = build_variance_tab(document);
    {
        let v_el: HtmlElement = variance.clone().dyn_into().unwrap();
        v_el.style().set_css_text("display: none;");
    }
    content.append_child(&variance).unwrap();

    let funding = build_funding_tab(document);
    {
        let f_el: HtmlElement = funding.clone().dyn_into().unwrap();
        f_el.style().set_css_text("display: none;");
    }
    content.append_child(&funding).unwrap();

    let royalties = build_royalties_tab(document);
    {
        let r_el: HtmlElement = royalties.clone().dyn_into().unwrap();
        r_el.style().set_css_text("display: none;");
    }
    content.append_child(&royalties).unwrap();

    let tax = build_tax_tab(document);
    {
        let t_el: HtmlElement = tax.clone().dyn_into().unwrap();
        t_el.style().set_css_text("display: none;");
    }
    content.append_child(&tax).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} budget/ledger requires wellfair_add_ledger_entry engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_budget_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-budget-panel", "budget").unwrap();

    let items: &[(&str, i64, &str, &str)] = &[
        ("Personnel", 5000000, "Phase 1", "sats"),
        ("Infrastructure", 1500000, "Phase 1", "sats"),
        ("Ontology licensing", 500000, "Phase 1", "sats"),
        ("Personnel", 4000000, "Phase 2", "sats"),
        ("Compute", 2000000, "Phase 2", "sats"),
    ];

    let table = make_table(document, &["Category", "Planned", "Phase", "Currency"]);
    let tbody = document.create_element("tbody").unwrap();
    let mut total = 0i64;
    for (cat, planned, phase, curr) in items {
        total += planned;
        let planned_str = format_planned(*planned);
        let tr = document.create_element("tr").unwrap();
        for val in &[cat, planned_str.as_str(), phase, curr] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px;",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    let tr = document.create_element("tr").unwrap();
    let td = document.create_element("td").unwrap();
    td.set_text_content(Some("Total"));
    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
    td_el.style().set_css_text(
        "padding: 4px 6px; font-weight: 600; color: var(--text-primary); font-size: 10px;",
    );
    tr.append_child(&td).unwrap();
    let td = document.create_element("td").unwrap();
    let total_str = format_planned(total);
    td.set_text_content(Some(&total_str));
    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
    td_el.style().set_css_text(
        "padding: 4px 6px; font-weight: 600; color: var(--accent-cyan); font-size: 10px;",
    );
    tr.append_child(&td).unwrap();
    for _ in 0..2 {
        let td = document.create_element("td").unwrap();
        tr.append_child(&td).unwrap();
    }
    tbody.append_child(&tr).unwrap();
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ Add Line Item"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&add_btn).unwrap();

    panel
}

fn build_ledger_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-budget-panel", "ledger").unwrap();

    let entries: &[(&str, i64, &str, &str, &str)] = &[
        (
            "Initial funding",
            5000000,
            "sats",
            "grant",
            "did:qualia:funder_01",
        ),
        (
            "Server costs",
            -200000,
            "sats",
            "infrastructure",
            "did:qualia:vendor_01",
        ),
        (
            "Personnel Q1",
            -1000000,
            "sats",
            "personnel",
            "did:qualia:timothy_charles_holborn",
        ),
        (
            "Ontology license",
            -50000,
            "sats",
            "licensing",
            "did:qualia:vendor_02",
        ),
    ];

    let table = make_table(
        document,
        &[
            "Description",
            "Amount",
            "Currency",
            "Category",
            "Counterparty",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    let mut balance = 0i64;
    for (desc, amount, curr, cat, party) in entries {
        balance += amount;
        let amount_str = format_planned(*amount);
        let tr = document.create_element("tr").unwrap();
        for val in &[desc, amount_str.as_str(), curr, cat, party] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if val.starts_with('-') {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: rgba(255, 99, 71, 0.8); font-size: 10px;",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px;",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let bal = document.create_element("div").unwrap();
    let bal_str = format!("Running balance: {} sats", format_planned(balance));
    bal.set_text_content(Some(&bal_str));
    let b_el: HtmlElement = bal.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 8px; font-size: 11px; font-weight: 600; \
             color: var(--accent-cyan); font-family: var(--font-mono);",
    );
    panel.append_child(&bal).unwrap();

    panel
}

fn build_variance_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-budget-panel", "variance")
        .unwrap();

    let rows: &[(&str, i64, i64, &str)] = &[
        ("Personnel", 5000000, 4800000, "Phase 1"),
        ("Infrastructure", 1500000, 1700000, "Phase 1"),
        ("Ontology licensing", 500000, 500000, "Phase 1"),
    ];

    let table = make_table(
        document,
        &["Category", "Planned", "Actual", "Variance", "Phase"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (cat, planned, actual, phase) in rows {
        let variance = actual - planned;
        let var_str = if variance > 0 {
            format!("+{}", format_planned(variance))
        } else {
            format_planned(variance)
        };
        let var_color = if variance > 0 {
            "rgba(255, 99, 71, 0.8)"
        } else {
            "rgba(100, 200, 100, 0.8)"
        };

        let planned_str = format_planned(*planned);
        let actual_str = format_planned(*actual);
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [
            cat,
            planned_str.as_str(),
            actual_str.as_str(),
            var_str.as_str(),
            phase,
        ]
        .iter()
        .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    var_color
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px;",
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

fn build_funding_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-budget-panel", "funding").unwrap();

    let flows: &[(&str, i64, &str, &str)] = &[
        ("did:qualia:funder_01", 5000000, "grant", "2026-08-01"),
        ("did:qualia:funder_02", 1000000, "donation", "2026-08-10"),
        ("did:qualia:corp_01", 500000, "sponsorship", "2026-08-15"),
    ];

    let table = make_table(document, &["Funder", "Amount", "Kind", "Date", "Action"]);
    let tbody = document.create_element("tbody").unwrap();
    for (funder, amount, kind, date) in flows {
        let amount_str = format_planned(*amount);
        let tr = document.create_element("tr").unwrap();
        for val in &[
            funder,
            amount_str.as_str(),
            kind,
            date,
            "Apply to obligation",
        ] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px;",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    panel
}

fn build_royalties_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-budget-panel", "royalties")
        .unwrap();

    let dist: &[(&str, f64, i64, &str)] = &[
        (
            "did:qualia:timothy_charles_holborn",
            0.60,
            3000000,
            "replay-safe",
        ),
        ("did:qualia:researcher_01", 0.25, 1250000, "replay-safe"),
        ("did:qualia:reviewer_02", 0.15, 750000, "replay-safe"),
    ];

    let table = make_table(
        document,
        &["Contributor", "ROI Multiplier", "Amount", "Status"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (who, mult, amount, status) in dist {
        let mult_str = format!("{:.0}%", mult * 100.0);
        let amount_str = format_planned(*amount);
        let tr = document.create_element("tr").unwrap();
        for val in &[who, mult_str.as_str(), amount_str.as_str(), status] {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px;",
            );
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    panel
}

fn build_tax_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-budget-panel", "tax").unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "TaxRecipientSuite \u{2014} jurisdiction: AU-NSW\n\
         Split: 12% tax / 88% principal+contributors\n\
         Recipients: ATO (8%), State Revenue (3%), Local Council (1%)",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 8px; background: var(--surface-panel); border-radius: 4px; \
             font-size: 10px; color: var(--text-secondary); white-space: pre-wrap; \
             font-family: var(--font-mono); margin-bottom: 6px;",
    );
    panel.append_child(&info).unwrap();

    let preview = document.create_element("div").unwrap();
    preview.set_text_content(Some(
        "Dispatch plan preview:\n\
         \u{2192} ILP stream: 600 sats/min \u{2192} ATO payment pointer\n\
         \u{2192} Lightning: 225 sats/min \u{2192} State Revenue node\n\
         \u{2192} XEC: 75 sats/min \u{2192} Local Council address",
    ));
    let p_el: HtmlElement = preview.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "padding: 8px; background: var(--surface-panel); border-radius: 4px; \
             font-size: 10px; color: var(--text-muted); white-space: pre-wrap; \
             font-family: var(--font-mono);",
    );
    panel.append_child(&preview).unwrap();

    let exec_btn = document.create_element("button").unwrap();
    exec_btn.set_text_content(Some("\u{26A1} Execute Tax Dispatch"));
    let e_el: HtmlElement = exec_btn.clone().dyn_into().unwrap();
    e_el.style().set_css_text(
        "margin-top: 6px; padding: 4px 12px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 10px;",
    );
    panel.append_child(&exec_btn).unwrap();

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

fn format_planned(sats: i64) -> String {
    let abs = sats.unsigned_abs();
    let prefix = if sats < 0 { "-" } else { "" };
    if abs >= 1_000_000 {
        format!(
            "{}{}.{:03}M sats",
            prefix,
            abs / 1_000_000,
            (abs % 1_000_000) / 1_000
        )
    } else if abs >= 1_000 {
        format!("{}{}k sats", prefix, abs / 1_000)
    } else {
        format!("{}{} sats", prefix, abs)
    }
}
