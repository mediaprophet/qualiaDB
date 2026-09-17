//! Egress and materialisation admission. Every sink uses the same checks.

use super::flow::JobLabelContext;
use super::projection::{project_sink, DerivedSink};
use super::types::{Confidentiality, LabelFields};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

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
}
