//! Remaining specialist surfaces persist as COP session records.
//!
//! Social, presence, finance, Aura, WebRTC, vision, listen, triad, portal,
//! webview, governance, and device are POET containers — not nested apps.

use web_sys::{Document, Element};

use super::cop_records::{build_family_panel, CopField};
use super::live_invoke;

fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.append_child(&child).unwrap();
    wrapper
}

fn ledger(
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

fn gray_ahash_args() -> serde_json::Value {
    serde_json::json!({
        "bytes": vec![128u64; 64],
        "width": 8,
        "height": 8
    })
}

fn hu_window_demo_args() -> serde_json::Value {
    serde_json::json!({
        "study_uid": "urn:poet:anatomy:demo-slice",
        "width": 2,
        "height": 2,
        "pixels": [-160.0, 40.0, 240.0, 1000.0],
        "window": 400.0,
        "level": 40.0
    })
}

fn gbm_var_args() -> serde_json::Value {
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

pub fn build_social_view(document: &Document) -> Element {
    ledger(
        document,
        "social_message",
        "Social messages persist on the COP ledger. Pulse.publish sends on the social channel.",
        &[
            CopField {
                key: "from",
                placeholder: "From DID",
            },
            CopField {
                key: "body",
                placeholder: "Body",
            },
        ],
        &[(
            "Pulse.publish",
            "Pulse.publish",
            serde_json::json!({ "channel": "poet/social", "payload_type": "agent-message" }),
        )],
    )
}

pub fn build_connection_requests_view(document: &Document) -> Element {
    ledger(
        document,
        "social_request",
        "Connection requests persist here. Signing requires an unlocked identity session.",
        &[
            CopField {
                key: "from",
                placeholder: "From DID",
            },
            CopField {
                key: "status",
                placeholder: "Status (pending|accepted|denied)",
            },
        ],
        &[(
            "Pulse.publish_notification",
            "Pulse.publish_notification",
            serde_json::json!({ "channel": "poet/social-requests" }),
        )],
    )
}

pub fn build_reputation_view(document: &Document) -> Element {
    ledger(
        document,
        "social_reputation",
        "Reputation receipts you record. Live projection needs signed contribution receipts.",
        &[
            CopField {
                key: "subject",
                placeholder: "Subject DID",
            },
            CopField {
                key: "note",
                placeholder: "Note",
            },
        ],
        &[(
            "Pulse.publish_telemetry",
            "Pulse.publish_telemetry",
            serde_json::json!({ "channel": "poet/reputation" }),
        )],
    )
}

pub fn build_presence_view(document: &Document) -> Element {
    ledger(
        document,
        "presence",
        "Presence roster records. Pulse.publish_presence is the live capability when the daemon is up.",
        &[
            CopField {
                key: "did",
                placeholder: "DID",
            },
            CopField {
                key: "status",
                placeholder: "Status (here|away)",
            },
        ],
        &[(
            "Pulse.publish_presence",
            "Pulse.publish_presence",
            serde_json::json!({ "channel": "poet/presence" }),
        )],
    )
}

pub fn build_channels_view(document: &Document) -> Element {
    ledger(
        document,
        "channel",
        "Channel directory records. Pulse.open_channel is the live capability when the daemon is up.",
        &[
            CopField {
                key: "name",
                placeholder: "Channel name",
            },
            CopField {
                key: "members",
                placeholder: "Members",
            },
        ],
        &[(
            "Pulse.open_channel",
            "Pulse.open_channel",
            serde_json::json!({ "channel": "poet/channel", "channel_type": "topic" }),
        )],
    )
}

pub fn build_conversations_view(document: &Document) -> Element {
    ledger(
        document,
        "social_message",
        "Conversation threads persist as social_message records.",
        &[
            CopField {
                key: "thread",
                placeholder: "Thread",
            },
            CopField {
                key: "body",
                placeholder: "Body",
            },
        ],
        &[(
            "Pulse.publish",
            "Pulse.publish",
            serde_json::json!({ "channel": "poet/conversations", "payload_type": "agent-message" }),
        )],
    )
}

pub fn build_settings_view(document: &Document) -> Element {
    ledger(
        document,
        "settings_pref",
        "Host preferences persist on the COP ledger.",
        &[
            CopField {
                key: "key",
                placeholder: "Key",
            },
            CopField {
                key: "value",
                placeholder: "Value",
            },
        ],
        &[(
            "CapabilityDiscovery.list",
            "CapabilityDiscovery.list",
            serde_json::json!({}),
        )],
    )
}

pub fn build_capabilities_view(document: &Document) -> Element {
    ledger(
        document,
        "capability_grant",
        "Capability grants persist here. Runtime enforcement is the Sentinel session.",
        &[
            CopField {
                key: "capability",
                placeholder: "Capability id",
            },
            CopField {
                key: "status",
                placeholder: "Status (granted|revoked)",
            },
        ],
        &[(
            "CapabilityDiscovery.list",
            "CapabilityDiscovery.list",
            serde_json::json!({}),
        )],
    )
}

pub fn build_protection_policies_view(document: &Document) -> Element {
    ledger(
        document,
        "policy_rule",
        "Sentinel policy records. Dry-run lives in Sentinel Policy Studio.",
        &[
            CopField {
                key: "modality",
                placeholder: "OBLIGATE|PERMIT|FORBID",
            },
            CopField {
                key: "status",
                placeholder: "Status (draft|armed)",
            },
        ],
        &[(
            "DeonticLogic.evaluate",
            "DeonticLogic.evaluate",
            serde_json::json!({ "party": "local", "now": 0 }),
        )],
    )
}

pub fn build_finance_view(document: &Document) -> Element {
    ledger(
        document,
        "finance_account",
        "Finance accounts you record. FinancialModeling.* needs entered numbers; none are invented.",
        &[
            CopField {
                key: "currency",
                placeholder: "Currency",
            },
            CopField {
                key: "balance",
                placeholder: "Balance (as reported)",
            },
        ],
        &[(
            "FinancialModeling.gbm_var",
            "FinancialModeling.gbm_var",
            gbm_var_args(),
        )],
    )
}

pub fn build_wallet_view(document: &Document) -> Element {
    ledger(
        document,
        "wallet_entry",
        "Wallet entries persist here. ILP/Lightning settlement is unbound until a rail session is registered.",
        &[
            CopField {
                key: "rail",
                placeholder: "Rail (local|ilp|lightning)",
            },
            CopField {
                key: "amount",
                placeholder: "Amount",
            },
        ],
        &[(
            "FinancialModeling.gbm_var",
            "FinancialModeling.gbm_var",
            gbm_var_args(),
        )],
    )
}

pub fn build_aura_view(document: &Document) -> Element {
    ledger(
        document,
        "aura_validation",
        "Aura SHACL sessions persist here. SHACL.validate runs against the live graph when the daemon is up.",
        &[
            CopField {
                key: "shape",
                placeholder: "Shape / subject",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
        &[(
            "SHACL.extensions",
            "SHACL.extensions",
            serde_json::json!({}),
        )],
    )
}

pub fn build_webview_view(document: &Document) -> Element {
    ledger(
        document,
        "webview_session",
        "Sandboxed navigation records. External fetch is not opened until a session contract is saved.",
        &[
            CopField {
                key: "uri",
                placeholder: "URI",
            },
            CopField {
                key: "status",
                placeholder: "Status (queued|blocked)",
            },
        ],
        &[(
            "Document.ingest",
            "Document.ingest",
            serde_json::json!({ "text": "sandbox navigation record", "uri": "urn:poet:webview" }),
        )],
    )
}

pub fn build_webrtc_view(document: &Document) -> Element {
    ledger(
        document,
        "webrtc_session",
        "WebRTC session records (peer DID, signaling notes). A live RTCDataChannel needs a signaling host.",
        &[
            CopField {
                key: "peer",
                placeholder: "Peer DID",
            },
            CopField {
                key: "status",
                placeholder: "Status (queued|signaling-unbound)",
            },
        ],
        &[(
            "Pulse.publish_sync",
            "Pulse.publish_sync",
            serde_json::json!({ "channel": "poet/webrtc" }),
        )],
    )
}

pub fn build_vision_view(document: &Document) -> Element {
    ledger(
        document,
        "vision_job",
        "Vision jobs you queue. ComputerVision.* runs when a media buffer is supplied; no fabricated detections.",
        &[
            CopField {
                key: "source",
                placeholder: "Source URI",
            },
            CopField {
                key: "status",
                placeholder: "Status (queued|unbound-buffer)",
            },
        ],
        &[(
            "ComputerVision.ahash",
            "ComputerVision.ahash",
            gray_ahash_args(),
        )],
    )
}

pub fn build_listen_view(document: &Document) -> Element {
    ledger(
        document,
        "listen_session",
        "Listen sessions persist here. Mic capture requires an explicit device-permission session.",
        &[
            CopField {
                key: "device",
                placeholder: "Device id",
            },
            CopField {
                key: "status",
                placeholder: "Status (queued|permission-required)",
            },
        ],
        &[(
            "Audio.oscillator",
            "Audio.oscillator",
            serde_json::json!({
                "waveform": "sine",
                "frequency": 440.0,
                "sample_rate": 44100.0,
                "n": 256
            }),
        )],
    )
}

pub fn build_triad_view(document: &Document) -> Element {
    ledger(
        document,
        "triad_session",
        "Triad orchestration records (q42/p64/d10). Agent endpoints are unbound until registered.",
        &[
            CopField {
                key: "q42",
                placeholder: "q42 endpoint",
            },
            CopField {
                key: "status",
                placeholder: "Status (draft|armed)",
            },
        ],
        &[(
            "CapabilityDiscovery.list",
            "CapabilityDiscovery.list",
            serde_json::json!({}),
        )],
    )
}

pub fn build_portal_view(document: &Document) -> Element {
    ledger(
        document,
        "portal_nav",
        "Portal destinations persist here. Navigation needs a typed destination manifold.",
        &[
            CopField {
                key: "destination",
                placeholder: "Destination manifold",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
        &[(
            "Scene.create",
            "Scene.create",
            serde_json::json!({ "name": "portal-destination" }),
        )],
    )
}

pub fn build_3d_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "3D viewport session notes. GPU frames live in Dual Studio.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (viewport)",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
        &[(
            "Render.gpu_adapter_info",
            "Render.gpu_adapter_info",
            serde_json::json!({}),
        )],
    )
}

pub fn build_health_vault_view(document: &Document) -> Element {
    ledger(
        document,
        "health_note",
        "Generic health vault. Prefer Health overview / documents / share containers.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (vault)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity (classified)",
            },
        ],
        &[(
            "ClinicalRisk.cha2ds2_vasc",
            "ClinicalRisk.cha2ds2_vasc",
            serde_json::json!({ "age": 65, "sex_female": false }),
        )],
    )
}

pub fn build_anatomy_view(document: &Document) -> Element {
    ledger(
        document,
        "health_note",
        "Anatomy session notes. Consent-gated; no fabricated physiology.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (anatomy)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity (classified)",
            },
        ],
        &[(
            "MedicalImaging.hu_window",
            "MedicalImaging.hu_window",
            hu_window_demo_args(),
        )],
    )
}

