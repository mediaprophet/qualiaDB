//! Egress and materialisation admission. Every sink uses the same checks.

use super::flow::JobLabelContext;
use super::projection::{project_sink, DerivedSink};
use super::types::{Confidentiality, LabelFields};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;
use crate::{q_hash, NQuin};

/// Host callers that must not grow a private egress path.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EgressCaller {
    Vibe = 1,
    Poet = 2,
    BackgroundWorker = 3,
    DelegatedTool = 4,
}

/// Admit an egress sink. Recipient, compartment, purpose and environment are
/// all checked. A missing check is [`QdnfError::Denied`]. Digests only.
pub fn admit_egress(
    job: &JobLabelContext,
    recipient: StrongDigest,
    compartment: StrongDigest,
    purpose: StrongDigest,
    env_profile: u8,
) -> Result<(), QdnfError> {
    let fields = job.current();
    if fields.confidentiality.is_unknown() {
        return Err(QdnfError::Denied);
    }
    check_recipient(fields, recipient)?;
    check_compartment(fields, compartment)?;
    check_purpose(fields, purpose)?;
    check_environment(fields.confidentiality, env_profile)?;
    Ok(())
}

/// Vibe, Poet, background workers and delegated tools share [`admit_egress`].
pub fn admit_caller_egress(
    job: &JobLabelContext,
    _caller: EgressCaller,
    sink: DerivedSink,
    recipient: StrongDigest,
    compartment: StrongDigest,
    purpose: StrongDigest,
    env_profile: u8,
) -> Result<(), QdnfError> {
    let _projected = project_sink(job, sink)?;
    admit_egress(job, recipient, compartment, purpose, env_profile)
}

