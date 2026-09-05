//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Finance container.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Finance (Black-Scholes, portfolio)
// ---------------------------------------------------------------------------

/// Finance container — portfolio, Black-Scholes, ledger entries.
pub fn build_finance_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Portfolio summary
    let summary = document.create_element("div").unwrap();
    summary.set_class_name("cr-card");
    let s_el: HtmlElement = summary.clone().dyn_into().unwrap();
    s_el.style().set_css_text("padding: 8px 10px; background: rgba(0, 210, 255, 0.08); border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); display: flex; justify-content: space-between; align-items: center; font-size: 10px;");
    summary.set_inner_html(
        "<div><span style='color: var(--accent-cyan); font-weight: 700;'>Portfolio Total:</span> $4,250.00 USD</div>\
         <div style='color: var(--accent-emerald); font-weight: 600;'>+8.4% (24h)</div>"
    );
    wrapper.append_child(&summary).unwrap();

    // Asset balances grid
    let asset_grid = document.create_element("div").unwrap();
    let ag_el: HtmlElement = asset_grid.clone().dyn_into().unwrap();
    ag_el
        .style()
        .set_css_text("display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;");

    let assets = [
        ("XEC Vault", "1,250 XEC", "$2,100.00", "var(--accent-amber)"),
        (
            "USDC Collateral",
            "340 USDC",
            "$340.00",
            "var(--accent-cyan)",
        ),
        (
            "Q42 Commons",
            "8,000 Q42",
            "$1,810.00",
            "var(--accent-violet)",
        ),
    ];

    for (name, bal, val, col) in assets {
        let card = document.create_element("div").unwrap();
        card.set_class_name("cr-card");
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text("padding: 6px 8px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 9px; display: flex; flex-direction: column; gap: 2px;");

        let n_el = document.create_element("span").unwrap();
        n_el.set_text_content(Some(name));
        n_el.set_attribute("style", &format!("font-weight: 700; color: {};", col))
            .unwrap();
        card.append_child(&n_el).unwrap();

        let b_el = document.create_element("span").unwrap();
        b_el.set_text_content(Some(bal));
        b_el.set_attribute("style", "color: var(--text-primary); font-weight: 600;")
            .unwrap();
        card.append_child(&b_el).unwrap();

        let v_el = document.create_element("span").unwrap();
        v_el.set_text_content(Some(val));
        v_el.set_attribute("style", "color: var(--text-muted); font-size: 8px;")
            .unwrap();
        card.append_child(&v_el).unwrap();

        asset_grid.append_child(&card).unwrap();
    }
    wrapper.append_child(&asset_grid).unwrap();

    // Ledger
    let ledger = document.create_element("div").unwrap();
    ledger.set_class_name("vibe-output");
    let l_el: HtmlElement = ledger.clone().dyn_into().unwrap();
    l_el.style()
        .set_css_text("display: flex; flex-direction: column; gap: 3px; font-size: 9px;");
    for entry in &[
        "2026-08-17 \u{00B7} +250.00 XEC \u{00B7} vault handshake (verified)",
        "2026-08-16 \u{00B7} -40.00 USDC \u{00B7} zero-knowledge tax batch",
        "2026-08-15 \u{00B7} +1,000 Q42 \u{00B7} semantic token minting",
    ] {
        let line = document.create_element("div").unwrap();
        line.set_class_name("vibe-out-line");
        line.set_text_content(Some(entry));
        ledger.append_child(&line).unwrap();
    }
    wrapper.append_child(&ledger).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Black-Scholes", "Tax Suite", "Send XEC", "Export Ledger"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}
