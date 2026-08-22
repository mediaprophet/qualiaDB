//! Workspace Sync — sync status, version history, conflict resolution (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const SYNC_HISTORY: &[(u64, &str, &str, &str, &str)] = &[
    (
        47,
        "desktop-01",
        "ContainerMoved: graph_canvas",
        "2026-08-19 11:48",
        "Applied",
    ),
    (
        46,
        "phone-01",
        "ManifoldChanged: ontology",
        "2026-08-19 11:45",
        "Applied",
    ),
    (
        45,
        "laptop-01",
        "ContainerAdded: shex_editor",
        "2026-08-19 11:42",
        "Applied",
    ),
    (
        44,
        "desktop-01",
        "DeviceAssigned: tablet-01 \u{2192} ControlSurface",
        "2026-08-19 11:30",
        "Applied",
    ),
    (
        43,
        "phone-01",
        "ContainerMoved: pulse-panel",
        "2026-08-19 11:28",
        "Applied",
    ),
    (
        42,
        "desktop-01",
        "ContainerRemoved: old_graph",
        "2026-08-19 11:15",
        "Applied",
    ),
    (
        41,
        "laptop-01",
        "ContainerMoved: n3_editor",
        "2026-08-19 11:10",
        "Conflict \u{2192} Resolved (laptop wins)",
    ),
    (
        40,
        "desktop-01",
        "DevicePaired: tablet-01",
        "2026-08-19 10:55",
        "Applied",
    ),
    (
        39,
        "phone-01",
        "ManifoldChanged: social",
        "2026-08-19 10:30",
        "Applied",
    ),
    (
        38,
        "desktop-01",
        "ContainerAdded: vocabulary_mapper",
        "2026-08-19 10:15",
        "Applied",
    ),
];

const SYNC_PEERS: &[(&str, &str, &str, &str)] = &[
    ("desktop-01", "Online", "2ms", "v47 \u{2713}"),
    ("laptop-01", "Online", "15ms", "v47 \u{2713}"),
    ("phone-01", "Online", "45ms", "v47 \u{2713}"),
    ("headless-01", "Online", "3ms", "v47 \u{2713}"),
    ("tablet-01", "Paired", "\u{2014}", "v40 (pending)"),
    ("watch-01", "Offline", "\u{2014}", "v38 (stale)"),
];

pub fn build_workspace_sync_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         align-items: center; flex-wrap: wrap;",
    );

    for label in &["Sync Now", "Force Push", "Rollback", "Export State"] {
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

    let version = document.create_element("span").unwrap();
    version.set_text_content(Some("v47 \u{2713} synced  |  4 peers online"));
    let v_el: HtmlElement = version.clone().dyn_into().unwrap();
    v_el.style().set_css_text(
        "font-size: 8px; color: rgba(100, 200, 100, 0.8); font-family: var(--font-mono); \
         font-weight: 600;",
    );
    toolbar.append_child(&version).unwrap();
    wrapper.append_child(&toolbar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px 8px;");

    // Peer sync status
    let peer_header = document.create_element("div").unwrap();
    peer_header.set_text_content(Some("Peer Sync Status"));
    let ph_el: HtmlElement = peer_header.clone().dyn_into().unwrap();
    ph_el.style().set_css_text(
        "font-size: 8px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; margin-bottom: 4px;",
    );
    content.append_child(&peer_header).unwrap();

    let peer_table = make_table(document, &["Device", "Status", "Latency", "Version"]);
    let peer_tbody = document.create_element("tbody").unwrap();
    for (device, status, latency, ver) in SYNC_PEERS {
        let tr = document.create_element("tr").unwrap();
        let vals = vec![
            device.to_string(),
            status.to_string(),
            latency.to_string(),
            ver.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                let color = if *status == "Online" {
                    "rgba(100, 200, 100, 0.8)"
                } else if *status == "Paired" {
                    "rgba(0, 200, 255, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 7px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 3 {
                let color = if val.contains("\u{2713}") {
                    "rgba(100, 200, 100, 0.8)"
                } else if val.contains("pending") {
                    "rgba(255, 165, 0, 0.8)"
                } else {
                    "var(--text-muted)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 7px; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        peer_tbody.append_child(&tr).unwrap();
    }
    peer_table.append_child(&peer_tbody).unwrap();
    content.append_child(&peer_table).unwrap();

    // Delta history
    let hist_header = document.create_element("div").unwrap();
    hist_header.set_text_content(Some("Delta History (last 10)"));
    let hh_el: HtmlElement = hist_header.clone().dyn_into().unwrap();
    hh_el.style().set_css_text(
        "font-size: 8px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; \
         margin-top: 8px; margin-bottom: 4px;",
    );
    content.append_child(&hist_header).unwrap();

    let hist_table = make_table(document, &["Ver", "Device", "Change", "Time", "Status"]);
    let hist_tbody = document.create_element("tbody").unwrap();
    for (ver, device, change, time, status) in SYNC_HISTORY {
        let tr = document.create_element("tr").unwrap();
        let vals = vec![
            format!("v{}", ver),
            device.to_string(),
            change.to_string(),
            time.to_string(),
            status.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = if val.contains("Conflict") {
                    "rgba(255, 165, 0, 0.8)"
                } else {
                    "rgba(100, 200, 100, 0.6)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 7px; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 0 {
                td_el.style().set_css_text(
                    "padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 7px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 7px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        hist_tbody.append_child(&tr).unwrap();
    }
    hist_table.append_child(&hist_tbody).unwrap();
    content.append_child(&hist_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} workspace sync requires WebRTC data channels + crypto chain signing.",
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
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 9px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 3px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
