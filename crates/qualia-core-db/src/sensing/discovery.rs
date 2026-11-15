//! Host supplied, bounded discovery for sensing endpoints.
//!
//! Discovery returns opaque endpoint references and relationship-scoped context
//! labels. It does not establish identity, guardianship, consent, permission,
//! calibration, connectivity, or sensor capability. A caller must separately
//! authorize use of any discovered endpoint.

use core::num::NonZeroU64;

/// Maximum discovery candidates accepted and returned in one call.
pub const MAX_DISCOVERY_RESULTS: usize = 32;

/// A nonzero opaque identifier in a provider-defined namespace.
///
/// These values are handles only. In particular, they are not assumed to be
/// DIDs, cryptographic identities, or globally comparable identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryId(NonZeroU64);

impl DiscoveryId {
    pub const fn new(value: u64) -> Result<Self, DiscoveryError> {
        match NonZeroU64::new(value) {
            Some(value) => Ok(Self(value)),
            None => Err(DiscoveryError::InvalidIdentifier),
        }
    }

    pub const fn value(self) -> u64 {
        self.0.get()
    }
}

/// Monotonic time in one explicit provider clock domain and restart epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryClock {
    clock_id: DiscoveryId,
    restart_epoch: u32,
    ticks_ns: u64,
    uncertainty_ns: u64,
}

impl DiscoveryClock {
    pub const fn new(
        clock_id: u64,
        restart_epoch: u32,
        ticks_ns: u64,
        uncertainty_ns: u64,
    ) -> Result<Self, DiscoveryError> {
        let clock_id = match DiscoveryId::new(clock_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        if ticks_ns.checked_add(uncertainty_ns).is_none() {
            return Err(DiscoveryError::InvalidTime);
        }
        Ok(Self {
            clock_id,
            restart_epoch,
            ticks_ns,
            uncertainty_ns,
        })
    }

    pub const fn clock_id(self) -> DiscoveryId {
        self.clock_id
    }

    pub const fn restart_epoch(self) -> u32 {
        self.restart_epoch
    }

    pub const fn ticks_ns(self) -> u64 {
        self.ticks_ns
    }

    pub const fn uncertainty_ns(self) -> u64 {
        self.uncertainty_ns
    }

    const fn lower_ns(self) -> u64 {
        self.ticks_ns.saturating_sub(self.uncertainty_ns)
    }

    const fn upper_ns(self) -> u64 {
        // Constructor has already checked this addition.
        self.ticks_ns + self.uncertainty_ns
    }
}

/// Provider-declared endpoint lease. Expiry and observation use the same
/// monotonic clock and restart epoch; they are not UTC timestamps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryLease {
    observed: DiscoveryClock,
    expires_at_ns: u64,
}

impl DiscoveryLease {
    pub const fn new(observed: DiscoveryClock, expires_at_ns: u64) -> Result<Self, DiscoveryError> {
        // The observation must be definitely before expiry, including its
        // entire declared uncertainty interval.
        if observed.upper_ns() >= expires_at_ns {
            return Err(DiscoveryError::InvalidTime);
        }
        Ok(Self {
            observed,
            expires_at_ns,
        })
    }

    pub const fn observed(self) -> DiscoveryClock {
        self.observed
    }

    pub const fn expires_at_ns(self) -> u64 {
        self.expires_at_ns
    }

    /// Conservative validity check: `now` must be definitely after the
    /// observation and its latest possible time must remain before expiry.
    pub fn is_fresh_at(self, now: DiscoveryClock) -> Result<bool, DiscoveryError> {
        if self.observed.clock_id != now.clock_id
            || self.observed.restart_epoch != now.restart_epoch
        {
            return Err(DiscoveryError::ClockDomainMismatch);
        }
        Ok(now.lower_ns() >= self.observed.upper_ns() && now.upper_ns() < self.expires_at_ns)
    }
}

/// Optional social or physical query filters. Relationship and context
/// identifiers are opaque labels supplied by the host; their presence is not
/// proof of a relationship or authorization. All filters may be absent for
/// environmental, nonhuman, or not-yet-classified observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryQuery {
    relationship_id: Option<DiscoveryId>,
    context_id: Option<DiscoveryId>,
    subject_id: Option<DiscoveryId>,
    sensor_id: Option<DiscoveryId>,
    protocol_id: Option<DiscoveryId>,
}

