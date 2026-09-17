//! Dual-path Tool Chest action for `CooperativeDelegation.permits`.

use serde_json::json;
use web_sys::Document;

const CAP: &str = "CooperativeDelegation.permits";

/// Live ABAC permit check — sample welfare-domain read under a granted delegation.
pub(super) fn run_delegation_permits(document: &Document, label: &str) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(
            CAP,
            "Local sketch: cooperative ABAC would fail-closed until QualiaDB is connected. Agency.evaluate is Ed25519 verify — this tool uses CooperativeDelegation.permits.",
        );
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        "Running CooperativeDelegation.permits…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let domain = "urn:qualia:agency-domain:welfare:personal_welfare";
        let args = json!({
            "delegation": {
                "id": "poet-demo-delegation",
                "principal_did": "did:wf:alice",
                "agent_dids": [],
                "domain": domain,
                "values_anchor": "urn:un:hr:udhr",
                "scope": [],
                "precedence": "primary",
                "valid_from_unix": 100,
                "consent": "granted",
                "revoked": false,
                "transfer_schedule": []
            },
            "request": {
                "domain": domain,
                "data_class": "overview",
                "action": "read",
                "sphere": "personhood"
            },
            "context": {
                "now_unix": 200,
                "occurred_events": [],
                "attestations": []
            }
        });
        match super::native_daemon::daemon_invoke(CAP, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(CAP, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    CAP,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("CooperativeDelegation.permits failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(CAP, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}