pub fn build_meetings_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_meeting",
        "Governance meetings persist as COP records. DeonticLogic.evaluate scans the live graph.",
        &[
            CopField {
                key: "when",
                placeholder: "When",
            },
            CopField {
                key: "quorum",
                placeholder: "Quorum",
            },
        ],
        &[(
            "DeonticLogic.evaluate",
            "DeonticLogic.evaluate",
            serde_json::json!({ "party": "local", "now": 0 }),
        )],
    )
}

pub fn build_disputes_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_dispute",
        "Disputes persist as COP records. LegalLogic.compute(jural) inspects Hohfeld correlatives.",
        &[
            CopField {
                key: "parties",
                placeholder: "Parties",
            },
            CopField {
                key: "status",
                placeholder: "Status (open|resolved)",
            },
        ],
        &[(
            "LegalLogic.compute",
            "LegalLogic.compute",
            serde_json::json!({ "mode": "jural", "role": "principal" }),
        )],
    )
}

pub fn build_complaints_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_complaint",
        "Complaints persist as COP records. LegalLogic.compute(responsibility) is the live scan.",
        &[
            CopField {
                key: "against",
                placeholder: "Against DID",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
        &[(
            "LegalLogic.compute",
            "LegalLogic.compute",
            serde_json::json!({ "mode": "responsibility" }),
        )],
    )
}

