//! Standing care permits. Release requires active mutual contact and a current grant.

use super::{admit_outcome, first_empty, require_active_mutual, MAX_SLOTS};
use crate::net::qdnf::authority::{ContactState, PolicyOutcome, TemporalGrant};
use crate::net::qdnf::contracts::{recheck_permit, BoundGenerations, LiveGenerations};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Routine transfer/reply between a pair, bound to purpose and record scope.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandingPermit {
    pub patient: StrongDigest,
    pub clinician: StrongDigest,
    pub purpose: StrongDigest,
    pub record_scope: StrongDigest,
    pub grant: TemporalGrant,
}

/// Eight-slot standing-permit table with independent revocation flags.
#[derive(Clone, Copy, Debug)]
pub struct PermitTable {
    slots: [Option<StandingPermit>; MAX_SLOTS],
    revoked: [bool; MAX_SLOTS],
    live: LiveGenerations,
}

impl PermitTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_SLOTS],
            revoked: [false; MAX_SLOTS],
            live: LiveGenerations {
                source: Generation(1),
                policy: Generation(1),
                identity: Generation(1),
            },
        }
    }

    fn find(
        &self,
        patient: StrongDigest,
        clinician: StrongDigest,
        purpose: StrongDigest,
        record_scope: StrongDigest,
    ) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_SLOTS {
            if let Some(p) = self.slots[i] {
                if p.patient == patient
                    && p.clinician == clinician
                    && p.purpose == purpose
                    && p.record_scope == record_scope
                {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    /// Duplicate (pair, purpose, scope) is idempotent.
    pub fn add(&mut self, permit: StandingPermit) -> Result<usize, QdnfError> {
        if permit.patient == StrongDigest::ZERO
            || permit.clinician == StrongDigest::ZERO
            || permit.purpose == StrongDigest::ZERO
            || permit.record_scope == StrongDigest::ZERO
        {
            return Err(QdnfError::Malformed);
        }
        if let Some(i) = self.find(
            permit.patient,
            permit.clinician,
            permit.purpose,
            permit.record_scope,
        ) {
            self.slots[i] = Some(permit);
            self.revoked[i] = false;
            return Ok(i);
        }
        let idx = first_empty(&self.slots).ok_or(QdnfError::Capacity)?;
        self.slots[idx] = Some(permit);
        self.revoked[idx] = false;
        Ok(idx)
    }

    /// Live generations at commit/release. A policy bump stale-fails authorize.
    pub fn set_live(&mut self, live: LiveGenerations) {
        self.live = live;
    }

    pub fn revoke(
        &mut self,
        patient: StrongDigest,
        clinician: StrongDigest,
        purpose: StrongDigest,
        record_scope: StrongDigest,
    ) -> Result<(), QdnfError> {
        let i = self
            .find(patient, clinician, purpose, record_scope)
            .ok_or(QdnfError::Unauthorized)?;
        self.revoked[i] = true;
        Ok(())
    }

    /// Extra recipients and other scopes are Unauthorized until a matching permit
    /// is added. Live Active mutual contact and `grant.current_at(now)` required.
    pub fn authorize(
        &self,
        patient: StrongDigest,
        clinician: StrongDigest,
        purpose: StrongDigest,
        record_scope: StrongDigest,
        recipient: StrongDigest,
        patient_contact: ContactState,
        clinician_contact: ContactState,
        outcome: PolicyOutcome,
        now_unix: u64,
    ) -> Result<(), QdnfError> {
        if recipient != clinician && recipient != patient {
            return Err(QdnfError::Unauthorized);
        }
        let i = self
            .find(patient, clinician, purpose, record_scope)
            .ok_or(QdnfError::Unauthorized)?;
        if self.revoked[i] {
            return Err(QdnfError::Denied);
        }
        let p = self.slots[i].ok_or(QdnfError::Unauthorized)?;
        let bound = BoundGenerations::new(
            Generation(p.grant.authority_generation),
            Generation(p.grant.authority_generation),
            Generation(p.grant.authority_generation),
        );
        recheck_permit(bound, self.live)?;
        require_active_mutual(patient_contact, clinician_contact)?;
        p.grant.current_at(now_unix)?;
        if p.grant.audience_digest != StrongDigest::ZERO
            && p.grant.audience_digest != recipient
            && p.grant.audience_digest != patient
            && p.grant.audience_digest != clinician
        {
            return Err(QdnfError::Denied);
        }
        admit_outcome(outcome)
    }
}

impl Default for PermitTable {
    fn default() -> Self {
        Self::new()
    }
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

    fn grant_until(audience: StrongDigest, exp: u64) -> TemporalGrant {
        TemporalGrant {
            purpose_digest: d(0x11),
            audience_digest: audience,
            authority_generation: 1,
            not_before_unix: 0,
            expires_unix: exp,
            profile: ProfileId::QDNF_CRYPTO_1,
        }
    }

    fn auth(
        t: &PermitTable,
        p: &StandingPermit,
        scope: StrongDigest,
        recip: StrongDigest,
        pc: ContactState,
        cc: ContactState,
        now: u64,
    ) -> Result<(), QdnfError> {
        t.authorize(
            p.patient,
            p.clinician,
            p.purpose,
            scope,
            recip,
            pc,
            cc,
            PolicyOutcome::Allow,
            now,
        )
    }

    #[test]
    fn active_current_grant_authorizes_pair() {
        let mut t = PermitTable::new();
        let clinician = d(2);
        let p = StandingPermit {
            patient: d(1),
            clinician,
            purpose: d(3),
            record_scope: d(4),
            grant: grant_until(clinician, 100),
        };
        assert!(t.add(p).is_ok());
        assert!(auth(
            &t,
            &p,
            p.record_scope,
            p.clinician,
            ContactState::Active,
            ContactState::Active,
            10
        )
        .is_ok());
        assert!(auth(
            &t,
            &p,
            p.record_scope,
            p.patient,
            ContactState::Active,
            ContactState::Active,
            10
        )
        .is_ok());
        assert_eq!(
            auth(
                &t,
                &p,
                p.record_scope,
                d(9),
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn suspended_blocked_revoked_wrong_audience_cannot_release() {
        let mut t = PermitTable::new();
        let clinician = d(2);
        let p = StandingPermit {
            patient: d(1),
            clinician,
            purpose: d(3),
            record_scope: d(4),
            grant: grant_until(clinician, 100),
        };
        t.add(p).unwrap();
        assert_eq!(
            auth(
                &t,
                &p,
                p.record_scope,
                p.clinician,
                ContactState::Active,
                ContactState::Suspended,
                10
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            auth(
                &t,
                &p,
                p.record_scope,
                p.clinician,
                ContactState::Blocked,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            auth(
                &t,
                &p,
                p.record_scope,
                p.clinician,
                ContactState::Request,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Unauthorized)
        );
        t.revoke(p.patient, p.clinician, p.purpose, p.record_scope)
            .unwrap();
        assert_eq!(
            auth(
                &t,
                &p,
                p.record_scope,
                p.clinician,
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Denied)
        );
        let mut t2 = PermitTable::new();
        let wrong = StandingPermit {
            grant: grant_until(d(9), 100),
            ..p
        };
        t2.add(wrong).unwrap();
        assert_eq!(
            auth(
                &t2,
                &wrong,
                wrong.record_scope,
                wrong.clinician,
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn expired_grant_is_expired() {
        let mut t = PermitTable::new();
        let clinician = d(2);
        let p = StandingPermit {
            patient: d(1),
            clinician,
            purpose: d(3),
            record_scope: d(7),
            grant: grant_until(clinician, 5),
        };
        t.add(p).unwrap();
        assert_eq!(
            auth(
                &t,
                &p,
                p.record_scope,
                p.clinician,
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Expired)
        );
    }

    #[test]
    fn live_policy_generation_advance_is_stale() {
        let mut t = PermitTable::new();
        let clinician = d(2);
        let p = StandingPermit {
            patient: d(1),
            clinician,
            purpose: d(3),
            record_scope: d(4),
            grant: grant_until(clinician, 100),
        };
        t.add(p).unwrap();
        t.set_live(LiveGenerations {
            source: Generation(1),
            policy: Generation(2),
            identity: Generation(1),
        });
        assert_eq!(
            auth(
                &t,
                &p,
                p.record_scope,
                p.clinician,
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::StaleGeneration)
        );
    }
}
