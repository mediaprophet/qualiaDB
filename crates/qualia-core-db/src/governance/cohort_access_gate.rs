//! Permission-Scoped Query, Explanation & Derivative Access Gate (`C4–C5`).
//!
//! Enforces sensitivity tiers in `context[56..63]` (`PUBLIC=0`, `RESTRICTED=1`, `CLASSIFIED=2`)
//! and human principal / delegated-agent authority (`AUTH-H1..H6`) across source queries,
//! cohort estimates, inference explanations, error logs, and HMC export bundles.
//!
//! Invariant (`C4`): Aggregation or estimation never auto-declassifies restricted inputs.

use core::fmt;
use crate::NQuin;

pub const SENSITIVITY_SHIFT: u32 = 56;
pub const SENSITIVITY_PUBLIC: u8 = 0;
pub const SENSITIVITY_RESTRICTED: u8 = 1;
pub const SENSITIVITY_CLASSIFIED: u8 = 2;
pub const CONTEXT_GRAPH_MASK: u64 = 0x00FF_FFFF_FFFF_FFFF;
pub const SENSITIVITY_MASK: u64 = 0xFFu64 << SENSITIVITY_SHIFT;

/// Scoped grant binding a human principal and an optional delegated agent.
///
/// Direct human action sets `agent_did_hash = 0`. Delegated action sets `agent_did_hash != 0`.
/// Missing human principal (`human_principal_did_hash = 0`) is always denied (`AUTH-H1..H6`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrincipalAndAgentGrant {
    pub human_principal_did_hash: u64,
    pub agent_did_hash: u64,
    pub authorized_context_hash: u64,
    pub max_sensitivity_class: u8,
    pub allow_derivative_export: bool,
    pub allow_explanation_trace: bool,
    pub issued_epoch: u64,
    pub expiry_epoch: u64,
    pub revoked: bool,
}

impl PrincipalAndAgentGrant {
    #[inline]
    pub const fn direct_human(human: u64, ctx: u64, sens: u8, issued: u64, expiry: u64) -> Self {
        Self {
            human_principal_did_hash: human,
            agent_did_hash: 0,
            authorized_context_hash: ctx,
            max_sensitivity_class: sens,
            allow_derivative_export: true,
            allow_explanation_trace: true,
            issued_epoch: issued,
            expiry_epoch: expiry,
            revoked: false,
        }
    }

    #[inline]
    pub const fn delegated_agent(
        human: u64, agent: u64, ctx: u64, sens: u8, deriv: bool, trace: bool, issued: u64, expiry: u64,
    ) -> Self {
        Self {
            human_principal_did_hash: human,
            agent_did_hash: agent,
            authorized_context_hash: ctx,
            max_sensitivity_class: sens,
            allow_derivative_export: deriv,
            allow_explanation_trace: trace,
            issued_epoch: issued,
            expiry_epoch: expiry,
            revoked: false,
        }
    }

    #[inline(always)]
    pub const fn is_direct_human(&self) -> bool { self.agent_did_hash == 0 }
    #[inline(always)]
    pub const fn is_delegated_agent(&self) -> bool { self.agent_did_hash != 0 }
}

/// Zero-heap receipt returned after filtering a candidate cohort NQuin slice.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CohortAccessReceipt {
    pub admitted_count: usize,
    pub redacted_count: usize,
    pub inherited_max_sensitivity: u8,
}

