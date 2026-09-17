//! Pulse stream container — live Pulse.publish + COP pulse_event ledger.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::browser::cop_records::{build_family_panel, CopField};
use crate::browser::live_invoke;

/// Pulse stream container — live Pulse.publish + COP pulse_event ledger.
pub fn build_pulse_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(
        "Topics must be poet/, pulse/, or clinic/. Unprefixed channels are rewritten to poet/{channel}. The log is the COP pulse_event ledger, not a canned stream.",
    ));
    let note_el: HtmlElement = note.clone().dyn_into().unwrap();
    note_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); padding: 4px 8px;",
    );
    wrapper.append_child(&note).unwrap();

    let panel = build_family_panel(
        document,
        "pulse_event",
        "Pulse events persist here after Pulse.publish. Empty until you publish or save.",
        &[
            CopField {
                key: "channel",
                placeholder: "Channel (poet/social)",
            },
            CopField {
                key: "payload_type",
                placeholder: "Payload type (agent-message|telemetry|presence)",
            },
        ],
    );
    panel
        .append_child(&live_invoke::action_bar(
            document,
            &[
                (
                    "Pulse.publish",
                    "Pulse.publish",
                    serde_json::json!({ "channel": "poet/pulse", "payload_type": "generic" }),
                ),
                (
                    "Pulse.publish_telemetry",
                    "Pulse.publish_telemetry",
                    serde_json::json!({ "channel": "poet/telemetry" }),
                ),
                (
                    "Pulse.open_channel",
                    "Pulse.open_channel",
                    serde_json::json!({ "channel": "poet/pulse", "channel_type": "topic" }),
                ),
            ],
        ))
        .unwrap();
    wrapper
        .append_child(&crate::browser::pulse_stream::build_live_stream(document))
        .unwrap();
    wrapper.append_child(&panel).unwrap();
    wrapper
}
