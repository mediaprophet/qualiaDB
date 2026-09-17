//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! WebView and WebRTC containers.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Webview (browser pane — desktop only)
// ---------------------------------------------------------------------------

/// Webview container — capability-gated browser pane.
pub fn build_webview_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px; font-family: var(--font-mono); color: var(--text-primary);");

    // Browser Tabs
    let tab_strip = document.create_element("div").unwrap();
    let ts_el: HtmlElement = tab_strip.clone().dyn_into().unwrap();
    ts_el.style().set_css_text("display: flex; gap: 4px; align-items: center; border-bottom: 1px solid var(--border-subtle); padding-bottom: 4px; font-size: 10px;");

    let tab1 = document.create_element("span").unwrap();
    tab1.set_text_content(Some("\u{1F310} Qualia Network Explorer \u{00D7}"));
    tab1.set_attribute("style", "padding: 3px 8px; background: var(--surface-panel-elevated); border: 1px solid var(--accent-cyan); border-radius: 4px; color: var(--text-primary); font-weight: 600;").unwrap();
    tab_strip.append_child(&tab1).unwrap();

    let tab_add = document.create_element("button").unwrap();
    tab_add.set_class_name("vibe-run-btn");
    tab_add.set_text_content(Some("+"));
    tab_strip.append_child(&tab_add).unwrap();
    wrapper.append_child(&tab_strip).unwrap();

    // URL & Navigation bar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let nav_back = document.create_element("button").unwrap();
    nav_back.set_class_name("vibe-run-btn");
    nav_back.set_text_content(Some("\u{25C0}"));
    bar.append_child(&nav_back).unwrap();

    let nav_fwd = document.create_element("button").unwrap();
    nav_fwd.set_class_name("vibe-run-btn");
    nav_fwd.set_text_content(Some("\u{25B6}"));
    bar.append_child(&nav_fwd).unwrap();

    let lock_icon = document.create_element("span").unwrap();
    lock_icon.set_text_content(Some("\u{1F512}"));
    lock_icon
        .set_attribute(
            "style",
            "font-size: 11px; margin-left: 2px; color: var(--accent-emerald);",
        )
        .unwrap();
    bar.append_child(&lock_icon).unwrap();

    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_value("https://qualia.network/explorer/habitat");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 4px 8px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono); outline: none;").unwrap();
    bar.append_child(&input).unwrap();

    let go_btn = document.create_element("button").unwrap();
    go_btn.set_class_name("vibe-run-btn");
    go_btn.set_text_content(Some("\u{21BB}"));
    bar.append_child(&go_btn).unwrap();
    wrapper.append_child(&bar).unwrap();

    // Rendered Viewport Sandbox Frame
    let viewport = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport.clone().dyn_into().unwrap();
    vp_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.4); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 12px; display: flex; flex-direction: column; gap: 8px; font-size: 11px; overflow-y: auto;");

    let cert_badge = document.create_element("div").unwrap();
    cert_badge.set_attribute("style", "display: flex; align-items: center; justify-content: space-between; padding: 4px 8px; background: rgba(0, 242, 169, 0.08); border: 1px solid rgba(0, 242, 169, 0.25); border-radius: 4px; font-size: 9px; color: var(--accent-emerald);").unwrap();
    cert_badge.set_inner_html("<span>\u{2713} Origin Trust: Level 4 (Verifiable DID Signed)</span><span>TLS 1.3 \u{00B7} Sandbox Strict</span>");
    viewport.append_child(&cert_badge).unwrap();

    let page_content = document.create_element("div").unwrap();
    page_content.set_inner_html(
        "<h3 style='margin: 0 0 6px 0; color: var(--accent-cyan); font-size: 13px;'>Qualia Network Habitat Explorer</h3>\
         <p style='margin: 0 0 8px 0; color: var(--text-secondary); line-height: 1.4;'>\
         Connected to local cluster gateway at <code>http://127.0.0.1:4242</code>. \
         All hypermedia documents are cryptographically resolved through zero-copy Super-Quins.\
         </p>\
         <div style='background: var(--surface-panel); padding: 8px; border-radius: 4px; border: 1px solid var(--border-subtle);'>\
         <strong>Active Habitat Node:</strong> did:qualia:timothy_charles_holborn<br/>\
         <strong>Routing Lane:</strong> Bilateral Micro-Commons (48-byte Packed)\
         </div>"
    );
    viewport.append_child(&page_content).unwrap();
    wrapper.append_child(&viewport).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// WebRTC (P2P mesh, DataChannel sync)
// ---------------------------------------------------------------------------

/// WebRTC container — P2P DataChannel mesh & Super-Quin synchronization.
pub fn build_webrtc_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Status Card
    let status_card = document.create_element("div").unwrap();
    status_card.set_class_name("cr-card");
    let sc_el: HtmlElement = status_card.clone().dyn_into().unwrap();
    sc_el.style().set_css_text("padding: 8px 10px; background: rgba(0, 210, 255, 0.08); border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); display: flex; justify-content: space-between; align-items: center; font-size: 10px;");
    status_card.set_inner_html(
        "<div><span style='color: var(--accent-cyan); font-weight: 700;'>P2P Mesh:</span> <span style='color: var(--accent-emerald); font-weight: 600;'>\u{25CF} Connected (3 Peers)</span></div>\
         <div style='color: var(--text-muted);'>ICE: Direct Host-to-Host</div>"
    );
    wrapper.append_child(&status_card).unwrap();

    // Active Swarm Peer List
    let peer_list = document.create_element("div").unwrap();
    let pl_el: HtmlElement = peer_list.clone().dyn_into().unwrap();
    pl_el
        .style()
        .set_css_text("display: flex; flex-direction: column; gap: 4px;");

    let peers = [
        (
            "did:qualia:edge:node-7f2a",
            "14ms",
            "182 KB/s",
            "CRDT Sync: Active",
        ),
        (
            "did:qualia:edge:node-3b91",
            "28ms",
            "94 KB/s",
            "CRDT Sync: Active",
        ),
        (
            "did:qualia:edge:node-c044",
            "19ms",
            "220 KB/s",
            "CRDT Sync: Active",
        ),
    ];

    for (peer_did, latency, throughput, sync_mode) in peers {
        let row = document.create_element("div").unwrap();
        row.set_class_name("vibe-output");
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text("padding: 6px 8px; display: flex; justify-content: space-between; align-items: center; font-size: 9px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs);");

        let left = document.create_element("div").unwrap();
        left.set_attribute("style", "display: flex; align-items: center; gap: 6px;")
            .unwrap();
        left.set_inner_html(&format!(
            "<span style='color: var(--accent-amber);'>\u{29BF}</span><strong>{}</strong>",
            peer_did
        ));
        row.append_child(&left).unwrap();

        let right = document.create_element("div").unwrap();
        right
            .set_attribute(
                "style",
                "display: flex; gap: 10px; color: var(--text-muted);",
            )
            .unwrap();
        right.set_inner_html(&format!(
            "<span>{}</span><span>{}</span><span style='color: var(--accent-emerald);'>{}</span>",
            latency, throughput, sync_mode
        ));
        row.append_child(&right).unwrap();

        peer_list.append_child(&row).unwrap();
    }
    wrapper.append_child(&peer_list).unwrap();

    // Action Toolbar
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &[
        "Broadcast Super-Quin",
        "Ping Swarm",
        "ICE Renegotiate",
        "Inspect SDP",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}