impl CohortAccessReceipt {
    #[inline(always)]
    pub const fn is_all_admitted(&self) -> bool { self.redacted_count == 0 }
    #[inline(always)]
    pub const fn total_examined(&self) -> usize { self.admitted_count + self.redacted_count }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessGateError {
    MissingHumanPrincipal,
    GrantRevoked,
    GrantExpired { current_epoch: u64, expiry_epoch: u64 },
    GrantNotYetValid { current_epoch: u64, issued_epoch: u64 },
    InvalidEpochRange { issued_epoch: u64, expiry_epoch: u64 },
    SensitivityExceeded { quin_sensitivity: u8, max_sensitivity: u8 },
    ContextUnauthorized { requested_context: u64, authorized_context: u64 },
    DeclassificationForbidden { required_sensitivity: u8, target_sensitivity: u8 },
    ExplanationTraceForbidden,
    DerivativeExportForbidden,
    OutputBufferTooSmall { required: usize, available: usize },
}

impl fmt::Display for AccessGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHumanPrincipal => write!(f, "Missing or unauthenticated human principal (DID hash 0)"),
            Self::GrantRevoked => write!(f, "Principal or agent grant has been revoked"),
            Self::GrantExpired { current_epoch, expiry_epoch } => write!(f, "Grant expired at {} (current {})", expiry_epoch, current_epoch),
            Self::GrantNotYetValid { current_epoch, issued_epoch } => write!(f, "Grant not valid until {} (current {})", issued_epoch, current_epoch),
            Self::InvalidEpochRange { issued_epoch, expiry_epoch } => write!(f, "issued_epoch {} > expiry_epoch {}", issued_epoch, expiry_epoch),
            Self::SensitivityExceeded { quin_sensitivity, max_sensitivity } => write!(f, "Quin sensitivity {} > max {}", quin_sensitivity, max_sensitivity),
            Self::ContextUnauthorized { requested_context, authorized_context } => write!(f, "Context {:014x} unauthorized (authorized {:014x})", requested_context, authorized_context),
            Self::DeclassificationForbidden { required_sensitivity, target_sensitivity } => write!(f, "Declassification forbidden: input sensitivity {} > target {}", required_sensitivity, target_sensitivity),
            Self::ExplanationTraceForbidden => write!(f, "Explanation trace forbidden by grant"),
            Self::DerivativeExportForbidden => write!(f, "Derivative export forbidden by grant"),
            Self::OutputBufferTooSmall { required, available } => write!(f, "Output buffer too small: required {}, available {}", required, available),
        }
    }
}

impl std::error::Error for AccessGateError {}

#[inline(always)]
pub const fn extract_sensitivity_class(context: u64) -> u8 { ((context >> SENSITIVITY_SHIFT) & 0xFF) as u8 }
#[inline(always)]
pub const fn extract_graph_context_hash(context: u64) -> u64 { context & CONTEXT_GRAPH_MASK }
#[inline(always)]
pub const fn pack_context_with_sensitivity(graph_hash_56bit: u64, sensitivity: u8) -> u64 {
    (graph_hash_56bit & CONTEXT_GRAPH_MASK) | ((sensitivity as u64) << SENSITIVITY_SHIFT)
}

/// Validates that the grant has an authentic human principal, is within its validity epoch,
/// and has not been revoked.
pub fn verify_grant_active(grant: &PrincipalAndAgentGrant, current_epoch: u64) -> Result<(), AccessGateError> {
    if grant.human_principal_did_hash == 0 { return Err(AccessGateError::MissingHumanPrincipal); }
    if grant.revoked { return Err(AccessGateError::GrantRevoked); }
    if grant.issued_epoch > grant.expiry_epoch {
        return Err(AccessGateError::InvalidEpochRange { issued_epoch: grant.issued_epoch, expiry_epoch: grant.expiry_epoch });
    }
    if current_epoch < grant.issued_epoch {
        return Err(AccessGateError::GrantNotYetValid { current_epoch, issued_epoch: grant.issued_epoch });
    }
    if current_epoch > grant.expiry_epoch {
        return Err(AccessGateError::GrantExpired { current_epoch, expiry_epoch: grant.expiry_epoch });
    }
    Ok(())
}

/// Zero-heap filter for candidate cohort Quins before query evaluation or statistical estimation.
pub fn filter_authorized_cohort_quins(
    quins: &[NQuin],
    grant: &PrincipalAndAgentGrant,
    current_epoch: u64,
    out: &mut [NQuin],
) -> Result<CohortAccessReceipt, AccessGateError> {
    verify_grant_active(grant, current_epoch)?;

    let mut admitted_count = 0;
    let mut redacted_count = 0;
    let mut inherited_max_sensitivity = SENSITIVITY_PUBLIC;

    for q in quins {
        let quin_sens = extract_sensitivity_class(q.context);
        let quin_graph = extract_graph_context_hash(q.context);

        let context_ok = grant.authorized_context_hash == 0
            || quin_graph == grant.authorized_context_hash
            || (quin_sens == SENSITIVITY_PUBLIC && quin_graph == 0);

        let sensitivity_ok = quin_sens <= grant.max_sensitivity_class;

        if context_ok && sensitivity_ok {
            if admitted_count >= out.len() {
                return Err(AccessGateError::OutputBufferTooSmall {
                    required: admitted_count + 1,
                    available: out.len(),
                });
            }
            out[admitted_count] = *q;
            admitted_count += 1;
            if quin_sens > inherited_max_sensitivity {
                inherited_max_sensitivity = quin_sens;
            }
        } else {
            redacted_count += 1;
        }
    }

    Ok(CohortAccessReceipt {
        admitted_count,
        redacted_count,
        inherited_max_sensitivity,
    })
}