pub fn build_coi_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_coi",
        "Conflict-of-interest declarations persist as COP records. Capacity/duress is LegalLogic.compute.",
        &[
            CopField {
                key: "declarant",
                placeholder: "Declarant DID",
            },
            CopField {
                key: "status",
                placeholder: "Status (declared|mitigated)",
            },
        ],
        &[(
            "LegalLogic.compute",
            "LegalLogic.compute",
            serde_json::json!({ "mode": "capacity" }),
        )],
    )
}

pub fn build_corrections_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_correction",
        "Corrections are append-only COP records. Originals are superseded, not deleted.",
        &[
            CopField {
                key: "supersedes",
                placeholder: "Supersedes id",
            },
            CopField {
                key: "body",
                placeholder: "Correction",
            },
        ],
        &[(
            "DeonticLogic.evaluate",
            "DeonticLogic.evaluate",
            serde_json::json!({ "party": "local", "now": 0 }),
        )],
    )
}

pub fn build_device_manager_view(document: &Document) -> Element {
    ledger(
        document,
        "device",
        "Paired devices persist as records. WebRTC pairing needs a signaling session.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (pair)",
            },
            CopField {
                key: "did",
                placeholder: "Device DID",
            },
            CopField {
                key: "status",
                placeholder: "Status (paired|pending)",
            },
        ],
        &[(
            "Pulse.publish_sync",
            "Pulse.publish_sync",
            serde_json::json!({ "channel": "poet/device" }),
        )],
    )
}

