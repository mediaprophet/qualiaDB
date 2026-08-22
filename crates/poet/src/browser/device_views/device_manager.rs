//! Device Manager — list paired devices, pair new device, revoke device (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const DEVICES: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    (
        "did:qualia:device:desktop-01",
        "Desktop",
        "\u{1F5A5}",
        "Tim's Desktop",
        "Online",
        "Primary",
        "3840x2160 + 1920x1080",
    ),
    (
        "did:qualia:device:laptop-01",
        "Laptop",
        "\u{1F4BB}",
        "ThinkPad X1",
        "Online",
        "Secondary",
        "2560x1600",
    ),
    (
        "did:qualia:device:phone-01",
        "Phone",
        "\u{1F4F1}",
        "Pixel 9 Pro",
        "Online",
        "Remote",
        "1344x2992",
    ),
    (
        "did:qualia:device:tablet-01",
        "Tablet",
        "\u{1F4F2}",
        "iPad Pro 12.9",
        "Paired",
        "Control Surface",
        "2048x2732",
    ),
    (
        "did:qualia:device:watch-01",
        "Watch",
        "\u{231A}",
        "Pixel Watch 3",
        "Offline",
        "Remote",
        "454x454",
    ),
    (
        "did:qualia:device:headless-01",
        "Headless",
        "\u{1F916}",
        "Home Server",
        "Online",
        "Compute",
        "\u{2014}",
    ),
];

pub fn build_device_manager_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    // Toolbar
    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         align-items: center; flex-wrap: wrap;",
    );

    let pair_btn = document.create_element("button").unwrap();
    pair_btn.set_text_content(Some("\u{2795} Pair New Device"));
    let pb_el: HtmlElement = pair_btn.clone().dyn_into().unwrap();
    pb_el.style().set_css_text(
        "padding: 3px 8px; border: 1px solid var(--accent-cyan); background: rgba(0, 200, 255, 0.1); \
         color: var(--accent-cyan); border-radius: 3px; cursor: pointer; font-size: 8px; \
         font-family: var(--font-mono); font-weight: 600;",
    );
    toolbar.append_child(&pair_btn).unwrap();

    for label in &["Scan QR", "Enter Code", "Revoke", "Trust Level"] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 6px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
        );
        toolbar.append_child(&btn).unwrap();
    }

    let spacer = document.create_element("div").unwrap();
    let sp_el: HtmlElement = spacer.clone().dyn_into().unwrap();
    sp_el.style().set_css_text("flex: 1;");
    toolbar.append_child(&spacer).unwrap();

    let stats = document.create_element("span").unwrap();
    stats.set_text_content(Some("4 online | 1 paired | 1 offline | 6 total"));
    let st_el: HtmlElement = stats.clone().dyn_into().unwrap();
    st_el
        .style()
        .set_css_text("font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);");
    toolbar.append_child(&stats).unwrap();
    wrapper.append_child(&toolbar).unwrap();

    // Crypto chain info
    let chain_info = document.create_element("div").unwrap();
    chain_info.set_text_content(Some(
        "\u{1F511} Crypto chain: did:qualia:timothy_charles_holborn  |  Trust: L3 Sovereign  |  6 device attestations  |  Key rotation: 2026-08-15",
    ));
    let ci_el: HtmlElement = chain_info.clone().dyn_into().unwrap();
    ci_el.style().set_css_text(
        "padding: 4px 8px; background: var(--surface-panel); border-radius: 4px; \
         margin: 4px 8px; font-size: 8px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&chain_info).unwrap();

    // Device list
    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px 8px;");

    for (did, dtype, icon, label, status, role, displays) in DEVICES {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "display: flex; align-items: center; gap: 8px; padding: 6px 8px; \
             background: var(--surface-panel); border-radius: 4px; margin-bottom: 4px; \
             border: 1px solid var(--border-subtle);",
        );

        // Device icon
        let icon_div = document.create_element("div").unwrap();
        icon_div.set_text_content(Some(icon));
        let ic_el: HtmlElement = icon_div.clone().dyn_into().unwrap();
        ic_el
            .style()
            .set_css_text("font-size: 20px; flex-shrink: 0;");
        card.append_child(&icon_div).unwrap();

        // Device info
        let info = document.create_element("div").unwrap();
        let i_el: HtmlElement = info.clone().dyn_into().unwrap();
        i_el.style().set_css_text("flex: 1; min-width: 0;");

        let name_div = document.create_element("div").unwrap();
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        n_el.style()
            .set_css_text("display: flex; align-items: center; gap: 4px;");

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(label));
        let nm_el: HtmlElement = name.clone().dyn_into().unwrap();
        nm_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono);",
        );
        n_el.append_child(&name).unwrap();

        let dtype_badge = document.create_element("span").unwrap();
        dtype_badge.set_text_content(Some(dtype));
        let dt_el: HtmlElement = dtype_badge.clone().dyn_into().unwrap();
        dt_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             padding: 0 4px; border: 1px solid var(--border-subtle); border-radius: 2px;",
        );
        n_el.append_child(&dtype_badge).unwrap();
        info.append_child(&n_el).unwrap();

        let did_div = document.create_element("div").unwrap();
        did_div.set_text_content(Some(did));
        let d_el: HtmlElement = did_div.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
        );
        info.append_child(&did_div).unwrap();

        let displays_div = document.create_element("div").unwrap();
        displays_div.set_text_content(Some(displays));
        let dp_el: HtmlElement = displays_div.clone().dyn_into().unwrap();
        dp_el.style().set_css_text(
            "font-size: 7px; color: var(--text-secondary); font-family: var(--font-mono);",
        );
        info.append_child(&displays_div).unwrap();

        card.append_child(&info).unwrap();

        // Role badge
        let role_badge = document.create_element("div").unwrap();
        role_badge.set_text_content(Some(role));
        let r_el: HtmlElement = role_badge.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "font-size: 7px; color: var(--accent-cyan); font-family: var(--font-mono); \
             font-weight: 600; padding: 2px 6px; border: 1px solid rgba(0, 200, 255, 0.3); \
             border-radius: 3px; flex-shrink: 0;",
        );
        card.append_child(&role_badge).unwrap();

        // Status indicator
        let status_div = document.create_element("div").unwrap();
        let st_dot = document.create_element("div").unwrap();
        let sd_el: HtmlElement = st_dot.clone().dyn_into().unwrap();
        let (dot_color, st_label) = match *status {
            "Online" => ("rgba(100, 200, 100, 0.8)", "Online"),
            "Offline" => ("var(--text-muted)", "Offline"),
            "Paired" => ("rgba(0, 200, 255, 0.8)", "Paired"),
            _ => ("var(--text-muted)", *status),
        };
        sd_el.style().set_css_text(&format!(
            "width: 6px; height: 6px; border-radius: 50%; background: {}; flex-shrink: 0;",
            dot_color,
        ));
        status_div.append_child(&st_dot).unwrap();

        let st_text = document.create_element("span").unwrap();
        st_text.set_text_content(Some(st_label));
        let stx_el: HtmlElement = st_text.clone().dyn_into().unwrap();
        stx_el.style().set_css_text(&format!(
            "font-size: 7px; color: {}; font-family: var(--font-mono); margin-left: 2px;",
            dot_color,
        ));
        status_div.append_child(&st_text).unwrap();

        let sx_el: HtmlElement = status_div.clone().dyn_into().unwrap();
        sx_el
            .style()
            .set_css_text("display: flex; align-items: center; flex-shrink: 0;");
        card.append_child(&status_div).unwrap();

        content.append_child(&card).unwrap();
    }
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} device pairing requires WebRTC + crypto chain integration.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
