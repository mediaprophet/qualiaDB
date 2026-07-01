use super::vault::VaultService;
use super::policy::PolicyDecisionService;
use wellfare_core::record::RecordEnvelope;
use ed25519_dalek::SigningKey;

/// Transport-neutral Host API exported for UI and qApps
pub struct WebizenHostApi {
    vault: VaultService,
    policy: PolicyDecisionService,
    signing_key: SigningKey,
}

impl WebizenHostApi {
    pub fn new(vault: VaultService, policy: PolicyDecisionService, signing_key: SigningKey) -> Self {
        Self { vault, policy, signing_key }
    }

    pub fn submit_record(&mut self, qapp_id: &str, envelope: RecordEnvelope) -> Result<usize, String> {
        let decision = self.policy.evaluate_access(
            qapp_id,
            "write_record",
            envelope.sensitivity,
            envelope.epistemic_status,
        );

        if let super::policy::DecisionResult::Deny { reason } = decision {
            return Err(format!("Policy Denied: {}", reason));
        }

        let principal_did = 0; // Default or fetched from auth context
        self.vault.commit_envelope(&envelope, &self.signing_key, principal_did)
            .map_err(|e| e.to_string())
    }
}
