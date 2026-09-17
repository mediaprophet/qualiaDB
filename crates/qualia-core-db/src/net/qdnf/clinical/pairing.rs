//! Private authenticated pairing with the known clinician and receiving endpoint.
//!
//! Public patient, VIP-family and diagnosis discovery are always Denied.

use super::{admit_outcome, first_empty, require_active_mutual, MAX_SLOTS};
use crate::net::qdnf::authority::{ContactState, PolicyOutcome, TemporalGrant};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

#[inline]
pub const fn public_patient_index_allowed() -> bool {
    false
}

#[inline]
pub const fn vip_association_indexed() -> bool {
    false
}

#[inline]
pub const fn diagnosis_discovery_allowed() -> bool {
    false
}

/// Directed known patient/clinician pair. Not a public index record.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClinicalPair {
    pub patient: StrongDigest,
    pub clinician: StrongDigest,
    pub patient_contact: ContactState,
    pub clinician_contact: ContactState,
    pub route: StrongDigest,
}

/// Eight-slot private pairing table.
#[derive(Clone, Copy, Debug)]
pub struct PairingTable {
    slots: [Option<ClinicalPair>; MAX_SLOTS],
}

impl PairingTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_SLOTS],
        }
    }

    fn find(&self, patient: StrongDigest, clinician: StrongDigest) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_SLOTS {
            if let Some(p) = self.slots[i] {
                if p.patient == patient && p.clinician == clinician {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    /// Duplicate is idempotent. A ninth distinct pair is Capacity.
    pub fn admit(&mut self, pair: ClinicalPair) -> Result<usize, QdnfError> {
        if pair.patient == StrongDigest::ZERO || pair.clinician == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        if pair.patient == pair.clinician {
            return Err(QdnfError::Conflict);
        }
        if let Some(i) = self.find(pair.patient, pair.clinician) {
            self.slots[i] = Some(pair);
            return Ok(i);
        }
        let i = first_empty(&self.slots).ok_or(QdnfError::Capacity)?;
        self.slots[i] = Some(pair);
        Ok(i)
    }

    pub fn get(
        &self,
        patient: StrongDigest,
        clinician: StrongDigest,
    ) -> Result<ClinicalPair, QdnfError> {
        let i = self
            .find(patient, clinician)
            .ok_or(QdnfError::Unauthorized)?;
        self.slots[i].ok_or(QdnfError::Unauthorized)
    }

    /// Update live contact states for an admitted pair.
    pub fn set_contacts(
        &mut self,
        patient: StrongDigest,
        clinician: StrongDigest,
        patient_contact: ContactState,
        clinician_contact: ContactState,
    ) -> Result<(), QdnfError> {
        let i = self
            .find(patient, clinician)
            .ok_or(QdnfError::Unauthorized)?;
        let mut pair = self.slots[i].ok_or(QdnfError::Unauthorized)?;
        pair.patient_contact = patient_contact;
        pair.clinician_contact = clinician_contact;
        self.slots[i] = Some(pair);
        Ok(())
    }

    /// Direct route exchange. Pair must be admitted. Inactive contacts cannot
    /// release a route. Grant currency is `grant.current_at(now)`, never a boolean.
    pub fn exchange_route(
        &mut self,
        patient: StrongDigest,
        clinician: StrongDigest,
        route: StrongDigest,
        outcome: PolicyOutcome,
        grant: &TemporalGrant,
        now_unix: u64,
    ) -> Result<StrongDigest, QdnfError> {
        if route == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        let i = self
            .find(patient, clinician)
            .ok_or(QdnfError::Unauthorized)?;
        let mut pair = self.slots[i].ok_or(QdnfError::Unauthorized)?;
        require_active_mutual(pair.patient_contact, pair.clinician_contact)?;
        grant.current_at(now_unix)?;
        if grant.audience_digest != StrongDigest::ZERO
            && grant.audience_digest != clinician
            && grant.audience_digest != patient
        {
            return Err(QdnfError::Denied);
        }
        admit_outcome(outcome)?;
        pair.route = route;
        self.slots[i] = Some(pair);
        Ok(route)
    }

    /// Public indexing of a known pair is always Denied.
    pub fn index_public(&self, _pair: ClinicalPair) -> Result<(), QdnfError> {
        Err(QdnfError::Denied)
    }
}

impl Default for PairingTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Relationship publication is always Denied (no public / VIP / edge index).
pub fn publish_relationship(
    _patient: StrongDigest,
    _clinician: StrongDigest,
) -> Result<(), QdnfError> {
    Err(QdnfError::Denied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::types::ProfileId;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x
    }

    fn grant_until(exp: u64) -> TemporalGrant {
        TemporalGrant {
            purpose_digest: d(0x11),
            audience_digest: d(2),
            authority_generation: 1,
            not_before_unix: 0,
            expires_unix: exp,
            profile: ProfileId::QDNF_CRYPTO_1,
        }
    }

    fn pair(patient: u8, clinician: u8, clinician_contact: ContactState) -> ClinicalPair {
        ClinicalPair {
            patient: d(patient),
            clinician: d(clinician),
            patient_contact: ContactState::Active,
            clinician_contact,
            route: StrongDigest::ZERO,
        }
    }

    #[test]
    fn private_pairing_rejects_public_vip_and_diagnosis_index() {
        let mut table = PairingTable::new();
        let p = pair(1, 2, ContactState::Active);
        assert!(table.admit(p).is_ok());
        assert_eq!(table.index_public(p), Err(QdnfError::Denied));
        assert_eq!(
            publish_relationship(p.patient, p.clinician),
            Err(QdnfError::Denied)
        );
        assert!(!public_patient_index_allowed());
        assert!(!vip_association_indexed());
        assert!(!diagnosis_discovery_allowed());
        let grant = grant_until(100);
        let route = d(9);
        assert_eq!(
            table.exchange_route(
                p.patient,
                p.clinician,
                route,
                PolicyOutcome::Allow,
                &grant,
                10
            ),
            Ok(route)
        );
        assert_eq!(table.get(p.patient, p.clinician).unwrap().route, route);
    }

    #[test]
    fn suspended_blocked_and_request_cannot_exchange() {
        let grant = grant_until(100);
        let route = d(9);
        let mut table = PairingTable::new();
        let p = pair(1, 2, ContactState::Active);
        table.admit(p).unwrap();
        table
            .set_contacts(
                p.patient,
                p.clinician,
                ContactState::Active,
                ContactState::Suspended,
            )
            .unwrap();
        assert_eq!(
            table.exchange_route(
                p.patient,
                p.clinician,
                route,
                PolicyOutcome::Allow,
                &grant,
                10
            ),
            Err(QdnfError::Denied)
        );
        table
            .set_contacts(
                p.patient,
                p.clinician,
                ContactState::Active,
                ContactState::Blocked,
            )
            .unwrap();
        assert_eq!(
            table.exchange_route(
                p.patient,
                p.clinician,
                route,
                PolicyOutcome::Allow,
                &grant,
                10
            ),
            Err(QdnfError::Denied)
        );
        table
            .set_contacts(
                p.patient,
                p.clinician,
                ContactState::Request,
                ContactState::Active,
            )
            .unwrap();
        assert_eq!(
            table.exchange_route(
                p.patient,
                p.clinician,
                route,
                PolicyOutcome::Allow,
                &grant,
                10
            ),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn unpaired_and_capacity() {
        let mut table = PairingTable::new();
        let grant = grant_until(100);
        assert_eq!(
            table.exchange_route(d(3), d(4), d(9), PolicyOutcome::Allow, &grant, 10),
            Err(QdnfError::Unauthorized)
        );
        let mut n = 1u8;
        while n <= 8 {
            assert!(table.admit(pair(n, n + 10, ContactState::Active)).is_ok());
            n += 1;
        }
        assert_eq!(
            table.admit(pair(20, 21, ContactState::Active)),
            Err(QdnfError::Capacity)
        );
    }
}
