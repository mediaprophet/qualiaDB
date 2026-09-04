//! Remaining specialist surfaces persist as COP session records.
//!
//! Social, presence, finance, Aura, WebRTC, vision, listen, triad, portal,
//! webview, governance, and device are POET containers - not nested apps.

use web_sys::{Document, Element};

use super::cop_records::{build_family_panel, CopField};
use super::live_invoke;

pub(super) fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.append_child(&child).unwrap();
    wrapper
}

pub(super) fn ledger(
    document: &Document,
    family: &'static str,
    heading: &str,
    fields: &'static [CopField],
    actions: &[(&str, &'static str, serde_json::Value)],
) -> Element {
    let panel = build_family_panel(document, family, heading, fields);
    if !actions.is_empty() {
        panel
            .append_child(&live_invoke::action_bar(document, actions))
            .unwrap();
    }
    wrap(document, panel)
}

pub(super) fn gray_ahash_args() -> serde_json::Value {
    serde_json::json!({
        "bytes": vec![128u64; 64],
        "width": 8,
        "height": 8
    })
}

pub(super) fn hu_window_demo_args() -> serde_json::Value {
    serde_json::json!({
        "study_uid": "urn:poet:anatomy:demo-slice",
        "width": 2,
        "height": 2,
        "pixels": [-160.0, 40.0, 240.0, 1000.0],
        "window": 400.0,
        "level": 40.0
    })
}

pub(super) fn gbm_var_args() -> serde_json::Value {
    serde_json::json!({
        "s0": 100.0,
        "mu": 0.05,
        "sigma": 0.2,
        "time_horizon": 1.0,
        "dt": 0.01,
        "portfolio_value": 100000.0,
        "confidence": 0.95,
        "paths": 256,
        "seed": 42
    })
}

mod social;
mod sessions;
mod governance;
mod device;

pub use device::*;
pub use governance::*;
pub use sessions::*;
pub use social::*;

#[cfg(test)]
mod tests {
    #[test]
    fn specialist_families_cover_remaining_j_surfaces() {
        let families = [
            "social_message",
            "presence",
            "channel",
            "finance_account",
            "aura_validation",
            "webrtc_session",
            "vision_job",
            "listen_session",
            "triad_session",
            "portal_nav",
            "webview_session",
            "gov_meeting",
            "device",
            "wallet_entry",
        ];
        assert_eq!(families.len(), 14);
    }
}
