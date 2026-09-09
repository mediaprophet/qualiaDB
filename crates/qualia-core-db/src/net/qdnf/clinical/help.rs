//! Confidential help independent of an abusive controller.
//!
//! Age/capacity/mandate is an explicit policy enum. A guardian is never
//! notified automatically.

use crate::net::qdnf::authority::ContactState;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Selected participation / capacity / mandate policy. Never a boolean
/// “notify parent”.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapacityMandatePolicy {
    AdultSelf = 1,
    ChildAssisted = 2,
    IndependentConfidentialHelp = 3,
    DisputedGuardian = 4,
}

/// Automatic guardian notification is never enabled.
#[inline]
pub const fn notify_guardian_automatically() -> bool {
    false
}

/// Auto-notify is always Denied. Independent confidential help does not
/// disclose to a restricted or disputed guardian.
pub fn notify_guardian(
    policy: CapacityMandatePolicy,
    guardian: StrongDigest,
    controller: ContactState,
) -> Result<(), QdnfError> {
    let _ = (policy, guardian, controller);
    debug_assert!(!notify_guardian_automatically());
    Err(QdnfError::Denied)
}

/// Admit confidential help independently of the controller contact state.
pub fn admit_confidential_help(
    helper: StrongDigest,
    policy: CapacityMandatePolicy,
    controller: ContactState,
) -> Result<(), QdnfError> {
    if helper == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    let _ = controller;
    match policy {
        CapacityMandatePolicy::AdultSelf
        | CapacityMandatePolicy::ChildAssisted
        | CapacityMandatePolicy::IndependentConfidentialHelp
        | CapacityMandatePolicy::DisputedGuardian => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x
    }

    #[test]
    fn guardian_is_not_auto_notified() {
        assert!(!notify_guardian_automatically());
        assert_eq!(
            notify_guardian(
                CapacityMandatePolicy::IndependentConfidentialHelp,
                d(3),
                ContactState::Active,
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            notify_guardian(
                CapacityMandatePolicy::DisputedGuardian,
                d(3),
                ContactState::Blocked,
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            notify_guardian(
                CapacityMandatePolicy::ChildAssisted,
                d(3),
                ContactState::Active
            ),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn confidential_help_ignores_abusive_controller() {
        assert!(admit_confidential_help(
            d(8),
            CapacityMandatePolicy::IndependentConfidentialHelp,
            ContactState::Blocked,
        )
        .is_ok());
        assert!(admit_confidential_help(
            d(8),
            CapacityMandatePolicy::DisputedGuardian,
            ContactState::Suspended,
        )
        .is_ok());
        assert_eq!(
            admit_confidential_help(
                StrongDigest::ZERO,
                CapacityMandatePolicy::AdultSelf,
                ContactState::Active,
            ),
            Err(QdnfError::Malformed)
        );
    }
}
