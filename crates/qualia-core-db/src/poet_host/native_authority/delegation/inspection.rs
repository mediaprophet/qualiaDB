use super::{
    super::*,
    verify::{verify_chain, ControllerRoot},
};
use qualia_cooperative_core::authoring::delegation::{
    check_delegated_operation, DelegatedOperation, SignedAuthoringDelegation,
};

/// An inspected candidate, deliberately not serializable or a dispatch permit.
/// No actual leaf login, durable adoption or resource consumption is asserted.
#[derive(Debug)]
pub struct ControllerDelegationInspection {
    pub project_admitted_root_subject: String,
    pub root_capacity: PrincipalKind,
    pub root_admission_ref: String,
    pub acting_agent_subject: String,
    pub delegation_path: Vec<[u8; 32]>,
    pub target: DocumentRevision,
    pub authority_generation: u64,
    /// Present only when this inspection also verified the live leaf session.
    /// This evidence is a snapshot, never a dispatch permit or adopted grant.
    pub leaf_authentication_ref: Option<String>,
    pub leaf_authenticated_until_unix: Option<u64>,
    root_controller: [u8; 32],
    chain_controllers: Vec<[u8; 32]>,
}

impl ControllerDelegationInspection {
    /// Positive overlap establishes a review conflict under a policy requiring
    /// separate controllers. False means only that this evidence did not prove
    /// overlap; it does not prove separate humans or independent acceptance.
    pub fn has_admitted_control_overlap(&self, other: &Self) -> bool {
        self.root_controller == other.root_controller
            || self.chain_controllers.iter().any(|key| other.chain_controllers.contains(key))
            || (self.target.project_id == other.target.project_id
                && (self.project_admitted_root_subject == other.project_admitted_root_subject
                    || self
                        .delegation_path
                        .iter()
                        .any(|d| other.delegation_path.contains(d))))
    }
}

impl NativeAuthority {
    /// Current authenticated principal explicitly inspects exact signed consent
    /// bytes and an operation. This is the backend seam for later durable
    /// adoption and delegated login; success authorizes no native side effect.
    pub fn inspect_controller_delegation(
        &self,
        root_session: &SessionHandle,
        chain: &[SignedAuthoringDelegation],
        operation: &DelegatedOperation<'_>,
    ) -> Result<ControllerDelegationInspection, String> {
        let state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let now = now()?;
        self.inspect_controller_delegation_locked(&state, root_session, chain, operation, now)
    }

    /// Inspect a candidate and a separately authenticated admitted leaf under
    /// the same owner lock. This proves the leaf session at this instant only:
    /// there is no durable adoption, delegated execution, revocation tracking,
    /// budget consumption or authorization to publish. The ordinary direct
    /// project session remains independent of this inspection.
    pub fn inspect_authenticated_controller_delegation(
        &self,
        root_session: &SessionHandle,
        leaf_session: &SessionHandle,
        chain: &[SignedAuthoringDelegation],
        operation: &DelegatedOperation<'_>,
    ) -> Result<ControllerDelegationInspection, String> {
        let state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let now = now()?;
        let (authentication_ref, authenticated_until_unix) =
            self.view(&state, leaf_session, now, |view| {
                let principal = view.principal.ok_or("native leaf principal unavailable")?;
                let admitted = state.durable.admission.snapshot.subjects.iter()
                    .find(|subject| subject.subject == principal.subject)
                    .ok_or("native leaf admission missing")?;
                if principal.kind != PrincipalKind::SoftwareAgent
                    || principal.subject != operation.actor_subject
                    || &admitted.controller_public_key != operation.actor_controller
                {
                    return Err("live leaf session does not match requested agent/controller".into());
                }
                Ok((principal.authentication_ref.to_owned(), principal.authenticated_until_unix))
            })?;
        let mut inspected = self.inspect_controller_delegation_locked(
            &state, root_session, chain, operation, now,
        )?;
        inspected.leaf_authentication_ref = Some(authentication_ref);
        inspected.leaf_authenticated_until_unix = Some(authenticated_until_unix);
        Ok(inspected)
    }

    fn inspect_controller_delegation_locked(
        &self,
        state: &State,
        root_session: &SessionHandle,
        chain: &[SignedAuthoringDelegation],
        operation: &DelegatedOperation<'_>,
        now: u64,
    ) -> Result<ControllerDelegationInspection, String> {
        self.view(state, root_session, now, |view| {
            let principal = view.principal.ok_or("native principal unavailable")?;
            // Controller consent cannot enlarge the root's actual project
            // rights. Reuse the current authoring owner for this exact action,
            // source and generation while the same native lock is held.
            authorize(
                view,
                &AuthoringRequest {
                    action: operation.action,
                    target: operation.target,
                    expected_authority_generation: view.generation,
                },
            )
            .map_err(|e| e.to_string())?;
            let root = state
                .durable
                .admission
                .snapshot
                .subjects
                .iter()
                .find(|s| s.subject == principal.subject)
                .ok_or("native root admission missing")?;
            if operation.target != view.current_document {
                return Err("delegation candidate operation must bind current host source".into());
            }
            let checked = verify_chain(
                ControllerRoot {
                    subject: principal.subject,
                    controller: &root.controller_public_key,
                    admission_digest: state.admission_digest,
                },
                &state.durable.admission.snapshot,
                chain,
                now,
                &[],
            )?;
            let leaf = &chain.last().ok_or("empty delegation chain")?.delegation;
            if operation.actor_subject != leaf.grantee_subject
                || operation.actor_controller != &leaf.grantee_controller
            {
                return Err("delegation candidate leaf controller mismatch".into());
            }
            // The same requested operation must lie inside every ancestor's
            // scope; actor identity at each edge remains separately verified.
            for edge in chain {
                let d = &edge.delegation;
                let ancestor_operation = DelegatedOperation {
                    actor_subject: &d.grantee_subject,
                    actor_controller: &d.grantee_controller,
                    purpose: operation.purpose,
                    command_id: operation.command_id,
                    action: operation.action,
                    effects: operation.effects,
                    target: operation.target,
                    now_unix: now,
                };
                check_delegated_operation(d, &ancestor_operation)
                    .map_err(|e| format!("delegation candidate operation denied: {e:?}"))?;
            }
            Ok(ControllerDelegationInspection {
                project_admitted_root_subject: root.subject.clone(),
                root_capacity: root.kind,
                root_admission_ref: root.admission_ref.clone(),
                acting_agent_subject: leaf.grantee_subject.clone(),
                delegation_path: checked.path[..checked.count].to_vec(),
                target: view.current_document.clone(),
                authority_generation: view.generation,
                leaf_authentication_ref: None,
                leaf_authenticated_until_unix: None,
                root_controller: root.controller_public_key,
                chain_controllers: std::iter::once(root.controller_public_key)
                    .chain(chain.iter().map(|edge| edge.delegation.grantee_controller))
                    .collect(),
            })
        })
    }
}
