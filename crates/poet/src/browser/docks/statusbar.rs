//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Bottom status bar Graph/Merkle/Gas/Strata/Volume chrome.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, KeyboardEvent};

fn wire_keep_volume_chip(volume: &Element) {
    let listen = volume.clone();
    let click = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            crate::browser::topbar::open_save_mode_dialog(&doc);
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    listen
        .add_event_listener_with_callback("click", click.as_ref().unchecked_ref())
        .ok();
    click.forget();

    let key_listen = volume.clone();
    let key = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        if event.key() == "Enter" || event.key() == " " {
            event.prevent_default();
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                crate::browser::topbar::open_save_mode_dialog(&doc);
            }
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);
    key_listen
        .add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref())
        .ok();
    key.forget();
}

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
    g_val.set_text_content(Some(&crate::browser::surface_honesty::graph_held_copy()));
    bar.set_attribute(
        "data-honesty",
        crate::browser::surface_honesty::honesty_attr("held"),
    )
    .ok();
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
    m_val.set_text_content(Some(&crate::browser::surface_honesty::merkle_held_copy()));
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
    g_val.set_text_content(Some(&crate::browser::surface_honesty::gas_held_copy()));
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
    s_val.set_text_content(Some(&crate::browser::surface_honesty::strata_held_copy()));
    strata.append_child(&s_label).unwrap();
    strata.append_child(&s_val).unwrap();
    right.append_child(&strata).unwrap();

    let volume = document.create_element("div").unwrap();
    volume.set_class_name("statusbar-item");
    let v_label = document.create_element("span").unwrap();
    v_label.set_class_name("statusbar-label");
    v_label.set_text_content(Some("Keep:"));
    volume
        .set_attribute("title", "Keep volume — open a sanctuary volume")
        .ok();
    volume.set_attribute("data-frame-a-keep", "1").ok();
    volume.set_attribute("role", "button").ok();
    volume.set_attribute("tabindex", "0").ok();
    volume.set_attribute("aria-label", "Keep volume").ok();
    let v_val = document.create_element("span").unwrap();
    v_val.set_id("statusbar-volume-state");
    v_val.set_class_name("volume-state-chip");
    v_val.set_attribute("data-volume-state", "closed").ok();
    v_val.set_text_content(Some("closed"));
    volume.append_child(&v_label).unwrap();
    volume.append_child(&v_val).unwrap();
    wire_keep_volume_chip(&volume);
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
                        "Keep volume — sanctuary closed. Open a volume when a daemon is connected.",
                    )
                    .ok();
                }
            }
        }
        _ => {
            if !is_daemon_connected() {
                bar.set_attribute(
                    "data-honesty",
                    crate::browser::surface_honesty::honesty_attr("held"),
                )
                .ok();
                if let Some(g) = document.get_element_by_id("statusbar-graph-state") {
                    g.set_text_content(Some(&crate::browser::surface_honesty::graph_held_copy()));
                    g.set_attribute(
                        "data-honesty",
                        crate::browser::surface_honesty::honesty_attr("held"),
                    )
                    .ok();
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
            body.set_text_content(Some(&format!(
                "{} (Native Connected is separate — Catalog · Lexicon / invoke use the daemon).",
                crate::browser::surface_honesty::vibe_ui_held_copy()
            )));
        }
    }
}
