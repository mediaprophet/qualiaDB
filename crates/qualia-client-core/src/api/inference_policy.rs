//! Privileged native configuration commands. Deliberately not exposed as an
//! unauthenticated HTTP/PoetHost/MCP signing endpoint. The embedding native UI
//! must authorize the human command, as it does other identity/profile changes.
use qualia_core_db::llm_agent::governed::POLICY_COLD_BUDGET_BYTES;

fn now() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| "Host clock precedes Unix epoch".into())
}
fn status(value: qualia_core_db::llm_agent::governed::PolicyStatus) -> serde_json::Value {
    serde_json::json!({"installed":value.installed,"generation":value.generation,
        "serial":value.serial,"policy_digest":value.digest})
}
/// Deliberate policy installation; absence never selects permissive defaults.
pub fn install_inference_policy(
    rules_json: String,
    expires_at: u64,
    expected_generation: u64,
) -> Result<serde_json::Value, String> {
    let now = now()?;
    crate::inference_policy::install_policy(
        &rules_json,
        now,
        expires_at,
        now,
        expected_generation,
        POLICY_COLD_BUDGET_BYTES,
    )
    .map(status)
}
pub fn get_inference_policy_status() -> Result<serde_json::Value, String> {
    crate::inference_policy::load_policy_status().map(status)
}
pub fn revoke_inference_policy(expected_generation: u64) -> Result<serde_json::Value, String> {
    crate::inference_policy::revoke_policy(expected_generation, POLICY_COLD_BUDGET_BYTES)
        .map(status)
}

/// Native operator command: choose project trust and actual signed admission.
/// These typed inputs are not accepted from chat, tools, HTTP, or remote signers.
pub fn configure_inference_host(
    configuration: crate::inference_policy::InferenceHostConfiguration,
    admission: qualia_core_db::poet_host::native_authority::SignedAdmission,
    document: qualia_cooperative_core::authoring::DocumentRevision,
) -> Result<(), String> {
    crate::inference_policy::configure_inference_authority(
        configuration,
        admission,
        document,
        crate::inference_policy::CONFIGURATION_BUDGET_BYTES,
    )
}
/// Explicit local key creation, separate from enrollment and policy installation.
pub fn create_inference_actor_key(label: String) -> Result<[u8; 32], String> {
    crate::inference_policy::create_inference_actor_signer(
        &label,
        crate::inference_policy::CONFIGURATION_BUDGET_BYTES,
    )
}
pub fn login_inference_actor(
    subject: String,
    selected: crate::inference_policy::SelectedInferenceIdentifier,
    signer_label: String,
    lifetime_seconds: u64,
) -> Result<crate::inference_policy::InferenceActorSession, String> {
    crate::inference_policy::login_inference_actor(
        &subject,
        selected,
        &signer_label,
        lifetime_seconds,
        crate::inference_policy::CONFIGURATION_BUDGET_BYTES,
    )
}
pub fn get_inference_actor_scope(
    actor: &crate::inference_policy::InferenceActorSession,
) -> Result<qualia_core_db::llm_agent::governed::ActorPolicyScope, String> {
    actor.policy_scope()
}
pub fn logout_inference_actor(
    actor: &crate::inference_policy::InferenceActorSession,
) -> Result<(), String> {
    actor.logout()
}
pub fn disconnect_inference_controller() {
    crate::inference_policy::disconnect_inference_controller();
}
pub fn lock_inference_controller_vault() -> Result<(), String> {
    crate::inference_policy::lock_inference_controller_vault()
}
pub fn replace_inference_actor_admission(
    admission: qualia_core_db::poet_host::native_authority::SignedAdmission,
) -> Result<(), String> {
    crate::inference_policy::replace_inference_admission(
        admission,
        crate::inference_policy::CONFIGURATION_BUDGET_BYTES,
    )
}
