//! The shared prepared semantic request (Prompt Precision route carry).
//!
//! One profile document — the same JSON shape the POET HTTP API accepts under
//! `conditioning` — is persisted at `{storage}/conditioning/active-profile.json`
//! and compiled per turn by the core compiler
//! (`poet_llm_conditioning::prepare_conditioned_prompt`). Every product route
//! then lowers the *same plan*:
//!
//! - native / orchestrated chat: the rendered envelope is prepended to the
//!   augmented prompt before decode;
//! - Ollama: the envelope travels in the native `system` role field, with the
//!   output budget carried as `num_predict`;
//! - remote MCP: `lower_mcp_tool_arguments` produces structured or flattened
//!   tool arguments with an explicit role-degradation receipt.
//!
//! "Same request" means same logical plan and traceability — wire syntax
//! differs per backend capability and receipts say so honestly.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const ACTIVE_PROFILE_DIR: &str = "conditioning";
pub const ACTIVE_PROFILE_FILENAME: &str = "active-profile.json";

pub fn active_profile_path(storage: &Path) -> PathBuf {
    storage.join(ACTIVE_PROFILE_DIR).join(ACTIVE_PROFILE_FILENAME)
}

/// One compiled semantic request — route-neutral plan receipt plus the
/// rendered request text the compiler produced for it.
#[derive(Debug, Clone)]
pub struct PreparedSemanticRequest {
    pub profile_id: String,
    pub plan_id: u64,
    pub selected_evidence: usize,
    pub token_budget: u32,
    pub text: String,
}

/// Per-route lowering receipt attached to inference results. `route` is
/// `"native"`, `"orchestrated"`, `"ollama"`, `"remote_mcp"` or `"http"`;
/// `role_mode` records how the request reached the backend: `"envelope"`
/// (merged into the prompt text), `"system"` (dedicated role field),
/// `"structured"` (separate MCP arguments) or `"flattened"` (single argument).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedRouteReceipt {
    pub profile_id: String,
    pub plan_id: u64,
    pub selected_evidence: usize,
    pub token_budget: u32,
    pub route: String,
    pub role_mode: String,
    pub role_degraded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation_reason: Option<String>,
}

impl PreparedSemanticRequest {
    pub fn receipt(
        &self,
        route: &str,
        role_mode: &str,
        role_degraded: bool,
        degradation_reason: Option<String>,
    ) -> PreparedRouteReceipt {
        PreparedRouteReceipt {
            profile_id: self.profile_id.clone(),
            plan_id: self.plan_id,
            selected_evidence: self.selected_evidence,
            token_budget: self.token_budget,
            route: route.to_string(),
            role_mode: role_mode.to_string(),
            role_degraded,
            degradation_reason,
        }
    }
}

/// The persisted active profile document, if present and well-formed JSON.
pub fn load_active_profile(storage: &Path) -> Option<serde_json::Value> {
    let raw = std::fs::read(active_profile_path(storage)).ok()?;
    serde_json::from_slice(&raw).ok()
}

/// Validate a profile by compiling it once against an empty context, then
/// persist it as the active profile shared by all inference routes.
/// Returns the probe plan receipt.
pub fn activate_profile(
    storage: &Path,
    profile: &serde_json::Value,
) -> Result<PreparedRouteReceipt, String> {
    let probe = prepare_semantic_request(
        profile,
        "profile activation validation",
        "urn:qualia:profile-activation",
        "",
        64,
    )?;
    let path = active_profile_path(storage);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(profile).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(probe.receipt("activation", "envelope", false, None))
}

/// Clear the active profile. Returns whether a profile was present.
pub fn deactivate_profile(storage: &Path) -> bool {
    let path = active_profile_path(storage);
    path.is_file() && std::fs::remove_file(path).is_ok()
}

/// Current activation state for status surfaces.
pub fn active_profile_status(storage: &Path) -> serde_json::Value {
    match load_active_profile(storage) {
        Some(profile) => serde_json::json!({
            "active": true,
            "profile_id": profile.get("profile_id").cloned().unwrap_or(serde_json::Value::Null),
            "path": active_profile_path(storage).to_string_lossy(),
        }),
        None => serde_json::json!({ "active": false }),
    }
}