/// Storing a more-sensitive object under a weaker cache key is Denied.
pub fn admit_cache_insert(
    key_level: Confidentiality,
    stored: Confidentiality,
) -> Result<(), QdnfError> {
    let key_rank = key_level.lattice_rank().map_err(|_| QdnfError::Denied)?;
    let stored_rank = stored.lattice_rank().map_err(|_| QdnfError::Denied)?;
    if stored_rank > key_rank {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

/// Zero-heap egress decision descriptor for QualiaDB `NQuin` enumeration (`NET-21`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EgressAuditReceipt {
    pub caller: EgressCaller,
    pub sink: DerivedSink,
    pub confidentiality: Confidentiality,
    pub recipient: StrongDigest,
    pub compartment: StrongDigest,
    pub purpose: StrongDigest,
    pub env_profile: u8,
    pub allowed: bool,
}

fn fold_digest_60(d: &StrongDigest, seed: u64) -> u64 {
    let mut h = seed ^ 0xcbf2_9ce4_8422_2325u64;
    let mut i = 0usize;
    while i < d.0.len() {
        h ^= d.0[i] as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
        i += 1;
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

/// Enumerate an [`EgressAuditReceipt`] into a canonical 48-byte [`NQuin`] with 5-field XOR parity.
pub fn enumerate_egress_audit_quin(
    receipt: &EgressAuditReceipt,
    context_hash: u64,
    clock_epoch: u64,
) -> NQuin {
    let s0 = fold_digest_60(&receipt.recipient, receipt.caller as u64);
    let s1 = fold_digest_60(&receipt.compartment, s0 ^ ((receipt.sink as u64) << 8));
    let subject = fold_digest_60(&receipt.purpose, s1);
    let predicate = q_hash("urn:qualia:q42:network:egressDecision") & 0x0FFF_FFFF_FFFF_FFFF;
    let object = if receipt.allowed {
        q_hash("urn:qualia:q42:network:EgressAllow") & 0x0FFF_FFFF_FFFF_FFFF
    } else {
        q_hash("urn:qualia:q42:network:EgressDeny") & 0x0FFF_FFFF_FFFF_FFFF
    };
    let context = if context_hash != 0 {
        context_hash
    } else {
        q_hash("urn:qualia:q42:profile:network:1") & 0x0FFF_FFFF_FFFF_FFFF
    };
    let lamport = (clock_epoch & 0x1FFF_FFFF) << 32;
    let low = (receipt.caller as u64)
        | ((receipt.sink as u64) << 8)
        | ((receipt.env_profile as u64) << 16)
        | ((receipt.allowed as u64) << 24);
    let metadata = lamport | low;
    let parity = subject ^ predicate ^ object ^ context ^ metadata;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Evaluate [`admit_caller_egress`] and always emit a 5-field parity-verified [`NQuin`] audit record.
pub fn admit_caller_egress_with_quin(
    job: &JobLabelContext,
    caller: EgressCaller,
    sink: DerivedSink,
    recipient: StrongDigest,
    compartment: StrongDigest,
    purpose: StrongDigest,
    env_profile: u8,
    context_hash: u64,
    clock_epoch: u64,
) -> (Result<(), QdnfError>, NQuin) {
    let decision = admit_caller_egress(job, caller, sink, recipient, compartment, purpose, env_profile);
    let receipt = EgressAuditReceipt {
        caller,
        sink,
        confidentiality: job.current().confidentiality,
        recipient,
        compartment,
        purpose,
        env_profile,
        allowed: decision.is_ok(),
    };
    let quin = enumerate_egress_audit_quin(&receipt, context_hash, clock_epoch);
    (decision, quin)
}

/// Batch-enumerate egress decisions into caller-supplied [`NQuin`]s with 5-field XOR parity.
pub fn enumerate_egress_decisions_to_quins(
    receipts: &[EgressAuditReceipt],
    context_hash: u64,
    clock_epoch: u64,
    out: &mut [NQuin],
) -> usize {
    let n = receipts.len().min(out.len());
    let mut i = 0usize;
    while i < n {
        out[i] = enumerate_egress_audit_quin(
            &receipts[i],
            context_hash,
            clock_epoch.saturating_add(i as u64),
        );
        i += 1;
    }
    n
}

fn check_recipient(fields: &LabelFields, recipient: StrongDigest) -> Result<(), QdnfError> {
    if recipient.is_zero() {
        return Err(QdnfError::Denied);
    }
    if fields.confidentiality.requires_audience() && fields.audience.is_zero() {
        return Err(QdnfError::Denied);
    }
    if !fields.audience.is_zero() && recipient != fields.audience {
        return Err(QdnfError::Unauthorized);
    }
    Ok(())
}

fn check_compartment(fields: &LabelFields, compartment: StrongDigest) -> Result<(), QdnfError> {
    if fields.compartment_count == 0 {
        return Ok(());
    }
    if compartment.is_zero() {
        return Err(QdnfError::Denied);
    }
    if !contains(fields.compartments(), compartment) {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

fn check_purpose(fields: &LabelFields, purpose: StrongDigest) -> Result<(), QdnfError> {
    if fields.purpose_count == 0 {
        return Ok(());
    }
    if purpose.is_zero() {
        return Err(QdnfError::Denied);
    }
    if !contains(fields.purposes(), purpose) {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

fn check_environment(conf: Confidentiality, env_profile: u8) -> Result<(), QdnfError> {
    if env_profile == 0 {
        return Err(QdnfError::Denied);
    }
    let need = match conf {
        Confidentiality::C0Public | Confidentiality::C1Private => 1u8,
        Confidentiality::C2Sensitive => 2,
        Confidentiality::C3Compartmented => 3,
        Confidentiality::Unknown => return Err(QdnfError::Denied),
    };
    if env_profile < need {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

fn contains(items: &[StrongDigest], item: StrongDigest) -> bool {
    let mut i = 0usize;
    while i < items.len() {
        if items[i] == item {
            return true;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::policy_labels::types::LabelFields;

    fn digest(b: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = b;
        d.0[47] = 1;
        d
    }

    fn c2_job() -> JobLabelContext {
        let mut fields = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
        fields.audience = digest(9);
        fields.compartment_count = 1;
        fields.compartments[0] = digest(4);
        fields.purpose_count = 1;
        fields.purposes[0] = digest(5);
        JobLabelContext::new(fields).unwrap()
    }

    #[test]
    fn matching_egress_admits() {
        let job = c2_job();
        assert!(admit_egress(&job, digest(9), digest(4), digest(5), 2).is_ok());
    }

    #[test]
    fn missing_recipient_is_denied() {
        let job = c2_job();
        assert_eq!(
            admit_egress(&job, StrongDigest::ZERO, digest(4), digest(5), 2),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn missing_environment_is_denied() {
        let job = c2_job();
        assert_eq!(
            admit_egress(&job, digest(9), digest(4), digest(5), 0),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn weak_environment_for_c2_is_denied() {
        let job = c2_job();
        assert_eq!(
            admit_egress(&job, digest(9), digest(4), digest(5), 1),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn wrong_compartment_is_denied() {
        let job = c2_job();
        assert_eq!(
            admit_egress(&job, digest(9), digest(8), digest(5), 2),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn vibe_poet_worker_use_same_function() {
        let job = c2_job();
        for caller in [
            EgressCaller::Vibe,
            EgressCaller::Poet,
            EgressCaller::BackgroundWorker,
            EgressCaller::DelegatedTool,
        ] {
            assert!(admit_caller_egress(
                &job,
                caller,
                DerivedSink::Export,
                digest(9),
                digest(4),
                digest(5),
                2,
            )
            .is_ok());
        }
    }

    #[test]
    fn cache_c2_under_c0_key_is_denied() {
        assert_eq!(
            admit_cache_insert(Confidentiality::C0Public, Confidentiality::C2Sensitive),
            Err(QdnfError::Denied)
        );
        assert!(
            admit_cache_insert(Confidentiality::C2Sensitive, Confidentiality::C2Sensitive).is_ok()
        );
    }

    #[test]
    fn covert_metadata_sink_inherits() {
        let job = c2_job();
        for sink in [DerivedSink::Log, DerivedSink::Receipt, DerivedSink::Backup] {
            let projected = project_sink(&job, sink).unwrap();
            assert_eq!(projected.confidentiality, Confidentiality::C2Sensitive);
            assert!(admit_caller_egress(
                &job,
                EgressCaller::BackgroundWorker,
                sink,
                digest(9),
                digest(4),
                digest(5),
                2,
            )
            .is_ok());
        }
    }

    #[test]
    fn delegated_tool_egress_uses_admit_egress() {
        let job = c2_job();
        assert_eq!(
            admit_caller_egress(
                &job,
                EgressCaller::DelegatedTool,
                DerivedSink::QueryResult,
                StrongDigest::ZERO,
                digest(4),
                digest(5),
                2,
            ),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn egress_audit_quins_carry_five_field_parity_for_allow_and_deny() {
        let job = c2_job();
        let (allow_res, allow_q) = admit_caller_egress_with_quin(
            &job,
            EgressCaller::Vibe,
            DerivedSink::Export,
            digest(9),
            digest(4),
            digest(5),
            2,
            0,
            42,
        );
        assert!(allow_res.is_ok());
        assert_eq!(
            allow_q.parity,
            allow_q.subject ^ allow_q.predicate ^ allow_q.object ^ allow_q.context ^ allow_q.metadata
        );
        assert!(allow_q.verify_ecc_parity());

        let (deny_res, deny_q) = admit_caller_egress_with_quin(
            &job,
            EgressCaller::DelegatedTool,
            DerivedSink::QueryResult,
            StrongDigest::ZERO,
            digest(4),
            digest(5),
            2,
            0,
            43,
        );
        assert_eq!(deny_res, Err(QdnfError::Denied));
        assert!(deny_q.verify_ecc_parity());
        assert_ne!(allow_q.object, deny_q.object);
    }
}
