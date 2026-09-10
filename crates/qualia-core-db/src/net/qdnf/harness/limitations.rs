//! Honest unsupported / unqualified claims (E21.5). Test counts are not certification.

/// Physical two-host Ethernet is not qualified in this environment.
pub const PHYSICAL_TWO_HOST: &str = "physical_two_host_qualified=false";
/// Observational snapshot of the Native Independent default-daemon claim.
/// Another agent may change crate default features; this string is not a live
/// measurement of `native_independent_daemon_proven()`.
pub const NATIVE_INDEPENDENT_DAEMON: &str = "native_independent_daemon_proven=true (LIG isolated)";
/// Operational FAR/FRR is not measured; remote matching stays unselectable.
pub const OPERATIONAL_BIOMETRIC: &str = "remote_matching_selectable=false";
/// Live settlement rails are Unsupported.
pub const LIVE_PAYMENT_RAIL: &str = "live_payment_rail=Unsupported";
/// Independent cryptographic composition review is not claimed.
pub const INDEPENDENT_CRYPTO_REVIEW: &str = "e02.6/e21.3 independent review not executed";
/// Bounded in-tree fuzz / model-check / seconds-churn exist; libFuzzer, TLA+,
/// days-long churn, and commissioned independent review do not.
pub const FUZZ_AND_CHURN: &str = "e21.2 bounded in-tree codec fuzz + ownership model-check (not libFuzzer, not TLA+); e21.4 bounded-seconds churn (not days); e21.3 review packet prepared, independent review not executed";
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
        assert!(PHYSICAL_TWO_HOST.contains("false"));
        assert!(LIVE_PAYMENT_RAIL.contains("Unsupported"));
        assert!(OPERATIONAL_BIOMETRIC.contains("false"));
        assert!(MSVC_RUNNER.contains("None"));
        assert!(INDEPENDENT_CRYPTO_REVIEW.contains("not executed"));
        assert!(FUZZ_AND_CHURN.contains("bounded"));
        assert!(FUZZ_AND_CHURN.contains("not libFuzzer"));
        assert!(FUZZ_AND_CHURN.contains("not TLA+"));
        assert!(FUZZ_AND_CHURN.contains("not days"));
        assert!(FUZZ_AND_CHURN.contains("independent review not executed"));
        // NATIVE_INDEPENDENT_DAEMON is observational of another crate's
        // default features; do not hard-assert `!native_independent_daemon_proven()`.
        let _ = NATIVE_INDEPENDENT_DAEMON;
    }
}
