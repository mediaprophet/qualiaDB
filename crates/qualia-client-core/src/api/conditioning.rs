//! Active semantic conditioning profile (Prompt Precision).
//!
//! The profile document is the same JSON shape the POET HTTP API accepts under
//! `conditioning`. Once activated it is compiled per turn and lowered for
//! every inference route — native chat, orchestrated graph-mutation decode,
//! Ollama, and remote MCP — with a per-route receipt on each result.

use std::path::Path;

fn conditioning_storage() -> Result<String, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("Application not initialized")?;
    let storage = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .storage_path
        .clone();
    Ok(storage)
}

/// Validate (by compiling) and activate a semantic conditioning profile for
/// all inference routes. The prior profile is replaced atomically enough for
/// a single-user store: activation only succeeds when the new profile
/// compiles, so a bad document can never displace a working one.
pub fn conditioning_profile_activate(profile_json: String) -> Result<serde_json::Value, String> {
    let storage = conditioning_storage()?;
    let profile: serde_json::Value =
        serde_json::from_str(&profile_json).map_err(|e| format!("invalid profile JSON: {e}"))?;
    let receipt =
        crate::conditioning::activate_profile(Path::new(&storage), &profile)?;
    serde_json::to_value(serde_json::json!({
        "active": true,
        "profile_id": receipt.profile_id,
        "plan_id": receipt.plan_id,
        "selected_evidence": receipt.selected_evidence,
        "token_budget": receipt.token_budget,
    }))
    .map_err(|e| e.to_string())
}

/// Deactivate the active profile. Routes revert to their unconditioned shape.
pub fn conditioning_profile_deactivate() -> Result<serde_json::Value, String> {
    let storage = conditioning_storage()?;
    let deactivated = crate::conditioning::deactivate_profile(Path::new(&storage));
    Ok(serde_json::json!({ "active": false, "deactivated": deactivated }))
}

/// Current activation state (`active`, `profile_id`, store path).
pub fn conditioning_profile_status() -> Result<serde_json::Value, String> {
    let storage = conditioning_storage()?;
    Ok(crate::conditioning::active_profile_status(Path::new(&storage)))
}
