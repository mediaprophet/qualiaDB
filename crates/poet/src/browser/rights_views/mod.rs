//! Rights & Wallet view modules — tabbed surfaces for agreements, wallet, etc.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

pub mod rights_tabs;
pub mod wallet_tabs;

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Rights & Agreements container — 5 tabs per §6.1 of Workstream A requirements.
pub fn build_rights_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let w_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    w_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); overflow-x: auto;",
    );

    let tabs = &[
        ("agreements", "Agreements"),
        ("deontic", "Deontic Norms"),
        ("jural", "Jural Relations"),
        ("breach", "Breach Log"),
        ("consents", "Consents"),
    ];

    for (i, (tab_id, tab_label)) in tabs.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-rights-tab", tab_id).unwrap();
        tab.set_text_content(Some(tab_label));
        let t_el: HtmlElement = tab.clone().dyn_into().unwrap();
        t_el.style().set_css_text(&format!(
            "padding: 4px 10px; border: none; border-bottom: 2px solid {}; \
             background: transparent; color: {}; font-size: 10px; \
             font-family: var(--font-mono); cursor: pointer; white-space: nowrap;",
            if i == 0 {
                "var(--color-rights)"
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

    content
        .append_child(&rights_tabs::build_agreements_tab(document))
        .unwrap();

    let deontic = rights_tabs::build_deontic_tab(document);
    {
        let d_el: HtmlElement = deontic.clone().dyn_into().unwrap();
        d_el.style().set_css_text("display: none;");
    }
    content.append_child(&deontic).unwrap();

    let jural = rights_tabs::build_jural_tab(document);
    {
        let j_el: HtmlElement = jural.clone().dyn_into().unwrap();
        j_el.style().set_css_text("display: none;");
    }
    content.append_child(&jural).unwrap();

    let breach = rights_tabs::build_breach_tab(document);
    {
        let b_el: HtmlElement = breach.clone().dyn_into().unwrap();
        b_el.style().set_css_text("display: none;");
    }
    content.append_child(&breach).unwrap();

    let consents = rights_tabs::build_consents_tab(document);
    {
        let cs_el: HtmlElement = consents.clone().dyn_into().unwrap();
        cs_el.style().set_css_text("display: none;");
    }
    content.append_child(&consents).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} rights/agreements requires AgreementDID + deontic engine commands.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

/// Wallet container — 4 tabs per §6.2 of Workstream A requirements.
pub fn build_wallet_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let w_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    w_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); overflow-x: auto;",
    );

    let tabs = &[
        ("balances", "Balances"),
        ("ilp", "ILP / Lightning / XEC"),
        ("tax", "Tax Suite"),
        ("compute", "Compute Costs"),
    ];

    for (i, (tab_id, tab_label)) in tabs.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-wallet-tab", tab_id).unwrap();
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

    content
        .append_child(&wallet_tabs::build_balances_tab(document))
        .unwrap();

    let ilp = wallet_tabs::build_ilp_tab(document);
    {
        let i_el: HtmlElement = ilp.clone().dyn_into().unwrap();
        i_el.style().set_css_text("display: none;");
    }
    content.append_child(&ilp).unwrap();

    let tax = wallet_tabs::build_tax_tab(document);
    {
        let t_el: HtmlElement = tax.clone().dyn_into().unwrap();
        t_el.style().set_css_text("display: none;");
    }
    content.append_child(&tax).unwrap();

    let compute = wallet_tabs::build_compute_tab(document);
    {
        let cc_el: HtmlElement = compute.clone().dyn_into().unwrap();
        cc_el.style().set_css_text("display: none;");
    }
    content.append_child(&compute).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} wallet requires ledger_balance + ILP/Lightning engine commands.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
