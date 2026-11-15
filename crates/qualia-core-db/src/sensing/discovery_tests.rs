use super::*;

fn id(value: u64) -> DiscoveryId {
    DiscoveryId::new(value).unwrap()
}

fn stamp(ticks_ns: u64, uncertainty_ns: u64) -> DiscoveryClock {
    DiscoveryClock::new(7, 2, ticks_ns, uncertainty_ns).unwrap()
}

fn endpoint(
    relationship_id: Option<u64>,
    context_id: Option<u64>,
    subject_id: Option<u64>,
    sensor_id: Option<u64>,
) -> DiscoveredEndpoint {
    DiscoveredEndpoint::new(
        10,
        11,
        relationship_id,
        context_id,
        subject_id,
        sensor_id,
        14,
        DiscoveryLease::new(stamp(100, 2), 200).unwrap(),
    )
    .unwrap()
}

fn query(
    relationship_id: Option<u64>,
    context_id: Option<u64>,
    subject_id: Option<u64>,
    sensor_id: Option<u64>,
) -> DiscoveryQuery {
    DiscoveryQuery::new(relationship_id, context_id, subject_id, sensor_id, Some(14)).unwrap()
}

struct FixedProvider {
    result: Option<DiscoveredEndpoint>,
    reported_count: usize,
}

impl DiscoveryProvider for FixedProvider {
    fn resolve_into(
        &self,
        _query: DiscoveryQuery,
        out: &mut [Option<DiscoveredEndpoint>],
    ) -> Result<usize, DiscoveryError> {
        if !out.is_empty() {
            out[0] = self.result;
        }
        Ok(self.reported_count)
    }
}

struct BatchProvider {
    endpoints: [Option<DiscoveredEndpoint>; 2],
}

impl DiscoveryProvider for BatchProvider {
    fn resolve_into(
        &self,
        _query: DiscoveryQuery,
        out: &mut [Option<DiscoveredEndpoint>],
    ) -> Result<usize, DiscoveryError> {
        out[..self.endpoints.len()].copy_from_slice(&self.endpoints);
        Ok(self.endpoints.len())
    }
}

#[test]
fn environmental_discovery_allows_unknown_subject_sensor_and_social_scope() {
    let provider = FixedProvider {
        result: Some(endpoint(None, None, None, None)),
        reported_count: 1,
    };
    let sentinel = Some(endpoint(None, None, None, None));
    let mut out = [None, sentinel];
    let count = discover_into(
        &provider,
        query(None, None, None, None),
        stamp(110, 1),
        &mut out,
    )
    .unwrap();
    assert_eq!(count, 1);
    let found = out[0].unwrap();
    assert_eq!(found.subject_id(), None);
    assert_eq!(found.sensor_id(), None);
    assert_eq!(found.relationship_id(), None);
    assert_eq!(found.context_id(), None);
    assert_eq!(found.provider_id(), id(10));
    assert_eq!(
        out[1], sentinel,
        "output after the returned prefix is retained"
    );
}

#[test]
fn query_filters_are_enforced_without_claiming_authority() {
    let provider = FixedProvider {
        result: Some(endpoint(Some(20), Some(21), Some(22), Some(23))),
        reported_count: 1,
    };
    let scoped = query(Some(20), Some(21), Some(22), Some(23));
    let mut out = [None; 1];
    assert_eq!(
        discover_into(&provider, scoped, stamp(110, 1), &mut out),
        Ok(1)
    );
    // The output contains locator metadata only. It has no permission or identity verdict.
    assert_eq!(out[0].unwrap().endpoint_ref(), id(11));

    let mismatched = query(Some(99), Some(21), Some(22), Some(23));
    let sentinel = Some(endpoint(None, None, None, None));
    out[0] = sentinel;
    assert_eq!(
        discover_into(&provider, mismatched, stamp(110, 1), &mut out),
        Err(DiscoveryError::InvalidProviderResult)
    );
    assert_eq!(
        out[0], sentinel,
        "invalid results do not partially overwrite output"
    );
}

#[test]
fn lease_requires_same_clock_epoch_and_conservative_freshness() {
    let lease = DiscoveryLease::new(stamp(100, 2), 200).unwrap();
    assert_eq!(lease.is_fresh_at(stamp(110, 1)), Ok(true));
    assert_eq!(lease.is_fresh_at(stamp(101, 1)), Ok(false));
    assert_eq!(lease.is_fresh_at(stamp(199, 2)), Ok(false));
    assert_eq!(
        lease.is_fresh_at(DiscoveryClock::new(8, 2, 110, 0).unwrap()),
        Err(DiscoveryError::ClockDomainMismatch)
    );
    assert_eq!(
        lease.is_fresh_at(DiscoveryClock::new(7, 3, 110, 0).unwrap()),
        Err(DiscoveryError::ClockDomainMismatch)
    );
}

#[test]
fn invalid_zero_ids_and_time_overflow_are_rejected() {
    assert_eq!(DiscoveryId::new(0), Err(DiscoveryError::InvalidIdentifier));
    assert_eq!(
        DiscoveryQuery::new(Some(0), None, None, None, None),
        Err(DiscoveryError::InvalidIdentifier)
    );
    assert_eq!(
        DiscoveryClock::new(1, 0, u64::MAX, 1),
        Err(DiscoveryError::InvalidTime)
    );
    assert_eq!(
        DiscoveryLease::new(stamp(100, 2), 100),
        Err(DiscoveryError::InvalidTime)
    );
}

#[test]
fn capacity_and_provider_count_errors_leave_output_unchanged() {
    let sentinel = Some(endpoint(None, None, None, None));
    let mut out = [sentinel];
    let query = query(None, None, None, None);
    let oversized = FixedProvider {
        result: sentinel,
        reported_count: 2,
    };
    assert_eq!(
        discover_into(&oversized, query, stamp(110, 1), &mut out),
        Err(DiscoveryError::Capacity)
    );
    assert_eq!(out[0], sentinel);

    let oversized_output = [None; MAX_DISCOVERY_RESULTS + 1];
    let provider = FixedProvider {
        result: None,
        reported_count: 0,
    };
    let mut oversized_output = oversized_output;
    assert_eq!(
        discover_into(&provider, query, stamp(110, 1), &mut oversized_output),
        Err(DiscoveryError::Capacity)
    );
}

#[test]
fn duplicate_provider_endpoint_pairs_are_rejected_before_output_write() {
    let duplicate = endpoint(None, None, None, None);
    let provider = BatchProvider {
        endpoints: [Some(duplicate), Some(duplicate)],
    };
    let sentinel = Some(endpoint(None, None, Some(44), None));
    let mut out = [sentinel; 2];
    assert_eq!(
        discover_into(
            &provider,
            query(None, None, None, None),
            stamp(110, 1),
            &mut out
        ),
        Err(DiscoveryError::DuplicateEndpoint)
    );
    assert_eq!(out, [sentinel; 2]);
}

#[test]
fn lease_rejects_observation_uncertainty_reaching_expiry() {
    assert_eq!(
        DiscoveryLease::new(stamp(100, 2), 102),
        Err(DiscoveryError::InvalidTime)
    );
    assert_eq!(
        DiscoveryLease::new(stamp(100, 2), 101),
        Err(DiscoveryError::InvalidTime)
    );
}
