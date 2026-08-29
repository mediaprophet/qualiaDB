//! Visible adapter for the process-wide Pulse SSE connection owned by
//! `native_daemon`.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::native_daemon::PulseEvent;

pub fn build_live_stream(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_attribute("data-pulse-live-stream", "").ok();
    root.set_attribute(
        "data-honesty",
        if super::native_daemon::is_daemon_connected() {
            "running"
        } else {
            "unavailable"
        },
    )
    .ok();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "border: 1px solid var(--border-subtle); border-radius: 4px; padding: 6px; \
         font-family: var(--font-mono); font-size: 9px;",
    );
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    status.set_attribute("data-pulse-live-status", "").ok();
    status.set_text_content(Some(if super::native_daemon::is_daemon_connected() {
        "Waiting for the next live Pulse SSE event…"
    } else {
        "Pulse SSE unavailable until the local daemon is connected."
    }));
    root.append_child(&status).unwrap();
    let list = document.create_element("div").unwrap();
    list.set_attribute("data-pulse-live-list", "").ok();
    root.append_child(&list).unwrap();
    root
}

/// Fan one event from the shared daemon EventSource into every mounted Pulse
/// container, then refresh its persisted COP ledger view.
pub fn render_event(pulse: &PulseEvent) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(roots) = document.query_selector_all("[data-pulse-live-stream]") else {
        return;
    };
    for index in 0..roots.length() {
        let Some(root) = roots
            .item(index)
            .and_then(|node| node.dyn_into::<Element>().ok())
        else {
            continue;
        };
        root.set_attribute("data-honesty", "live").ok();
        if let Some(status) = root
            .query_selector("[data-pulse-live-status]")
            .ok()
            .flatten()
        {
            status.set_text_content(Some(&format!("Live Pulse SSE · sequence {}", pulse.seq)));
        }
        if !pulse.topic.is_empty() {
            if let Some(list) = root.query_selector("[data-pulse-live-list]").ok().flatten() {
                let row = document.create_element("div").unwrap();
                row.set_text_content(Some(&format!(
                    "#{} · {} · {}",
                    pulse.seq, pulse.topic, pulse.payload_summary
                )));
                list.prepend_with_node_1(&row).ok();
                while list.child_element_count() > 20 {
                    if let Some(last) = list.last_element_child() {
                        last.remove();
                    }
                }
            }
            if let Some(parent) = root.parent_element() {
                if let Some(panel) = parent
                    .query_selector("[data-cop-family=\"pulse_event\"]")
                    .ok()
                    .flatten()
                {
                    super::cop_records::refresh_family_panel(&panel);
                }
            }
        }
    }
}
