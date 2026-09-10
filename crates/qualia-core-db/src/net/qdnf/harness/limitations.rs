//! Honest unsupported / unqualified claims (E21.5). Test counts are not certification.

/// Physical two-host Ethernet is not qualified in this environment.
pub const PHYSICAL_TWO_HOST: &str = "physical_two_host_qualified=false";
/// Native Independent default-daemon proof is not claimed.
pub const NATIVE_INDEPENDENT_DAEMON: &str = "native_independent_daemon_proven=false";
/// Operational FAR/FRR is not measured; remote matching stays unselectable.
pub const OPERATIONAL_BIOMETRIC: &str = "remote_matching_selectable=false";
/// Live settlement rails are Unsupported.
pub const LIVE_PAYMENT_RAIL: &str = "live_payment_rail=Unsupported";
/// Independent cryptographic composition review is not claimed.
pub const INDEPENDENT_CRYPTO_REVIEW: &str = "e02.6/e21.3 independent review not executed";
/// Long-duration churn / libFuzzer / model-check are not claimed.
pub const FUZZ_AND_CHURN: &str = "e21.2/e21.4 not executed";
/// MSVC runner is not present on this Linux host.
pub const MSVC_RUNNER: &str = "MSVC_RUNNER=None";

/// Count of documented known limitations. Not a completeness score.
pub const KNOWN_LIMITATION_COUNT: usize = 7;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::bearer::raw_ethernet::physical_two_host_qualified;
    use crate::net::qdnf::biometrics::remote_matching_selectable;
    use crate::net::qdnf::economics::live_payment_rail;
    use crate::net::qdnf::errors::QdnfError;
    use crate::net::qdnf::harness::qualification::MSVC_RUNNER as Q_MSVC;
    use crate::net::qdnf::harness::scenarios::qualified_count;

    #[test]
    fn limitations_are_not_inferred_from_fixtures() {
        assert!(!physical_two_host_qualified());
        assert!(!remote_matching_selectable());
        assert_eq!(live_payment_rail(), Err(QdnfError::Unsupported));
        assert!(Q_MSVC.is_none());
        assert_eq!(qualified_count(), 0);
        assert_eq!(KNOWN_LIMITATION_COUNT, 7);
        let _ = (
            PHYSICAL_TWO_HOST,
            NATIVE_INDEPENDENT_DAEMON,
            OPERATIONAL_BIOMETRIC,
            LIVE_PAYMENT_RAIL,
            INDEPENDENT_CRYPTO_REVIEW,
            FUZZ_AND_CHURN,
            MSVC_RUNNER,
        );
    }
}
