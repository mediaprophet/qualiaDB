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
pub fn install_native_inference_policy(
    rules_json: String,
    observations: qualia_core_db::llm_agent::governed::evidence::ObservationRequirements,
    expires_at: u64,
    expected_generation: u64,
) -> Result<serde_json::Value, String> {
    crate::inference_policy::install_evidence_policy(
        &rules_json,
        observations,
        now()?,
        expires_at,
        expected_generation,
        POLICY_COLD_BUDGET_BYTES,
    )
    .map(status)
}

/// Local host command connecting an already logged-in actor to PoET's existing
/// token-authenticated HTTP transport. No signing API is exposed remotely.
pub fn bind_poet_inference_actor(
    actor: &crate::inference_policy::InferenceActorSession,
    preparation: qualia_core_db::llm_agent::native_request::NativePreparationSpec,
) -> Result<(), String> {
    crate::inference_policy::bind_poet_inference_session(actor, preparation)
}

/// Explicit delegation to the native MCP service; never remotely callable.
pub fn bind_mcp_inference_actor(
    actor: &crate::inference_policy::InferenceActorSession,
    preparation: qualia_core_db::llm_agent::native_request::NativePreparationSpec,
) -> Result<(), String> {
    crate::inference_policy::bind_mcp_inference_session(actor, preparation)
}

/// Obtain the retained context consumed by an embedding's native session API.
/// In particular the CLI must inject it after loading its selected model.
pub fn get_native_inference_session(
    actor: &crate::inference_policy::InferenceActorSession,
    preparation: qualia_core_db::llm_agent::native_request::NativePreparationSpec,
) -> Result<std::sync::Arc<qualia_core_db::llm_agent::native_request::NativeProductSession>, String> {
    crate::inference_policy::native_product_session(actor, preparation)
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
/// Login and connect that authenticated actor to a specific local chat. The
/// native embedding supplies retained source leases and explicit input budgets.
pub fn login_chat_inference_actor(
    chat_id: String,
    subject: String,
    selected: crate::inference_policy::SelectedInferenceIdentifier,
    signer_label: String,
    lifetime_seconds: u64,
    preparation: qualia_core_db::llm_agent::native_request::NativePreparationSpec,
) -> Result<crate::inference_policy::InferenceActorSession, String> {
    let actor = login_inference_actor(subject, selected, signer_label, lifetime_seconds)?;
    if let Err(error) =
        crate::inference_policy::bind_chat_inference_session(&chat_id, &actor, preparation)
    {
        let _ = actor.logout();
        return Err(error);
    }
    Ok(actor)
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
