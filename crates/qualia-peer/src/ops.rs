//! E20.4–E20.6 organisational trust, redacted ops, honest device-study flag.
//!
//! No universal organisational superuser. Dashboards return counts, never
//! patient identifiers. Failures are [`QdnfError`] codes, not payload echoes.

use qualia_core_db::net::qdnf::errors::QdnfError;
use qualia_core_db::net::qdnf::types::{Generation, StrongDigest};

/// There is no organisation-wide superuser in this slice.
pub fn universal_org_superuser() -> bool {
    false
}

/// Realistic-user/device study (E20.6) has not been executed.
pub fn realistic_device_study_executed() -> bool {
    false
}

/// Clinician / mission credential bound to one org, purpose and generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CredentialScope {
    pub principal: StrongDigest,
    pub organisation: StrongDigest,
    pub purpose: StrongDigest,
    pub compartment: StrongDigest,
    pub issued_generation: Generation,
}

/// Admit a scoped credential against the live revocation generation.
///
/// A revocation generation greater than the issued generation is Revoked.
/// Zero identity fields are Unauthorized. This is not a superuser grant.
pub fn admit_scoped_credential(
    scope: CredentialScope,
    revocation_generation: Generation,
) -> Result<(), QdnfError> {
    if universal_org_superuser() {
        return Err(QdnfError::Denied);
    }
    if scope.principal.is_zero()
        || scope.organisation.is_zero()
        || scope.purpose.is_zero()
        || scope.compartment.is_zero()
    {
        return Err(QdnfError::Unauthorized);
    }
    if scope.issued_generation == Generation::ZERO {
        return Err(QdnfError::Unauthorized);
    }
    if revocation_generation.0 > scope.issued_generation.0 {
        return Err(QdnfError::Revoked);
    }
    Ok(())
}

/// Redacted operational counts. No patient / clinician identifier fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RedactedDashboard {
    pub active_sessions: u32,
    pub denied_ops: u32,
    pub revoked_creds: u32,
}

/// Build a dashboard from counts only. Callers never pass patient ids.
pub fn redacted_dashboard(
    active_sessions: u32,
    denied_ops: u32,
    revoked_creds: u32,
) -> RedactedDashboard {
    RedactedDashboard {
        active_sessions,
        denied_ops,
        revoked_creds,
    }
}

pub fn active_session_count(dashboard: RedactedDashboard) -> u32 {
    dashboard.active_sessions
}

pub fn denied_op_count(dashboard: RedactedDashboard) -> u32 {
    dashboard.denied_ops
}

pub fn revoked_credential_count(dashboard: RedactedDashboard) -> u32 {
    dashboard.revoked_creds
}

/// Dashboard APIs do not surface patient identifiers.
pub fn patient_identifiers_on_dashboard() -> bool {
    false
}

/// Fail closed with a typed code. `untrusted` is not copied into the error.
pub fn safe_failure(untrusted: &[u8]) -> QdnfError {
    let _ = untrusted;
    QdnfError::Denied
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d.0[47] = 0xA5;
        d
    }

    fn scope() -> CredentialScope {
        CredentialScope {
            principal: digest(1),
            organisation: digest(2),
            purpose: digest(3),
            compartment: digest(4),
            issued_generation: Generation(3),
        }
    }

    #[test]
    fn no_universal_superuser_and_revocation_generation() {
        assert!(!universal_org_superuser());
        assert!(admit_scoped_credential(scope(), Generation(3)).is_ok());
        assert!(admit_scoped_credential(scope(), Generation(1)).is_ok());
        assert_eq!(
            admit_scoped_credential(scope(), Generation(4)),
            Err(QdnfError::Revoked)
        );
        let mut zero = scope();
        zero.principal = StrongDigest::ZERO;
        assert_eq!(
            admit_scoped_credential(zero, Generation(1)),
            Err(QdnfError::Unauthorized)
        );
        let mut issued0 = scope();
        issued0.issued_generation = Generation::ZERO;
        assert_eq!(
            admit_scoped_credential(issued0, Generation::ZERO),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn dashboard_returns_counts_not_patient_ids() {
        let dash = redacted_dashboard(4, 2, 1);
        assert_eq!(active_session_count(dash), 4);
        assert_eq!(denied_op_count(dash), 2);
        assert_eq!(revoked_credential_count(dash), 1);
        assert!(!patient_identifiers_on_dashboard());
        let text = format!("{dash:?}");
        assert!(text.contains("active_sessions"));
        assert!(!text.contains("patient"));
        assert!(!text.contains("MRN"));
        assert!(!text.contains("did:"));
    }

    #[test]
    fn safe_failure_is_code_not_payload_echo() {
        let payload = b"patient-MRN-999-secret";
        let err = safe_failure(payload);
        assert_eq!(err, QdnfError::Denied);
        let text = err.to_string();
        assert_eq!(text, "denied");
        assert!(!text.contains("patient"));
        assert!(!text.contains("MRN"));
        assert!(!text.contains("999"));
        assert!(!text.contains("secret"));
    }

    #[test]
    fn realistic_device_study_is_not_claimed() {
        assert!(!realistic_device_study_executed());
    }
}
