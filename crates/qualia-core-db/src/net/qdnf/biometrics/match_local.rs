//! Device-local matching (E15.1, E15.2). Default path.
//!
//! QDNF records a platform authenticator decision bound to a cancellable
//! template. It does not expose template bytes to the network and does not
//! treat a match as an [`crate::net::qdnf::authority::ExecutionPermit`].

use crate::net::qdnf::authority::ExecutionPermit;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::capture::RawCapture;
use super::template::DerivedTemplate;
use super::BiometricObjectKind;

/// Local comparison outcome. Sensitive derived data, not an authority grant.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchDecision {
    NoMatch = 0,
    Match = 1,
}

/// Score/decision plus uncertainty. Synthetic fixture scores are not FAR/FRR.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchResult {
    decision: MatchDecision,
    score: u16,
    uncertainty: u16,
    quality: u16,
    spoof_indicator: u16,
    purpose: StrongDigest,
    expires_unix: u64,
}

impl MatchResult {
    #[inline]
    pub const fn kind(self) -> BiometricObjectKind {
        BiometricObjectKind::MatchResult
    }

    #[inline]
    pub const fn decision(self) -> MatchDecision {
        self.decision
    }

    #[inline]
    pub const fn score(self) -> u16 {
        self.score
    }

    #[inline]
    pub const fn uncertainty(self) -> u16 {
        self.uncertainty
    }

    #[inline]
    pub const fn quality(self) -> u16 {
        self.quality
    }

    #[inline]
    pub const fn spoof_indicator(self) -> u16 {
        self.spoof_indicator
    }

    #[inline]
    pub const fn purpose(self) -> StrongDigest {
        self.purpose
    }

    #[inline]
    pub const fn expires_unix(self) -> u64 {
        self.expires_unix
    }

    /// A match never grants session, routing, or identity privilege.
    #[inline]
    pub const fn grants_network_authority(self) -> bool {
        false
    }
}

/// Network authority is independent of biometric activation (NIST 800-63B).
#[inline]
pub const fn match_grants_network_authority() -> bool {
    false
}

/// MatchResult cannot produce an execution permit. Always Denied.
pub fn execution_permit_from_match(_result: &MatchResult) -> Result<ExecutionPermit, QdnfError> {
    Err(QdnfError::Denied)
}

/// Default matcher: device-local, purpose-bound, non-revoked reference.
///
/// `platform_match` is the authenticator's local decision. The returned
/// `score` is a protocol fixture (quality echo), not operational accuracy.
pub fn match_local(
    probe: &RawCapture,
    reference: &DerivedTemplate,
    platform_match: bool,
    now_unix: u64,
    expires_unix: u64,
) -> Result<MatchResult, QdnfError> {
    if reference.is_revoked() {
        return Err(QdnfError::Revoked);
    }
    if probe.modality() != reference.modality() {
        return Err(QdnfError::Conflict);
    }
    if now_unix >= expires_unix {
        return Err(QdnfError::Expired);
    }
    let decision = if platform_match {
        MatchDecision::Match
    } else {
        MatchDecision::NoMatch
    };
    Ok(MatchResult {
        decision,
        score: probe.quality(),
        uncertainty: probe.timestamp_uncertainty(),
        quality: probe.quality(),
        spoof_indicator: 0,
        purpose: reference.purpose(),
        expires_unix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::biometrics::capture::Modality;
    use crate::net::qdnf::biometrics::template::derive_template;
    use crate::net::qdnf::types::Generation;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x.0[47] = 0xC3;
        x
    }

    fn probe() -> RawCapture {
        RawCapture::acquire(Modality::Iris, d(1), d(2), 40, 2).unwrap()
    }

    fn reference() -> DerivedTemplate {
        derive_template(&probe(), d(10), d(11), d(12), Generation(4)).unwrap()
    }

    #[test]
    fn kinds_and_gates() {
        let r = match_local(&probe(), &reference(), true, 1, 100).unwrap();
        assert_eq!(r.kind(), BiometricObjectKind::MatchResult);
        assert_eq!(r.decision(), MatchDecision::Match);
        assert!(!match_grants_network_authority());
        assert!(!r.grants_network_authority());
    }

    #[test]
    fn successful_match_does_not_issue_execution_permit() {
        let r = match_local(&probe(), &reference(), true, 1, 100).unwrap();
        assert_eq!(
            execution_permit_from_match(&r).unwrap_err(),
            QdnfError::Denied
        );
    }

    #[test]
    fn no_match_is_still_not_authority() {
        let r = match_local(&probe(), &reference(), false, 1, 100).unwrap();
        assert_eq!(r.decision(), MatchDecision::NoMatch);
        assert_eq!(
            execution_permit_from_match(&r).unwrap_err(),
            QdnfError::Denied
        );
    }

    #[test]
    fn revoked_reference_cannot_match() {
        let mut t = reference();
        crate::net::qdnf::biometrics::template::revoke_template(&mut t).unwrap();
        assert_eq!(
            match_local(&probe(), &t, true, 1, 100).unwrap_err(),
            QdnfError::Revoked
        );
    }

    #[test]
    fn modality_mismatch_is_conflict() {
        let voice = RawCapture::acquire(Modality::Voice, d(1), d(2), 40, 2).unwrap();
        assert_eq!(
            match_local(&voice, &reference(), true, 1, 100).unwrap_err(),
            QdnfError::Conflict
        );
    }

    #[test]
    fn expired_window_is_expired() {
        assert_eq!(
            match_local(&probe(), &reference(), true, 100, 100).unwrap_err(),
            QdnfError::Expired
        );
    }
}
