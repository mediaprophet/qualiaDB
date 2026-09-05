//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Bottom status bar Graph/Merkle/Gas/Strata/Volume chrome.

use web_sys::{Document, Element};

/// Build the bottom status bar.
pub fn build_bottom_statusbar(document: &Document) -> Element {
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("bottom-statusbar");
    crate::browser::surface_aspects::mark(&bar, "dwell");

    // Left section
    let left = document.create_element("div").unwrap();
    left.set_class_name("statusbar-section");

    let graph = document.create_element("div").unwrap();
    graph.set_class_name("statusbar-item");
    let g_label = document.create_element("span").unwrap();
    g_label.set_class_name("statusbar-label");
    g_label.set_text_content(Some("Graph:"));
    let g_val = document.create_element("span").unwrap();
    g_val.set_id("statusbar-graph-state");
    g_val.set_class_name("statusbar-value");
    g_val.set_text_content(Some("unavailable"));
    bar.set_attribute("data-honesty", "unavailable").ok();
    bar.set_attribute("data-statusbar", "poet-bottom").ok();
    graph.append_child(&g_label).unwrap();
    graph.append_child(&g_val).unwrap();
    left.append_child(&graph).unwrap();

    let merkle = document.create_element("div").unwrap();
    merkle.set_class_name("statusbar-item");
    let m_label = document.create_element("span").unwrap();
    m_label.set_class_name("statusbar-label");
    m_label.set_text_content(Some("Merkle:"));
    let m_val = document.create_element("span").unwrap();
    m_val.set_class_name("statusbar-value");
    m_val.set_text_content(Some("unavailable"));
    merkle.append_child(&m_label).unwrap();
    merkle.append_child(&m_val).unwrap();
    left.append_child(&merkle).unwrap();

    bar.append_child(&left).unwrap();

    // Right section
    let right = document.create_element("div").unwrap();
    right.set_class_name("statusbar-section");

    let gas = document.create_element("div").unwrap();
    gas.set_class_name("statusbar-item");
    let g_label = document.create_element("span").unwrap();
    g_label.set_class_name("statusbar-label");
    g_label.set_text_content(Some("Gas:"));
    let g_val = document.create_element("span").unwrap();
    g_val.set_class_name("statusbar-gas");
    g_val.set_text_content(Some("unavailable"));
    gas.append_child(&g_label).unwrap();
    gas.append_child(&g_val).unwrap();
    right.append_child(&gas).unwrap();

    let strata = document.create_element("div").unwrap();
    strata.set_class_name("statusbar-item");
    let s_label = document.create_element("span").unwrap();
    s_label.set_class_name("statusbar-label");
    s_label.set_text_content(Some("Strata:"));
    let s_val = document.create_element("span").unwrap();
    s_val.set_class_name("statusbar-value");
    s_val.set_text_content(Some("unavailable"));
    strata.append_child(&s_label).unwrap();
    strata.append_child(&s_val).unwrap();
    right.append_child(&strata).unwrap();

    let volume = document.create_element("div").unwrap();
    volume.set_class_name("statusbar-item");
    let v_label = document.create_element("span").unwrap();
    v_label.set_class_name("statusbar-label");
    v_label.set_text_content(Some("Volume:"));
    let v_val = document.create_element("span").unwrap();
    v_val.set_id("statusbar-volume-state");
    v_val.set_class_name("volume-state-chip");
    v_val.set_attribute("data-volume-state", "closed").ok();
    v_val.set_text_content(Some("closed"));
    volume.append_child(&v_label).unwrap();
    volume.append_child(&v_val).unwrap();
    right.append_child(&volume).unwrap();

    bar.append_child(&right).unwrap();
    refresh_bottom_statusbar_from_daemon(&bar);
    bar
}

/// Elevate Graph chrome when Native daemon is connected; Volume stays closed until open.
/// Vibe UI Live Engine dock is a separate host — not implied by daemon connect.
pub fn refresh_bottom_statusbar_from_daemon(bar: &Element) {
    use crate::browser::native_daemon::{
        get_daemon_state, is_daemon_connected, DaemonConnectionState,
    };
    let document = match bar.owner_document() {
        Some(d) => d,
        None => return,
    };
    let state = get_daemon_state();
    match state {
        DaemonConnectionState::Connected {
            graph_quin_count,
            port,
            ..
        } => {
            bar.set_attribute("data-honesty", "live").ok();
            bar.set_attribute("data-daemon-port", &port.to_string())
                .ok();
            if let Some(g) = document.get_element_by_id("statusbar-graph-state") {
                g.set_text_content(Some(&format!("live · {graph_quin_count} quins")));
                g.set_attribute("data-honesty", "live").ok();
            }
            // Volume remains closed until volume_open — honest sanctuary default.
            if let Some(v) = document.get_element_by_id("statusbar-volume-state") {
                if v.get_attribute("data-volume-state").as_deref() == Some("closed")
                    || v.get_attribute("data-volume-state").is_none()
                {
                    v.set_text_content(Some("closed"));
                    v.set_attribute(
                        "title",
                        "Sanctuary volume closed — open via GraphDatabase.volume_open",
                    )
                    .ok();
                }
            }
        }
        _ => {
            if !is_daemon_connected() {
                bar.set_attribute("data-honesty", "unavailable").ok();
                if let Some(g) = document.get_element_by_id("statusbar-graph-state") {
                    g.set_text_content(Some("unavailable"));
                    g.set_attribute("data-honesty", "unavailable").ok();
                }
            }
        }
    }
}

/// Refresh statusbar if present in the live document (called on daemon connect).
pub fn refresh_bottom_statusbar_in_document(document: &Document) {
    if let Ok(Some(bar)) = document.query_selector(".bottom-statusbar") {
        refresh_bottom_statusbar_from_daemon(&bar);
    }
    // Vibe UI Live Engine is a separate host — not implied by Native: Connected.
    if let Ok(Some(body)) = document.query_selector("[data-vibe-ui-host]") {
        if crate::browser::native_daemon::is_daemon_connected() {
            body.set_text_content(Some(
                "Unavailable: Vibe UI host not mounted (Native Connected is separate — Catalog · Lexicon / invoke use the daemon).",
            ));
        }
    }
}
