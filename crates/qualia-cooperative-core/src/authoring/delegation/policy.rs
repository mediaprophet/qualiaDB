use super::*;
use crate::{
    agency_delegation::{delegation_permits, AccessRequest, ConsentState},
    agency_domain::{agency_domain_taxonomy, ids::AI_PROXY},
    authority_type::ids,
    taxonomy::Sphere,
    trigger::TriggerContext,
};
use qualia_app_contract::{delegation::DelegationMode, effects::CommandEffect};
use std::io::Write;

fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4096
}

struct ByteCount(usize);
impl Write for ByteCount {
    fn write(&mut self, value: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(value.len())
            .filter(|n| *n <= MAX_AUTHORING_DELEGATION_BYTES)
            .ok_or_else(|| std::io::Error::other("delegation byte budget exceeded"))?;
        Ok(value.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Structural/policy validation only. The native owner must verify every
/// signature, qualified association and root adoption separately.
pub fn validate_delegation(d: &AuthoringDelegation, now: u64) -> Result<(), DelegationError> {
    let a = &d.agency;
    let t = &d.task;
    if d.actions.is_empty()
        || d.actions.len() > 10
        || a.agent_dids.len() != 1
        || a.scope.len() > 12
        || t.allowed_command_ids.is_empty()
        || t.allowed_command_ids.len() > 32
        || t.allowed_effects.is_empty()
        || t.allowed_effects.len() > 8
        || d.remaining_depth as usize >= MAX_AUTHORING_DELEGATION_DEPTH
        || a.trigger.is_some()
        || !a.transfer_schedule.is_empty()
    {
        return Err(DelegationError::Bounds);
    }
    for value in [
        &d.root_subject,
        &d.project_id,
        &d.document_id,
        &d.source_id,
        &d.grantor_subject,
        &d.grantee_subject,
        &a.id,
        &a.principal_did,
        &a.agent_dids[0],
        &a.values_anchor,
        &t.delegation_id,
        &t.principal_id,
        &t.actor_id,
        &t.goal,
    ] {
        if !text(value) {
            return Err(DelegationError::Binding);
        }
    }
    if d.version != 1
        || d.root_adoption_digest == [0; 32]
        || d.grantor_subject == d.grantee_subject
        || d.grantor_controller == [0; 32]
        || d.grantee_controller == [0; 32]
        || t.principal_id != d.root_subject
        || t.actor_id != d.grantee_subject
        || t.app_id.as_str() != "poet"
        || t.delegation_id != a.id
        || a.evidence_ref.as_deref().is_none_or(|s| !text(s))
        || d.actions
            .iter()
            .enumerate()
            .any(|(i, v)| d.actions[..i].contains(v))
        || t.allowed_effects
            .iter()
            .enumerate()
            .any(|(i, v)| t.allowed_effects[..i].contains(v))
        || t.allowed_command_ids
            .iter()
            .enumerate()
            .any(|(i, v)| !text(v) || t.allowed_command_ids[..i].contains(v))
    {
        return Err(DelegationError::Binding);
    }
    // The first native adapter supports explicit automated agency only. Other
    // profiles/triggers/jurisdictions remain retained data, never silently ignored.
    if a.domain != AI_PROXY
        || a.jurisdiction.is_some()
        || a.authority.modality.as_deref() != Some(ids::MOD_AUTOMATED)
        || a.authority.trigger.as_deref() != Some(ids::TRIG_PERSISTENT)
        || a.authority.accountability.as_deref() != Some(ids::ACCT_VALUES_BOUND)
        || !a.authority.flags.is_empty()
    {
        return Err(DelegationError::UnsupportedPolicy);
    }
    if a.consent != ConsentState::Granted {
        return Err(DelegationError::Consent);
    }
    let end = a.valid_to_unix.ok_or(DelegationError::Binding)? as u64;
    if a.revoked
        || a.valid_from_unix as u64 >= end
        || now < a.valid_from_unix as u64
        || now >= end
        || !t.is_active()
        || now
            .checked_mul(1000)
            .is_none_or(|ms| ms >= t.expiry_timestamp_ms.get())
        || t.expiry_timestamp_ms.get() > end * 1000
    {
        return Err(DelegationError::ExpiredOrRevoked);
    }
    if t.budgets.max_steps == 0
        || t.budgets.max_cpu_time_ms == 0
        || t.budgets.max_memory_bytes == 0
        || t.budgets.external_cost_minor_units.is_some_and(|n| n != 0)
        || t.allowed_effects.contains(&CommandEffect::ExternalCharge)
    {
        return Err(DelegationError::Budget);
    }
    // No inferred network/provider/device authority in this bounded adapter.
    if t.allowed_effects.iter().any(|e| {
        matches!(
            e,
            CommandEffect::NetworkDisclosure | CommandEffect::DeviceCapture
        )
    }) {
        return Err(DelegationError::UnsupportedPolicy);
    }
    if a.scope.iter().any(|(k, v)| {
        !text(k) || !text(v) || !matches!(k.as_str(), "purpose" | "action" | "data_class")
    }) || a
        .scope
        .iter()
        .enumerate()
        .any(|(i, v)| a.scope[..i].contains(v))
        || !a.scope.iter().any(|(k, v)| k == "purpose" && v == &t.goal)
        || !a
            .scope
            .iter()
            .any(|(k, v)| k == "data_class" && v == AUTHORING_DATA_CLASS)
        || d.actions.iter().any(|action| {
            !a.scope
                .iter()
                .any(|(k, v)| k == "action" && v == action_name(*action))
        })
    {
        return Err(DelegationError::Scope);
    }
    serde_json::to_writer(&mut ByteCount(0), d).map_err(|_| DelegationError::Bounds)?;
    Ok(())
}

pub fn action_name(action: super::super::AuthoringAction) -> &'static str {
    use super::super::AuthoringAction::*;
    match action {
        ReadSource => "read_source",
        ExecuteSource => "execute_source",
        MaterialEdit => "material_edit",
        Undo => "undo",
        Redo => "redo",
        Rebuild => "rebuild",
        SaveCheckpoint => "save_checkpoint",
        ReopenCheckpoint => "reopen_checkpoint",
        ContributionRecord => "contribution_record",
        ReviewDecision => "review_decision",
    }
}

/// Check the operation against one already admitted edge. This returns no
/// reusable permit and consumes no budget; the locked host must do both.
pub fn check_delegated_operation(
    d: &AuthoringDelegation,
    op: &DelegatedOperation<'_>,
) -> Result<(), DelegationError> {
    validate_delegation(d, op.now_unix)?;
    if op.actor_subject != d.grantee_subject
        || op.actor_controller != &d.grantee_controller
        || op.target.project_id != d.project_id
        || op.target.document_id != d.document_id
        || op.target.source_id != d.source_id
        || op.purpose != d.task.goal
        || !d.actions.contains(&op.action)
        || !d
            .task
            .allowed_command_ids
            .iter()
            .any(|v| v == op.command_id)
        || op.effects.is_empty()
        || op
            .effects
            .iter()
            .any(|effect| !d.task.is_effect_allowed(effect))
    {
        return Err(DelegationError::Scope);
    }
    use super::super::AuthoringAction::*;
    let required_effect = match op.action {
        ReadSource => CommandEffect::Read,
        SaveCheckpoint => CommandEffect::FilesystemWrite,
        ContributionRecord | ReviewDecision => CommandEffect::WorkspaceMutation,
        _ => CommandEffect::DocumentMutation,
    };
    if !op.effects.contains(&required_effect) {
        return Err(DelegationError::Scope);
    }
    match d.task.mode {
        DelegationMode::Observe
            if op.action == super::super::AuthoringAction::ReadSource
                && op.effects == [CommandEffect::Read] => {}
        DelegationMode::BoundedDelegation => {}
        _ => return Err(DelegationError::UnsupportedPolicy),
    }
    let req = AccessRequest {
        domain: d.agency.domain.clone(),
        data_class: AUTHORING_DATA_CLASS.into(),
        action: match op.action {
            super::super::AuthoringAction::ReadSource => "read",
            super::super::AuthoringAction::ReviewDecision => "decide",
            _ => "write",
        }
        .into(),
        sphere: Sphere::Personhood,
        jurisdiction: None,
        provenance: None,
    };
    // Native action/purpose/data constraints are checked above; reuse the
    // existing agency evaluator for its consent/domain/selfhood semantics.
    let now = u32::try_from(op.now_unix).map_err(|_| DelegationError::ExpiredOrRevoked)?;
    if !delegation_permits(
        &d.agency,
        &agency_domain_taxonomy(),
        &req,
        &TriggerContext::at(now),
    )
    .is_permit()
    {
        return Err(DelegationError::Scope);
    }
    Ok(())
}

/// Parent hash/signature matching is performed by the native verifier. This
/// compares all policy-bearing fields without inferring wildcard authority.
pub fn check_parent_attenuation(
    p: &AuthoringDelegation,
    c: &AuthoringDelegation,
) -> Result<(), DelegationError> {
    let pb = &p.task.budgets;
    let cb = &c.task.budgets;
    if p.remaining_depth == 0
        || c.remaining_depth >= p.remaining_depth
        || c.root_subject != p.root_subject
        || c.root_adoption_digest != p.root_adoption_digest
        || c.project_id != p.project_id
        || c.document_id != p.document_id
        || c.source_id != p.source_id
        || c.grantor_subject != p.grantee_subject
        || c.grantor_controller != p.grantee_controller
        || c.agency.principal_did != p.agency.principal_did
        || c.agency.domain != p.agency.domain
        || c.agency.authority != p.agency.authority
        || c.agency.values_anchor != p.agency.values_anchor
        || c.agency.jurisdiction != p.agency.jurisdiction
        || c.agency.precedence != p.agency.precedence
        || c.agency.valid_from_unix < p.agency.valid_from_unix
        || c.agency.valid_to_unix > p.agency.valid_to_unix
        || c.task.expiry_timestamp_ms > p.task.expiry_timestamp_ms
        || c.task.goal != p.task.goal
        || c.task.app_id != p.task.app_id
        || c.task.mode != p.task.mode
        || c.actions.iter().any(|v| !p.actions.contains(v))
        || c.agency.scope.iter().any(|v| !p.agency.scope.contains(v))
        || c.task
            .allowed_effects
            .iter()
            .any(|v| !p.task.allowed_effects.contains(v))
        || c.task
            .allowed_command_ids
            .iter()
            .any(|v| !p.task.allowed_command_ids.contains(v))
        || cb.max_steps > pb.max_steps
        || cb.max_cpu_time_ms > pb.max_cpu_time_ms
        || cb.max_memory_bytes > pb.max_memory_bytes
        || cb.external_cost_minor_units != pb.external_cost_minor_units
    {
        return Err(DelegationError::Attenuation);
    }
    Ok(())
}
