//! Session specialist persistence surfaces.

use super::{ledger, CopField, gbm_var_args, gray_ahash_args, hu_window_demo_args};
use web_sys::{Document, Element};

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
