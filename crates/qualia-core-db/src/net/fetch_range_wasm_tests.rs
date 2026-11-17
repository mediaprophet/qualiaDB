use super::*;
use crate::bundle::stream::http::{FallbackPolicy, LEGACY_FULL_OBJECT_FALLBACK_BYTES};
use crate::net::disclosure::NetworkDisclosureRegistry;

#[test]
fn empty_registry_allows_packages_and_production() {
    let reg = NetworkDisclosureRegistry::new();
    assert!(authorize_url(&reg, EWM_RANGE_ADAPTER, "/packages/qbdl/base.hmc").is_ok());
    assert!(authorize_url(
        &reg,
        EWM_RANGE_ADAPTER,
        "/production/lush-woodland/scene.json"
    )
    .is_ok());
}

#[test]
fn https_and_other_prefixes_fail_closed() {
    let reg = NetworkDisclosureRegistry::new();
    assert_eq!(
        authorize_url(&reg, EWM_RANGE_ADAPTER, "https://elevation.example/tile"),
        Err(FetchRangeError::Denied)
    );
    assert_eq!(
        authorize_url(&reg, EWM_RANGE_ADAPTER, "/data/ewm/first-paint.json"),
        Err(FetchRangeError::Denied)
    );
}

#[test]
fn traversal_and_protocol_relative_rejected() {
    let reg = NetworkDisclosureRegistry::new();
    assert!(!same_origin_static_bypass("/packages/../secret"));
    assert!(!same_origin_static_bypass("//evil.example/packages/x"));
    assert_eq!(
        authorize_url(&reg, EWM_RANGE_ADAPTER, "/packages/../secret"),
        Err(FetchRangeError::InvalidUrl)
    );
}

#[test]
fn ranged_200_over_one_mib_is_error() {
    let fallback = FallbackPolicy::explicit(LEGACY_FULL_OBJECT_FALLBACK_BYTES).unwrap();
    assert_eq!(
        admit_range_body(200, 2 * 1024 * 1024, 2 * 1024 * 1024, fallback),
        Err(FetchRangeError::OverBudget)
    );
}

#[test]
fn ranged_200_matching_length_is_not_synthetic_206() {
    assert_eq!(
        admit_range_body(200, 64, 64, FallbackPolicy::FORBIDDEN),
        Err(FetchRangeError::OverBudget)
    );
}

#[test]
fn ranged_206_exact_length_ok() {
    assert!(admit_range_body(206, 64, 64, FallbackPolicy::FORBIDDEN).is_ok());
}

#[test]
fn ranged_206_wrong_length_fails() {
    assert_eq!(
        admit_range_body(206, 64, 32, FallbackPolicy::FORBIDDEN),
        Err(FetchRangeError::Length)
    );
}

#[test]
fn display_is_human_readable() {
    assert_eq!(
        FetchRangeError::Denied.to_string(),
        "network disclosure denied this URL"
    );
    assert_eq!(
        FetchRangeError::InvalidUrl.to_string(),
        "URL is outside same-origin static prefixes"
    );
    assert_eq!(
        FetchRangeError::Status(416).to_string(),
        "HTTP 416 from range fetch"
    );
    assert_eq!(
        FetchRangeError::Length.to_string(),
        "range body length does not match the requested window"
    );
    assert_eq!(
        FetchRangeError::OverBudget.to_string(),
        "range fallback exceeded 1 MiB or the fetch budget"
    );
    assert_eq!(
        FetchRangeError::Transport.to_string(),
        "browser fetch failed"
    );
}

#[test]
fn live_gis_needs_consent() {
    let mut reg = NetworkDisclosureRegistry::new();
    assert!(authorize_url(&reg, "dem_adapter", "https://elevation.example/a").is_err());
    reg.register_egress(
        "dem_adapter",
        "https://elevation.example",
        "DEM",
        "opt-in ingest",
    );
    assert!(authorize_url(&reg, "dem_adapter", "https://elevation.example/a").is_ok());
}