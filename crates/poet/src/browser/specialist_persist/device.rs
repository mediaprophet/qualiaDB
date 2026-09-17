//! Device specialist persistence surfaces.

use super::{ledger, CopField};
use web_sys::{Document, Element};

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
