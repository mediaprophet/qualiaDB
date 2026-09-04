//! Social specialist persistence surfaces.

use super::{ledger, CopField};
use web_sys::{Document, Element};

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