impl DiscoveryQuery {
    pub const fn new(
        relationship_id: Option<u64>,
        context_id: Option<u64>,
        subject_id: Option<u64>,
        sensor_id: Option<u64>,
        protocol_id: Option<u64>,
    ) -> Result<Self, DiscoveryError> {
        let relationship_id = match optional_id(relationship_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let context_id = match optional_id(context_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let subject_id = match optional_id(subject_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let sensor_id = match optional_id(sensor_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let protocol_id = match optional_id(protocol_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        Ok(Self {
            relationship_id,
            context_id,
            subject_id,
            sensor_id,
            protocol_id,
        })
    }

    pub const fn relationship_id(self) -> Option<DiscoveryId> {
        self.relationship_id
    }

    pub const fn context_id(self) -> Option<DiscoveryId> {
        self.context_id
    }

    pub const fn subject_id(self) -> Option<DiscoveryId> {
        self.subject_id
    }

    pub const fn sensor_id(self) -> Option<DiscoveryId> {
        self.sensor_id
    }

    pub const fn protocol_id(self) -> Option<DiscoveryId> {
        self.protocol_id
    }
}

/// Opaque host locator/reference plus the optional scope the host says it serves.
/// The value is intentionally not a network address and is meaningful only
/// to the provider that returned it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveredEndpoint {
    provider_id: DiscoveryId,
    endpoint_ref: DiscoveryId,
    relationship_id: Option<DiscoveryId>,
    context_id: Option<DiscoveryId>,
    subject_id: Option<DiscoveryId>,
    sensor_id: Option<DiscoveryId>,
    protocol_id: DiscoveryId,
    lease: DiscoveryLease,
}

impl DiscoveredEndpoint {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        provider_id: u64,
        endpoint_ref: u64,
        relationship_id: Option<u64>,
        context_id: Option<u64>,
        subject_id: Option<u64>,
        sensor_id: Option<u64>,
        protocol_id: u64,
        lease: DiscoveryLease,
    ) -> Result<Self, DiscoveryError> {
        let provider_id = match DiscoveryId::new(provider_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let endpoint_ref = match DiscoveryId::new(endpoint_ref) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let relationship_id = match optional_id(relationship_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let context_id = match optional_id(context_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let subject_id = match optional_id(subject_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let sensor_id = match optional_id(sensor_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        let protocol_id = match DiscoveryId::new(protocol_id) {
            Ok(id) => id,
            Err(error) => return Err(error),
        };
        Ok(Self {
            provider_id,
            endpoint_ref,
            relationship_id,
            context_id,
            subject_id,
            sensor_id,
            protocol_id,
            lease,
        })
    }

    pub const fn provider_id(self) -> DiscoveryId {
        self.provider_id
    }
    pub const fn endpoint_ref(self) -> DiscoveryId {
        self.endpoint_ref
    }
    pub const fn relationship_id(self) -> Option<DiscoveryId> {
        self.relationship_id
    }
    pub const fn context_id(self) -> Option<DiscoveryId> {
        self.context_id
    }
    pub const fn subject_id(self) -> Option<DiscoveryId> {
        self.subject_id
    }
    pub const fn sensor_id(self) -> Option<DiscoveryId> {
        self.sensor_id
    }
    pub const fn protocol_id(self) -> DiscoveryId {
        self.protocol_id
    }
    pub const fn lease(self) -> DiscoveryLease {
        self.lease
    }
}

/// Host-supplied resolver. Implementations may bridge QDNF, SPARQL-MM,
/// social directories, or another service; this contract itself performs no
/// network I/O and assigns no trust to provider output.
pub trait DiscoveryProvider {
    fn resolve_into(
        &self,
        query: DiscoveryQuery,
        out: &mut [Option<DiscoveredEndpoint>],
    ) -> Result<usize, DiscoveryError>;
}

/// Errors from bounded, caller-buffered resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryError {
    InvalidIdentifier,
    InvalidTime,
    ClockDomainMismatch,
    Capacity,
    InvalidProviderResult,
    DuplicateEndpoint,
    ProviderFailure,
}

/// Resolve a scoped query into caller-owned output.
///
/// A fixed stack scratch buffer validates the complete result set before the
/// caller's output is modified. The provider result is still an assertion and
/// does not authorize use of a sensor or establish the endpoint's identity.
/// On success, only `out[..count]` is replaced; the caller-owned tail remains
/// untouched and has no validity guarantee from this call.
pub fn discover_into<P: DiscoveryProvider>(
    provider: &P,
    query: DiscoveryQuery,
    now: DiscoveryClock,
    out: &mut [Option<DiscoveredEndpoint>],
) -> Result<usize, DiscoveryError> {
    if out.len() > MAX_DISCOVERY_RESULTS {
        return Err(DiscoveryError::Capacity);
    }
    let mut scratch = [None; MAX_DISCOVERY_RESULTS];
    let count = provider.resolve_into(query, &mut scratch)?;
    if count > scratch.len() || count > out.len() {
        return Err(DiscoveryError::Capacity);
    }
    let mut i = 0;
    while i < count {
        let endpoint = scratch[i].ok_or(DiscoveryError::InvalidProviderResult)?;
        if query
            .relationship_id
            .is_some_and(|id| Some(id) != endpoint.relationship_id)
            || query
                .context_id
                .is_some_and(|id| Some(id) != endpoint.context_id)
            || query
                .subject_id
                .is_some_and(|id| Some(id) != endpoint.subject_id)
            || query
                .sensor_id
                .is_some_and(|id| Some(id) != endpoint.sensor_id)
            || query
                .protocol_id
                .is_some_and(|id| id != endpoint.protocol_id)
            || !endpoint.lease.is_fresh_at(now)?
        {
            return Err(DiscoveryError::InvalidProviderResult);
        }
        let mut prior = 0;
        while prior < i {
            let earlier = scratch[prior].ok_or(DiscoveryError::InvalidProviderResult)?;
            if endpoint.provider_id == earlier.provider_id
                && endpoint.endpoint_ref == earlier.endpoint_ref
            {
                return Err(DiscoveryError::DuplicateEndpoint);
            }
            prior += 1;
        }
        i += 1;
    }
    i = 0;
    while i < count {
        out[i] = scratch[i];
        i += 1;
    }
    Ok(count)
}

const fn optional_id(value: Option<u64>) -> Result<Option<DiscoveryId>, DiscoveryError> {
    match value {
        Some(value) => match DiscoveryId::new(value) {
            Ok(id) => Ok(Some(id)),
            Err(error) => Err(error),
        },
        None => Ok(None),
    }
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
