use super::{
    actor::InferenceActorSession,
    configuration::{self, InferenceHostConfiguration},
    registry,
};
use qualia_cooperative_core::authoring::DocumentRevision;
use qualia_core_db::{
    key_vault::{KeyRole, KeyVault},
    llm_agent::governed::{
        GovernedTokenSession, InferenceAdmission, NativePolicyHost, PolicyError, PolicyRules,
        PolicyStatus, MAX_POLICY_JSON_BYTES, POLICY_COLD_BUDGET_BYTES,
    },
    poet_host::native_authority::{NativeAuthority, SignedAdmission},
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

pub(super) struct ConfiguredHost {
    root: PathBuf,
    configuration_digest: [u8; 32],
    pub owner: Arc<NativeAuthority>,
    pub host: Arc<Mutex<NativePolicyHost>>,
}
impl Drop for ConfiguredHost {
    fn drop(&mut self) {
        self.host
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .disconnect();
    }
}
static HOST: OnceLock<Mutex<Option<ConfiguredHost>>> = OnceLock::new();

/// Explicit logout/controller-loss hook. Dropping the policy host invalidates
/// old admitted sessions; a new login cannot resurrect them.
pub fn disconnect_inference_controller() {
    registry::invalidate();
    super::product::clear();
    if let Some(host) = HOST.get() {
        let mut slot = host.lock().unwrap_or_else(|e| e.into_inner());
        *slot = None;
    }
    registry::revoke_tracked();
}
fn trust_failure(error: impl ToString) -> String {
    disconnect_inference_controller();
    error.to_string()
}
pub(super) fn meta_root() -> Result<PathBuf, String> {
    let root = crate::state::app_meta_dir();
    root.canonicalize()
        .map_err(|_| "Application metadata directory is unavailable".into())
}
pub(super) fn child_directory(root: &Path, name: &str) -> Result<PathBuf, String> {
    let path = root.join(name);
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    let resolved = path.canonicalize().map_err(|e| e.to_string())?;
    if !resolved.starts_with(root) {
        return Err("Inference storage escapes configured metadata directory".into());
    }
    Ok(resolved)
}
pub(super) fn with_vault<T>(
    operation: impl FnOnce(&KeyVault) -> Result<T, String>,
) -> Result<T, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or_else(|| trust_failure("Application is not initialized"))?;
    let vault = state
        .key_vault
        .lock()
        .map_err(|_| trust_failure("Controller key vault lock failed"))?;
    if vault.is_locked() {
        return Err(trust_failure("Unlock the configured controller vault"));
    }
    operation(&vault)
}
pub(super) fn with_host<T>(
    operation: impl FnOnce(
        &mut ConfiguredHost,
        &ed25519_dalek::SigningKey,
        &KeyVault,
    ) -> Result<T, String>,
) -> Result<T, String> {
    with_vault(|vault| {
        let root = meta_root().map_err(trust_failure)?;
        let key = vault.derive_key_for_role(KeyRole::ControllerSigning);
        let (config, document) = configuration::load(&root, &key).map_err(trust_failure)?;
        let digest = configuration::selection_digest(&config, &document);
        let mut slot = match HOST.get_or_init(|| Mutex::new(None)).lock() {
            Ok(s) => s,
            Err(e) => {
                let mut s = e.into_inner();
                *s = None;
                registry::invalidate();
                registry::revoke_tracked();
                return Err("Policy host lock failed; prior controller disconnected".into());
            }
        };
        if slot
            .as_ref()
            .is_some_and(|h| h.root != root || h.configuration_digest != digest)
        {
            *slot = None;
            registry::invalidate();
            registry::revoke_tracked();
        }
        if slot.is_none() {
            let owner = registry::open(&root, &config, &document, None)?;
            let policy = child_directory(&root, "inference-policy")?;
            let authority = child_directory(&root, "inference-policy-authority")?;
            let host = NativePolicyHost::open(
                &policy,
                &authority,
                config.trust()?,
                POLICY_COLD_BUDGET_BYTES,
            )
            .map_err(|e| e.to_string())?;
            *slot = Some(ConfiguredHost {
                root,
                configuration_digest: digest,
                owner,
                host: Arc::new(Mutex::new(host)),
            });
        }
        operation(slot.as_mut().expect("host initialized"), &key, vault)
    })
}
/// Privileged LOCAL host configuration, never an inference tool. Caller chooses
/// an independent project trust anchor and supplies its actual signed admission
/// and source revision. No configuration or policy is synthesized from a login.
/// Typed inputs are caller-owned; earlier parsing allocations are outside this
/// adapter budget. Existing native journal limits remain enforced independently.
pub fn configure_inference_authority(
    config: InferenceHostConfiguration,
    admission: SignedAdmission,
    document: DocumentRevision,
    budget: usize,
) -> Result<(), String> {
    configuration::budget(budget)?;
    with_vault(|vault| {
        let key = vault.derive_key_for_role(KeyRole::ControllerSigning);
        configuration::preflight(&config, &document, &key).map_err(trust_failure)?;
        let root = meta_root().map_err(trust_failure)?;
        // Local registry/epoch capacity refusals occur before invalidating state.
        registry::preflight(&root, &config)?;
        registry::preflight_epoch()?;
        disconnect_inference_controller();
        let result: Result<(), String> = (|| {
            let owner = registry::open(&root, &config, &document, Some(admission))?;
            let policy = child_directory(&root, "inference-policy")?;
            let authority = child_directory(&root, "inference-policy-authority")?;
            let host = NativePolicyHost::open(
                &policy,
                &authority,
                config.trust()?,
                POLICY_COLD_BUDGET_BYTES,
            )
            .map_err(|e| e.to_string())?;
            configuration::save(&root, config.clone(), document.clone(), &key)?;
            let mut slot = HOST
                .get_or_init(|| Mutex::new(None))
                .lock()
                .map_err(|_| "Policy host lock failed")?;
            *slot = Some(ConfiguredHost {
                root,
                configuration_digest: configuration::selection_digest(&config, &document),
                owner,
                host: Arc::new(Mutex::new(host)),
            });
            Ok(())
        })();
        result.map_err(trust_failure)
    })
}
/// A real signed successor is verified by the existing project owner; no signing
/// or enrollment authority is inferred from the acting inference session.
pub fn replace_inference_admission(
    replacement: SignedAdmission,
    budget: usize,
) -> Result<(), String> {
    configuration::budget(budget)?;
    let result = with_host(|h, _, _| {
        h.host
            .lock()
            .map_err(|_| "Policy host lock failed")?
            .disconnect();
        registry::invalidate();
        registry::revoke_tracked();
        h.owner.replace_admission(replacement)
    });
    if result.is_err() {
        disconnect_inference_controller();
    }
    result
}
pub fn install_policy(
    rules_json: &str,
    not_before: u64,
    expires_at: u64,
    now: u64,
    expected_generation: u64,
    budget: usize,
) -> Result<PolicyStatus, String> {
    if budget < POLICY_COLD_BUDGET_BYTES {
        return Err(PolicyError::Budget.to_string());
    }
    if rules_json.len() > MAX_POLICY_JSON_BYTES {
        return Err(PolicyError::Capacity.to_string());
    }
    let rules: PolicyRules =
        serde_json::from_str(rules_json).map_err(|e| format!("Invalid Sentinel policy: {e}"))?;
    with_host(|h, key, _| {
        h.host
            .lock()
            .map_err(|_| "Policy host lock failed")?
            .install(
                key,
                rules,
                not_before,
                expires_at,
                now,
                expected_generation,
                budget,
            )
            .map_err(|e| e.to_string())
    })
}
pub fn load_policy_status() -> Result<PolicyStatus, String> {
    with_host(|h, _, _| {
        Ok(h.host
            .lock()
            .map_err(|_| "Policy host lock failed")?
            .status())
    })
}
/// Install the actual V3 per-token evidence policy required by native products.
/// Observation requirements are explicitly selected by the privileged host.
pub fn install_evidence_policy(
    rules_json: &str,
    observations: qualia_core_db::llm_agent::governed::evidence::ObservationRequirements,
    not_before: u64,
    expires_at: u64,
    expected_generation: u64,
    budget: usize,
) -> Result<PolicyStatus, String> {
    if budget < POLICY_COLD_BUDGET_BYTES || rules_json.len() > MAX_POLICY_JSON_BYTES {
        return Err(PolicyError::Budget.to_string());
    }
    let rules: PolicyRules =
        serde_json::from_str(rules_json).map_err(|e| format!("Invalid Sentinel policy: {e}"))?;
    with_host(|h, key, _| {
        h.host
            .lock()
            .map_err(|_| "Policy host lock failed")?
            .install_evidence(
                key,
                rules,
                observations,
                not_before,
                expires_at,
                expected_generation,
                budget,
            )
            .map_err(|e| e.to_string())
    })
}
pub fn revoke_policy(expected_generation: u64, budget: usize) -> Result<PolicyStatus, String> {
    with_host(|h, key, _| {
        h.host
            .lock()
            .map_err(|_| "Policy host lock failed")?
            .revoke(key, expected_generation, budget)
            .map_err(|e| e.to_string())
    })
}
pub fn admit_inference(
    request: InferenceAdmission,
    actor: &InferenceActorSession,
    budget: usize,
) -> Result<GovernedTokenSession, String> {
    with_host(|h, key, _| {
        if !Arc::ptr_eq(&h.owner, actor.owner()) {
            return Err("Actor belongs to a different configured authority".into());
        }
        let association = actor.bind()?;
        h.host
            .lock()
            .map_err(|_| "Policy host lock failed")?
            .admit(key, request, budget, Arc::clone(&h.owner), association)
            .map_err(|e| e.to_string())
    })
}
/// This native lifecycle entry invalidates policy before returning from vault lock.
pub fn lock_inference_controller_vault() -> Result<(), String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("Application is not initialized")?;
    let mut vault = state
        .key_vault
        .lock()
        .map_err(|_| trust_failure("Controller key vault lock failed"))?;
    vault.lock();
    disconnect_inference_controller();
    Ok(())
}
