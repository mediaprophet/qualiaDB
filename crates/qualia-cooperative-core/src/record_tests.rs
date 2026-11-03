use super::*;

#[test]
fn t71_instant_bridge_basic() {
    let i = InstantBridge::unix(1_700_000_000, 500_000_000);
    assert_eq!(i.secs, 1_700_000_000);
    assert_eq!(i.nanos, 500_000_000);
}

#[test]
fn t71_instant_bridge_from_coarse() {
    let i = InstantBridge::from_coarse(1_700_000_000);
    assert_eq!(i.secs, 1_700_000_000);
    assert_eq!(i.nanos, 0);
}

#[test]
fn t71_instant_bridge_to_unix_nanos() {
    let i = InstantBridge::unix(1, 500_000_000);
    assert_eq!(i.to_unix_nanos(), 1_500_000_000);
}

#[test]
fn t71_envelope_asserted_instant() {
    let env = RecordEnvelope {
        id: "test".into(),
        owner_did: "did:alice".into(),
        author_did: "did:alice".into(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Public,
        asserted_time_unix: 1_700_000_000,
        asserted_instant: None,
        valid_time_start_unix: None,
        valid_time_start_instant: None,
        valid_time_end_unix: None,
        valid_time_end_instant: None,
        predecessor_id: None,
        blob_hash: None,
        tombstone: false,
    };
    let instant = env.asserted_instant();
    assert_eq!(instant.secs, 1_700_000_000);
    assert_eq!(instant.nanos, 0);
}

#[test]
fn t71_envelope_with_asserted_instant() {
    let instant = InstantBridge::unix(1_700_000_000, 123_456);
    let env = RecordEnvelope::with_asserted_instant(
        "test",
        "did:alice",
        "did:alice",
        EpistemicStatus::Asserted,
        EvidenceType::SelfReported,
        SensitivityClass::Public,
        &instant,
        None,
    );
    assert_eq!(env.asserted_time_unix, 1_700_000_000);
    let recovered = env.asserted_instant();
    assert_eq!(recovered.secs, 1_700_000_000);
}

#[test]
fn t71_valid_time_instant_bridges() {
    let env = RecordEnvelope {
        id: "test".into(),
        owner_did: "did:alice".into(),
        author_did: "did:alice".into(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Public,
        asserted_time_unix: 1_700_000_000,
        asserted_instant: None,
        valid_time_start_unix: Some(1_699_000_000),
        valid_time_start_instant: None,
        valid_time_end_unix: Some(1_701_000_000),
        valid_time_end_instant: None,
        predecessor_id: None,
        blob_hash: None,
        tombstone: false,
    };
    let start = env.valid_time_start_instant().unwrap();
    assert_eq!(start.secs, 1_699_000_000);
    let end = env.valid_time_end_instant().unwrap();
    assert_eq!(end.secs, 1_701_000_000);
}

#[test]
fn t71_instant_comparison() {
    let a = InstantBridge::unix(1_700_000_000, 0);
    let b = InstantBridge::unix(1_700_000_001, 0);
    assert!(a.is_before(&b));
    assert!(b.is_after(&a));
    assert!(!a.is_after(&b));
    assert!(!b.is_before(&a));
    assert_eq!(a.min(b), a);
    assert_eq!(a.max(b), b);
}

#[test]
fn t71_instant_subsecond_comparison() {
    let a = InstantBridge::unix(1_700_000_000, 500_000_000);
    let b = InstantBridge::unix(1_700_000_000, 600_000_000);
    assert!(a.is_before(&b));
    assert!(b.is_after(&a));
}

#[test]
fn t71_instant_ord() {
    let a = InstantBridge::unix(1, 0);
    let b = InstantBridge::unix(2, 0);
    let c = InstantBridge::unix(1, 500_000_000);
    let mut v = vec![b, c, a];
    v.sort();
    assert_eq!(v, vec![a, c, b]);
}

#[test]
fn t71_duration_since() {
    let a = InstantBridge::unix(1_700_000_000, 0);
    let b = InstantBridge::unix(1_700_000_002, 500_000_000);
    let d = b.duration_since(&a);
    assert_eq!(d.secs, 2);
    assert_eq!(d.nanos, 500_000_000);
    assert_eq!(d.total_nanos(), 2_500_000_000);
}

#[test]
fn t71_duration_since_negative() {
    let a = InstantBridge::unix(1_700_000_002, 0);
    let b = InstantBridge::unix(1_700_000_000, 0);
    let d = b.duration_since(&a);
    assert!(d.total_nanos() < 0);
    assert!(!d.is_positive());
}

#[test]
fn t71_add_duration() {
    let a = InstantBridge::unix(1_700_000_000, 500_000_000);
    let d = DurationBridge::from_secs(2);
    let b = a.add_duration(&d);
    assert_eq!(b.secs, 1_700_000_002);
    assert_eq!(b.nanos, 500_000_000);
}

#[test]
fn t71_sub_duration() {
    let a = InstantBridge::unix(1_700_000_002, 0);
    let d = DurationBridge::from_secs(2);
    let b = a.sub_duration(&d);
    assert_eq!(b.secs, 1_700_000_000);
}

#[test]
fn t71_duration_from_nanos() {
    let d = DurationBridge::from_nanos(1_500_000_000);
    assert_eq!(d.secs, 1);
    assert_eq!(d.nanos, 500_000_000);
}

#[test]
fn t71_duration_total_millis() {
    let d = DurationBridge::from_nanos(1_500_000_000);
    assert_eq!(d.total_millis(), 1500);
}

#[test]
fn t71_duration_abs() {
    let d = DurationBridge::from_nanos(-1_500_000_000);
    assert!(d.total_nanos() < 0);
    let a = d.abs();
    assert_eq!(a.total_nanos(), 1_500_000_000);
}

#[test]
fn t71_duration_is_zero() {
    assert!(DurationBridge::from_nanos(0).is_zero());
    assert!(!DurationBridge::from_nanos(1).is_zero());
}

#[test]
fn t71_envelope_with_instants() {
    let asserted = InstantBridge::unix(1_700_000_000, 123_456);
    let start = InstantBridge::unix(1_699_000_000, 0);
    let end = InstantBridge::unix(1_701_000_000, 0);
    let env = RecordEnvelope::with_instants(
        "test",
        "did:alice",
        "did:alice",
        EpistemicStatus::Asserted,
        EvidenceType::SelfReported,
        SensitivityClass::Public,
        &asserted,
        Some(&start),
        Some(&end),
        None,
    );
    assert_eq!(env.asserted_time_unix, 1_700_000_000);
    assert_eq!(env.valid_time_start_unix, Some(1_699_000_000));
    assert_eq!(env.valid_time_end_unix, Some(1_701_000_000));
}

#[test]
fn t71_envelope_valid_time_duration() {
    let env = RecordEnvelope {
        id: "test".into(),
        owner_did: "did:alice".into(),
        author_did: "did:alice".into(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Public,
        asserted_time_unix: 1_700_000_000,
        asserted_instant: None,
        valid_time_start_unix: Some(1_699_000_000),
        valid_time_start_instant: None,
        valid_time_end_unix: Some(1_701_000_000),
        valid_time_end_instant: None,
        predecessor_id: None,
        blob_hash: None,
        tombstone: false,
    };
    let d = env.valid_time_duration().unwrap();
    assert_eq!(d.secs, 2_000_000);
    assert!(d.is_positive());
}

#[test]
fn t71_envelope_valid_time_duration_none() {
    let env = RecordEnvelope {
        id: "test".into(),
        owner_did: "did:alice".into(),
        author_did: "did:alice".into(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Public,
        asserted_time_unix: 1_700_000_000,
        asserted_instant: None,
        valid_time_start_unix: None,
        valid_time_start_instant: None,
        valid_time_end_unix: None,
        valid_time_end_instant: None,
        predecessor_id: None,
        blob_hash: None,
        tombstone: false,
    };
    assert!(env.valid_time_duration().is_none());
}
