use super::{policy_digest_v3, BoundProposal, VerifiedEvidencePolicy};
use crate::{
    inference::inference_agent::governed::{ActorPolicyScope, PolicyError},
    modalities::{
        inference_guard::{evaluate_inference_guard, InferenceGuardAction},
        logic::WebizenVM,
    },
    NQuin,
};
use etalon_inference_contract::Decision;
/// Actual VM + graph evaluation, but NOT a current actor admission or one-use
/// publication grant. The live owner/policy gate must wrap this in integration.
pub fn evaluate_constraints(
    policy: &VerifiedEvidencePolicy<'_>,
    proposal: &BoundProposal<'_>,
    actor: ActorPolicyScope,
    now: u64,
) -> Result<Decision, PolicyError> {
    validate_context(policy, proposal, actor, now)?;
    proposal.recheck_payload()?;
    execute_programs(policy, proposal, actor, now)
}

pub(super) fn validate_context(
    policy: &VerifiedEvidencePolicy<'_>,
    proposal: &BoundProposal<'_>,
    actor: ActorPolicyScope,
    now: u64,
) -> Result<(), PolicyError> {
    let d = &policy.signed.document;
    d.base.valid_at(now)?;
    actor.validate()?;
    if now >= actor.valid_until || !d.base.rules.actors.as_slice().contains(&actor) {
        return Err(PolicyError::Scope);
    }
    let p = proposal.proposal;
    if p.identity.policy_id != policy_digest_v3(d)
        || p.identity.policy_generation != d.base.policy_generation
        || !d.base.rules.model.allows(p.identity.model_id)
        || !d.base.rules.request.allows(p.request_digest)
        || (!d.base.rules.graph_scopes.as_slice().is_empty()
            && !d
                .base
                .rules
                .graph_scopes
                .as_slice()
                .contains(&p.observation.context))
    {
        return Err(PolicyError::Scope);
    }
    // Presence and source requirements are evaluated BEFORE either VM program.
    d.observations.check(proposal)?;
    Ok(())
}

/// Canonical VM/graph implementation. Internal callers must validate immutable
/// retained bytes before obtaining the fresh time passed here; this performs no
/// payload hashing. The public helper preserves its original supplied-time API.
pub(super) fn execute_programs(
    policy: &VerifiedEvidencePolicy<'_>,
    proposal: &BoundProposal<'_>,
    actor: ActorPolicyScope,
    now: u64,
) -> Result<Decision, PolicyError> {
    let d = &policy.signed.document;
    let p = proposal.proposal;
    let rules = &d.base.rules;
    let mut vm = WebizenVM::new();
    for (dst, instruction) in vm.bytecode_buffer.iter_mut().zip(rules.program.as_slice()) {
        *dst = Some(instruction.lower()?);
    }
    let vm_allowed = vm.execute_constraint(&p.observation);
    let mut score_allowed = true;
    if !d.observations.score_program.as_slice().is_empty() {
        let score = p.numerical.ok_or(PolicyError::InvalidCandidate)?;
        let mut q = NQuin {
            subject: actor.subject,
            predicate: crate::q_hash("qualia:observed-numerical-score"),
            object: crate::frame_layout::pack_float_object(score.value),
            context: p.observation.context,
            metadata: score.meaning as u64,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        let mut vm = WebizenVM::new();
        for (dst, instruction) in vm
            .bytecode_buffer
            .iter_mut()
            .zip(d.observations.score_program.as_slice())
        {
            *dst = Some(instruction.lower()?);
        }
        score_allowed = vm.execute_constraint(&q);
    }
    let mut facts = [NQuin::default(); 256];
    let n = rules.facts.as_slice().len();
    facts[..n].copy_from_slice(rules.facts.as_slice());
    facts[n] = p.observation;
    let mut trace = [NQuin::default(); 256];
    let t = rules.trace.as_slice().len();
    trace[..t].copy_from_slice(rules.trace.as_slice());
    trace[t] = p.observation;
    let guard = evaluate_inference_guard(
        rules.norms.as_slice(),
        &facts[..n + 1],
        rules.epistemic.as_slice(),
        &trace[..t + 1],
        actor.subject,
        rules.required_claim,
        rules.temporal_invariant,
        now as u32,
        rules.non_derogable,
    );
    Ok(
        if !vm_allowed || !score_allowed || guard.action == InferenceGuardAction::Deny {
            Decision::Deny
        } else if guard.action == InferenceGuardAction::Steer {
            Decision::Steer {
                steering_digest: policy_digest_v3(d),
            }
        } else {
            Decision::Allow
        },
    )
}
