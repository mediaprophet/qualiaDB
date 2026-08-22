//! Biometrics — biometric record list with ZK proof actions (§5, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const BIOMETRICS: &[(&str, &str, &str, &str)] = &[
    (
        "fingerprint",
        "0x7a3f8b2c9e1d4a6f",
        "2024-01-15",
        "Device-A1",
    ),
    (
        "voiceprint",
        "0x4b8e2c1f9a3d5b7e",
        "2024-02-20",
        "Device-A1",
    ),
    ("face", "0xc2d9e6a3b1f8470c", "2024-03-10", "Device-A1"),
    ("HRV", "0x8f1a3c5e7b9d2046", "2025-06-01", "Watch-B2"),
];

pub fn build_biometrics_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let notice = document.create_element("div").unwrap();
    notice.set_text_content(Some(
        "\u{1F512} Your biometric template is never stored. Only a mathematical commitment is kept. \
         You can prove facts about your biometric without revealing it.",
    ));
    let n_el: HtmlElement = notice.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: rgba(200, 150, 255, 0.8); \
         font-family: var(--font-mono); background: var(--surface-panel); \
         border-radius: 4px; border: 1px solid rgba(200, 150, 255, 0.3); \
         margin: 4px 8px;",
    );
    wrapper.append_child(&notice).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    let table = make_table(
        document,
        &[
            "Kind",
            "Commitment Hash",
            "Enrolled At",
            "Device",
            "Actions",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (kind, hash, enrolled, device) in BIOMETRICS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [kind, hash, enrolled, device].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            td_el.style().set_css_text(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                 color: var(--text-primary); font-size: 10px; \
                 font-family: var(--font-mono);",
            );
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: rgba(200, 150, 255, 0.8); font-size: 9px; \
                     font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }

        let actions_td = document.create_element("td").unwrap();
        let at_el: HtmlElement = actions_td.clone().dyn_into().unwrap();
        at_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             display: flex; gap: 4px;",
        );
        for label in &["ZK Proof", "Verify", "Revoke"] {
            let btn = document.create_element("button").unwrap();
            btn.set_text_content(Some(label));
            let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
            b_el.style().set_css_text(
                "padding: 1px 4px; border: 1px solid var(--border-medium); \
                 background: transparent; color: var(--text-secondary); border-radius: 2px; \
                 cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
            );
            actions_td.append_child(&btn).unwrap();
        }
        tr.append_child(&actions_td).unwrap();
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();
    wrapper.append_child(&content).unwrap();

    let zk_section = document.create_element("div").unwrap();
    zk_section.set_text_content(Some(
        "ZK Proof Actions: Prove identity | Prove age >= 18 | Selective disclosure | Verify proof",
    ));
    let z_el: HtmlElement = zk_section.clone().dyn_into().unwrap();
    z_el.style().set_css_text(
        "padding: 4px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); border-top: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&zk_section).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} biometrics require BIO-1..BIO-5 + ZK-1..ZK-7. \
         Template is NEVER displayed, stored, or transmitted.",
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