pub fn build_display_layout_view(document: &Document) -> Element {
    ledger(
        document,
        "device",
        "Display layout records. Multi-window OS APIs are unbound; layouts persist here.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (layout)",
            },
            CopField {
                key: "monitors",
                placeholder: "Monitor count",
            },
        ],
        &[(
            "Render.gpu_adapter_info",
            "Render.gpu_adapter_info",
            serde_json::json!({}),
        )],
    )
}

pub fn build_workspace_sync_view(document: &Document) -> Element {
    ledger(
        document,
        "device",
        "Workspace sync notes. Live CRDT sync needs an open data-channel session.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (sync)",
            },
            CopField {
                key: "peer",
                placeholder: "Peer DID",
            },
        ],
        &[(
            "Pulse.publish_sync",
            "Pulse.publish_sync",
            serde_json::json!({ "channel": "poet/sync" }),
        )],
    )
}

pub fn build_device_role_assigner_view(document: &Document) -> Element {
    ledger(
        document,
        "device",
        "Device roles persist as records.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (role)",
            },
            CopField {
                key: "role",
                placeholder: "Role",
            },
            CopField {
                key: "device",
                placeholder: "Device DID",
            },
        ],
        &[(
            "DeonticLogic.evaluate",
            "DeonticLogic.evaluate",
            serde_json::json!({ "modality": "permit", "body": "device:role" }),
        )],
    )
}

pub fn build_remote_control_view(document: &Document) -> Element {
    ledger(
        document,
        "device",
        "Remote-control intents persist as records. They do not move another device until a session is paired.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (remote)",
            },
            CopField {
                key: "action",
                placeholder: "Action",
            },
        ],
        &[(
            "Pulse.publish_sync",
            "Pulse.publish_sync",
            serde_json::json!({ "channel": "poet/remote" }),
        )],
    )
}

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
