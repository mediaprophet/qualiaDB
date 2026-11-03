//! Explicit V3 installation/admission on the SAME configured host/store/epoch.
use super::*;
use crate::inference::inference_agent::governed::evidence::{
    EvidencePolicy, EvidenceStorage, GovernedEvidenceSession, ObservationRequirements,
    SignedEvidencePolicy, AUDIENCE_V3,
};
use crate::poet_host::native_authority::{
    inference::AuthenticatedInferenceAssociation, NativeAuthority,
};

impl NativePolicyHost {
    /// Privileged local-host operation, never an actor self-grant or remote
    /// signing endpoint. Installer identity comes from configured host trust.
    pub fn install_evidence(
        &mut self,
        key: &SigningKey,
        rules: PolicyRules,
        observations: ObservationRequirements,
        not_before: u64,
        expires_at: u64,
        expected_generation: u64,
        budget: usize,
    ) -> Result<PolicyStatus, PolicyError> {
        check_budget(budget)?;
        self.check_key(key)?;
        self.check_generation(expected_generation)?;
        let generation = self
            .record
            .generation()
            .checked_add(1)
            .ok_or(PolicyError::CounterExhausted)?;
        let base = PolicyDocument {
            version: POLICY_VERSION,
            audience: self.trust.audience,
            principal: self.trust.principal,
            controller_generation: self.trust.controller_generation,
            policy_generation: generation,
            not_before,
            expires_at,
            rules,
        };
        let policy = SignedEvidencePolicy::sign(
            EvidencePolicy {
                version: 3,
                audience: AUDIENCE_V3,
                base,
                observations,
            },
            key,
        )?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| PolicyError::Clock)?
            .as_secs();
        policy.verify(
            &TrustedPolicyIdentity {
                audience: AUDIENCE_V3,
                ..self.trust
            },
            now,
        )?;
        let mut next = self.next_record()?;
        next.set_generation(generation);
        next.install_v3(policy);
        self.persist(&mut next, key)?;
        self.publish(next)?;
        Ok(self.status())
    }
    /// Durable admission with a caller-owned exclusive retained-byte slot.
    /// Earlier JSON parsing/source allocation is outside this storage budget.
    /// No public timestamp or copied ActorPolicyScope substitutes live authority.
    pub fn admit_evidence<'slot, 'bytes>(
        &mut self,
        key: &SigningKey,
        request: InferenceAdmission,
        budget: usize,
        owner: Arc<NativeAuthority>,
        association: AuthenticatedInferenceAssociation,
        storage: &'slot mut EvidenceStorage<'bytes>,
    ) -> Result<GovernedEvidenceSession<'slot, 'bytes>, PolicyError> {
        check_budget(budget)?;
        self.check_key(key)?;
        if self.record.v2().is_some() {
            return Err(PolicyError::PolicyVersionRequired);
        }
        let controller = self
            .evidence_controller
            .as_ref()
            .ok_or(PolicyError::MissingPolicy)?;
        let session_id = self
            .record
            .next_session()
            .checked_add(1)
            .ok_or(PolicyError::CounterExhausted)?;
        let session = GovernedEvidenceSession::admit(
            Arc::clone(controller),
            request,
            session_id,
            owner,
            association,
            storage,
        )?;
        let mut next = self.next_record()?;
        next.set_next_session(session_id);
        self.persist(&mut next, key)?;
        // Bookkeeping does not reconnect or change authority epoch.
        self.record = next;
        Ok(session)
    }
}
