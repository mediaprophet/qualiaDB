//! Private nonconstructible disclosure decision. Not PolicyOutcome::Allow or a bool.
use super::codec::DecisionTicketFields;
use super::recognition::RecognizedController;
use super::PolicyError as E;
use crate::crypto::network::object_stream::RecipientRecord;
use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Independent admission facts. Check A against these, never against A itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndependentAdmission {
    pub descriptor: StrongDigest,
    pub context: StrongDigest,
    pub recipient: StrongDigest,
    pub publisher: StrongDigest,
    pub service: StrongDigest,
    pub purpose: StrongDigest,
    pub instrument: StrongDigest,
    pub scope: StrongDigest,
    pub operation: [u8; 16],
    pub closure: StrongDigest,
    pub policy: StrongDigest,
    pub consumer: StrongDigest,
    pub source: StrongDigest,
    pub source_length: u64,
}

/// Current remote decision. Fields are private; no public constructor exists.
#[derive(Debug, PartialEq, Eq)]
pub struct DisclosureDecision {
    action: super::DisclosureAction,
    policy: StrongDigest,
    controller_generation: Generation,
    decision_id: u64,
    expires: u64,
}

impl DisclosureDecision {
    pub const fn action(&self) -> super::DisclosureAction {
        self.action
    }
    pub const fn policy(&self) -> StrongDigest {
        self.policy
    }
    pub const fn controller_generation(&self) -> Generation {
        self.controller_generation
    }
    pub const fn decision_id(&self) -> u64 {
        self.decision_id
    }
    pub const fn expires(&self) -> u64 {
        self.expires
    }

    pub fn refuse_policy_outcome(outcome: PolicyOutcome) -> E {
        super::refuse_policy_outcome(outcome)
    }

    pub fn refuse_bool(allow: bool) -> E {
        super::refuse_bool_authority(allow)
    }

    /// R2 mints this after paired verification. R1 only binds schema/recognition.
    pub(super) fn bind(
        recognized: &RecognizedController,
        ticket: &DecisionTicketFields,
        now: u64,
    ) -> Result<Self, E> {
        recognized.current_at(now)?;
        ticket.valid_at(now)?;
        if ticket.policy != recognized.policy()
            || ticket.controller_generation != recognized.controller_generation()
            || ticket.decision_id < recognized.decision_id_floor()
            || !recognized.allowed_ops().contains(ticket.action)
            || ticket.lease_secs()? > recognized.max_decision_lifetime()
            || ticket.expires > recognized.expires_unix()
        {
            return Err(E::Reference);
        }
        Ok(Self {
            action: ticket.action,
            policy: ticket.policy,
            controller_generation: ticket.controller_generation,
            decision_id: ticket.decision_id,
            expires: ticket.expires,
        })
    }
}

pub fn check_recipient_against_independent(
    record: &RecipientRecord<'_>,
    expected: &IndependentAdmission,
) -> Result<(), E> {
    let a = record.fields();
    if a.descriptor != expected.descriptor
        || a.context != expected.context
        || a.recipient != expected.recipient
        || a.publisher != expected.publisher
        || a.service != expected.service
        || a.purpose != expected.purpose
        || a.instrument != expected.instrument
        || a.scope != expected.scope
        || a.operation != expected.operation
        || a.closure != expected.closure
        || a.policy != expected.policy
        || a.consumer != expected.consumer
        || a.source != expected.source
        || a.source_length != expected.source_length
    {
        return Err(E::Reference);
    }
    Ok(())
}