/// Enforces C4: Aggregation or estimation never auto-declassifies restricted inputs.
///
/// If any source Quin has sensitivity `> target_export_sensitivity`, fails closed with
/// `DeclassificationForbidden`. Requires derivative or explanation export permissions.
pub fn authorize_derivative_or_explanation_export(
    source_quins: &[NQuin],
    grant: &PrincipalAndAgentGrant,
    target_export_sensitivity: u8,
    is_explanation_trace: bool,
    current_epoch: u64,
) -> Result<u64, AccessGateError> {
    verify_grant_active(grant, current_epoch)?;

    if is_explanation_trace {
        if !grant.allow_explanation_trace {
            return Err(AccessGateError::ExplanationTraceForbidden);
        }
    } else if !grant.allow_derivative_export {
        return Err(AccessGateError::DerivativeExportForbidden);
    }

    let mut required_sensitivity = SENSITIVITY_PUBLIC;
    for q in source_quins {
        let quin_sens = extract_sensitivity_class(q.context);
        let quin_graph = extract_graph_context_hash(q.context);

        if grant.authorized_context_hash != 0
            && quin_graph != grant.authorized_context_hash
            && !(quin_sens == SENSITIVITY_PUBLIC && quin_graph == 0)
        {
            return Err(AccessGateError::ContextUnauthorized {
                requested_context: quin_graph,
                authorized_context: grant.authorized_context_hash,
            });
        }

        if quin_sens > required_sensitivity {
            required_sensitivity = quin_sens;
        }
    }

    if required_sensitivity > grant.max_sensitivity_class {
        return Err(AccessGateError::SensitivityExceeded {
            quin_sensitivity: required_sensitivity,
            max_sensitivity: grant.max_sensitivity_class,
        });
    }

    if required_sensitivity > target_export_sensitivity {
        return Err(AccessGateError::DeclassificationForbidden {
            required_sensitivity,
            target_sensitivity: target_export_sensitivity,
        });
    }

    let mut auth_token = grant.human_principal_did_hash
        ^ grant.agent_did_hash.rotate_left(17)
        ^ ((target_export_sensitivity as u64) << SENSITIVITY_SHIFT)
        ^ grant.authorized_context_hash.rotate_left(7)
        ^ current_epoch;

    for q in source_quins {
        auth_token = auth_token.wrapping_mul(0x1000_0000_1B3) ^ q.parity ^ q.context;
    }

    let salt = if is_explanation_trace { 0x5452_4143_4530_3031 } else { 0x4445_5249_5630_3031 };
    Ok(auth_token ^ salt)
}

/// Point check for single-Quin read access under an active grant.
pub fn validate_quin_access(
    quin: &NQuin,
    grant: &PrincipalAndAgentGrant,
    current_epoch: u64,
) -> Result<u8, AccessGateError> {
    verify_grant_active(grant, current_epoch)?;
    let quin_sens = extract_sensitivity_class(quin.context);
    let quin_graph = extract_graph_context_hash(quin.context);

    let context_ok = grant.authorized_context_hash == 0
        || quin_graph == grant.authorized_context_hash
        || (quin_sens == SENSITIVITY_PUBLIC && quin_graph == 0);

    if !context_ok {
        return Err(AccessGateError::ContextUnauthorized {
            requested_context: quin_graph,
            authorized_context: grant.authorized_context_hash,
        });
    }
    if quin_sens > grant.max_sensitivity_class {
        return Err(AccessGateError::SensitivityExceeded {
            quin_sensitivity: quin_sens,
            max_sensitivity: grant.max_sensitivity_class,
        });
    }
    Ok(quin_sens)
}

// ── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const HUMAN_DID: u64 = 0x0011_2233_4455_6677;
    const AGENT_DID: u64 = 0x0088_99AA_BBCC_DDEE;
    const CTX_CORPUS: u64 = 0x0000_AABB_CCDD_EEFF;
    const CTX_OTHER: u64 = 0x0000_1111_2222_3333;

    fn test_quin(subject: u64, context_hash: u64, sensitivity: u8) -> NQuin {
        let mut q = NQuin {
            subject,
            predicate: 0x1000,
            object: 0x2000,
            context: pack_context_with_sensitivity(context_hash, sensitivity),
            metadata: 0,
            parity: 0,
        };
        q.recalculate_parity();
        q
    }

    #[test]
    fn test_context_sensitivity_packing_roundtrip() {
        let p1 = pack_context_with_sensitivity(CTX_CORPUS, SENSITIVITY_RESTRICTED);
        assert_eq!((extract_sensitivity_class(p1), extract_graph_context_hash(p1)), (SENSITIVITY_RESTRICTED, CTX_CORPUS));

        let p2 = pack_context_with_sensitivity(CTX_CORPUS, SENSITIVITY_CLASSIFIED);
        assert_eq!((extract_sensitivity_class(p2), extract_graph_context_hash(p2)), (SENSITIVITY_CLASSIFIED, CTX_CORPUS));
    }

    #[test]
    fn test_valid_human_and_delegated_agent_read_authorized_cohort() {
        let quins = [
            test_quin(1, CTX_CORPUS, SENSITIVITY_PUBLIC),
            test_quin(2, CTX_CORPUS, SENSITIVITY_RESTRICTED),
            test_quin(3, 0, SENSITIVITY_PUBLIC),
        ];
        let mut out = [NQuin::default(); 4];

        let human = PrincipalAndAgentGrant::direct_human(HUMAN_DID, CTX_CORPUS, SENSITIVITY_RESTRICTED, 100, 200);
        let rec = filter_authorized_cohort_quins(&quins, &human, 150, &mut out).unwrap();
        assert_eq!((rec.admitted_count, rec.redacted_count, rec.inherited_max_sensitivity), (3, 0, SENSITIVITY_RESTRICTED));

        let agent = PrincipalAndAgentGrant::delegated_agent(HUMAN_DID, AGENT_DID, CTX_CORPUS, SENSITIVITY_RESTRICTED, true, true, 100, 200);
        let rec_ag = filter_authorized_cohort_quins(&quins, &agent, 150, &mut out).unwrap();
        assert_eq!(rec_ag.admitted_count, 3);
    }

    #[test]
    fn test_negative_declassification_forbidden_on_restricted_or_classified_cohort() {
        let cohort_r = [test_quin(1, CTX_CORPUS, SENSITIVITY_PUBLIC), test_quin(2, CTX_CORPUS, SENSITIVITY_RESTRICTED)];
        let grant = PrincipalAndAgentGrant::direct_human(HUMAN_DID, CTX_CORPUS, SENSITIVITY_CLASSIFIED, 100, 200);

        let err = authorize_derivative_or_explanation_export(&cohort_r, &grant, SENSITIVITY_PUBLIC, false, 150).unwrap_err();
        assert_eq!(err, AccessGateError::DeclassificationForbidden { required_sensitivity: SENSITIVITY_RESTRICTED, target_sensitivity: SENSITIVITY_PUBLIC });

        let cohort_c = [test_quin(1, CTX_CORPUS, SENSITIVITY_CLASSIFIED)];
        let err2 = authorize_derivative_or_explanation_export(&cohort_c, &grant, SENSITIVITY_RESTRICTED, false, 150).unwrap_err();
        assert_eq!(err2, AccessGateError::DeclassificationForbidden { required_sensitivity: SENSITIVITY_CLASSIFIED, target_sensitivity: SENSITIVITY_RESTRICTED });
    }

    #[test]
    fn test_negative_missing_principal_expired_or_revoked_fails_closed() {
        let quins = [test_quin(1, CTX_CORPUS, SENSITIVITY_PUBLIC)];
        let mut out = [NQuin::default(); 2];

        let invalid_p = PrincipalAndAgentGrant::delegated_agent(0, AGENT_DID, CTX_CORPUS, SENSITIVITY_PUBLIC, true, true, 100, 200);
        assert_eq!(filter_authorized_cohort_quins(&quins, &invalid_p, 150, &mut out).unwrap_err(), AccessGateError::MissingHumanPrincipal);

        let exp = PrincipalAndAgentGrant::direct_human(HUMAN_DID, CTX_CORPUS, SENSITIVITY_PUBLIC, 100, 200);
        assert_eq!(filter_authorized_cohort_quins(&quins, &exp, 201, &mut out).unwrap_err(), AccessGateError::GrantExpired { current_epoch: 201, expiry_epoch: 200 });

        let mut rev = exp;
        rev.revoked = true;
        assert_eq!(filter_authorized_cohort_quins(&quins, &rev, 150, &mut out).unwrap_err(), AccessGateError::GrantRevoked);

        let mut bad_epoch = exp;
        bad_epoch.issued_epoch = 500;
        bad_epoch.expiry_epoch = 200;
        assert_eq!(verify_grant_active(&bad_epoch, 150).unwrap_err(), AccessGateError::InvalidEpochRange { issued_epoch: 500, expiry_epoch: 200 });
    }

    #[test]
    fn test_negative_unauthorized_explanation_trace_fails_closed() {
        let quins = [test_quin(1, CTX_CORPUS, SENSITIVITY_PUBLIC)];
        let grant_no_trace = PrincipalAndAgentGrant::delegated_agent(HUMAN_DID, AGENT_DID, CTX_CORPUS, SENSITIVITY_PUBLIC, true, false, 100, 200);
        let err = authorize_derivative_or_explanation_export(&quins, &grant_no_trace, SENSITIVITY_PUBLIC, true, 150).unwrap_err();
        assert_eq!(err, AccessGateError::ExplanationTraceForbidden);
    }

    #[test]
    fn test_derivative_export_forbidden_and_context_isolation() {
        let quins = [test_quin(1, CTX_CORPUS, SENSITIVITY_PUBLIC)];
        let no_export = PrincipalAndAgentGrant::delegated_agent(HUMAN_DID, AGENT_DID, CTX_CORPUS, SENSITIVITY_PUBLIC, false, true, 100, 200);
        assert_eq!(authorize_derivative_or_explanation_export(&quins, &no_export, SENSITIVITY_PUBLIC, false, 150).unwrap_err(), AccessGateError::DerivativeExportForbidden);

        let foreign_quins = [test_quin(1, CTX_OTHER, SENSITIVITY_RESTRICTED)];
        let grant_corpus = PrincipalAndAgentGrant::direct_human(HUMAN_DID, CTX_CORPUS, SENSITIVITY_RESTRICTED, 100, 200);
        let err_ctx = authorize_derivative_or_explanation_export(&foreign_quins, &grant_corpus, SENSITIVITY_RESTRICTED, false, 150).unwrap_err();
        assert_eq!(err_ctx, AccessGateError::ContextUnauthorized { requested_context: CTX_OTHER, authorized_context: CTX_CORPUS });
    }

    #[test]
    fn test_redaction_and_buffer_overflow_handling() {
        let quins = [test_quin(1, CTX_CORPUS, SENSITIVITY_PUBLIC), test_quin(2, CTX_CORPUS, SENSITIVITY_CLASSIFIED)];
        let grant = PrincipalAndAgentGrant::direct_human(HUMAN_DID, CTX_CORPUS, SENSITIVITY_RESTRICTED, 100, 200);

        let mut out = [NQuin::default(); 2];
        let rec = filter_authorized_cohort_quins(&quins, &grant, 150, &mut out).unwrap();
        assert_eq!((rec.admitted_count, rec.redacted_count, rec.inherited_max_sensitivity, out[0].subject), (1, 1, SENSITIVITY_PUBLIC, 1));

        let mut tiny_out = [];
        assert_eq!(filter_authorized_cohort_quins(&quins, &grant, 150, &mut tiny_out).unwrap_err(), AccessGateError::OutputBufferTooSmall { required: 1, available: 0 });
    }

    #[test]
    fn test_successful_derivative_export_auth_token() {
        let quins = [test_quin(1, CTX_CORPUS, SENSITIVITY_PUBLIC), test_quin(2, CTX_CORPUS, SENSITIVITY_RESTRICTED)];
        let grant = PrincipalAndAgentGrant::direct_human(HUMAN_DID, CTX_CORPUS, SENSITIVITY_RESTRICTED, 100, 200);

        let tok = authorize_derivative_or_explanation_export(&quins, &grant, SENSITIVITY_RESTRICTED, false, 150).unwrap();
        let trace_tok = authorize_derivative_or_explanation_export(&quins, &grant, SENSITIVITY_RESTRICTED, true, 150).unwrap();
        assert!(tok != 0 && trace_tok != 0 && tok != trace_tok);
    }
}