/// Compile one profile document against one request. Never silently
/// rewrites: failures mean the declared profile cannot run, so callers
/// fail closed instead of dropping enforced requirements.
#[cfg(not(target_arch = "wasm32"))]
pub fn prepare_semantic_request(
    profile: &serde_json::Value,
    prompt: &str,
    principal_did: &str,
    graph_context: &str,
    max_tokens: u32,
) -> Result<PreparedSemanticRequest, String> {
    let prepared = qualia_core_db::services::poet_llm_conditioning::prepare_conditioned_prompt(
        profile,
        prompt,
        principal_did,
        graph_context,
        max_tokens,
    )?;
    Ok(PreparedSemanticRequest {
        profile_id: prepared.profile_id.unwrap_or_default(),
        plan_id: prepared.plan_id.unwrap_or(0),
        selected_evidence: prepared.selected_evidence,
        token_budget: prepared.token_budget,
        text: prepared.text,
    })
}

#[cfg(target_arch = "wasm32")]
pub fn prepare_semantic_request(
    _profile: &serde_json::Value,
    _prompt: &str,
    _principal_did: &str,
    _graph_context: &str,
    _max_tokens: u32,
) -> Result<PreparedSemanticRequest, String> {
    Err("the conditioning compiler is not available on wasm targets".to_string())
}

/// Compile the persisted active profile for this turn.
/// `Ok(None)` when no profile is active — routes keep their legacy shape.
pub fn prepare_active_semantic_request(
    storage: &Path,
    prompt: &str,
    principal_did: &str,
    graph_context: &str,
    max_tokens: u32,
) -> Result<Option<PreparedSemanticRequest>, String> {
    match load_active_profile(storage) {
        Some(profile) => {
            prepare_semantic_request(&profile, prompt, principal_did, graph_context, max_tokens)
                .map(Some)
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "profile_id": "urn:qualia:profile:test-suite",
            "requirements": [
                {"id": "r1", "class": "enforced", "rule": "Cite only supplied evidence", "validator": "provenance", "required": true, "priority": 1}
            ],
            "evidence": [
                {"source_id": "doc:a", "content": "Qualia keeps the final user request intact."}
            ]
        })
    }

    #[test]
    fn activate_compile_persist_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let receipt = activate_profile(dir.path(), &sample_profile()).unwrap();
        assert_eq!(receipt.profile_id, "urn:qualia:profile:test-suite");
        assert!(receipt.plan_id != 0);
        assert!(active_profile_path(dir.path()).is_file());

        let loaded = load_active_profile(dir.path()).unwrap();
        assert_eq!(loaded["profile_id"], "urn:qualia:profile:test-suite");
        assert_eq!(active_profile_status(dir.path())["active"], true);

        assert!(deactivate_profile(dir.path()));
        assert!(load_active_profile(dir.path()).is_none());
        assert!(!deactivate_profile(dir.path()));
    }

    #[test]
    fn prepared_request_carries_plan_and_receipt() {
        let dir = tempfile::tempdir().unwrap();
        activate_profile(dir.path(), &sample_profile()).unwrap();
        let prepared = prepare_active_semantic_request(
            dir.path(),
            "Summarise the evidence",
            "did:example:alice",
            "urn:test:ctx",
            128,
        )
        .unwrap()
        .expect("active profile should compile");

        assert_eq!(prepared.profile_id, "urn:qualia:profile:test-suite");
        assert!(prepared.text.contains("Cite only supplied evidence"));
        assert!(prepared.text.contains("Summarise the evidence"));

        let receipt = prepared.receipt("ollama", "system", false, None);
        assert_eq!(receipt.route, "ollama");
        assert_eq!(receipt.plan_id, prepared.plan_id);
    }

    #[test]
    fn invalid_profile_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let err = activate_profile(dir.path(), &serde_json::json!({"requirements": 42}))
            .unwrap_err();
        assert!(err.contains("Invalid conditioning profile"));
        assert!(load_active_profile(dir.path()).is_none());
    }

    #[test]
    fn no_profile_is_ok_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(prepare_active_semantic_request(dir.path(), "p", "d", "g", 64)
            .unwrap()
            .is_none());
    }
}
